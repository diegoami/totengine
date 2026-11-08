#!/usr/bin/env python3
"""Read-only, bounded ToT corpus inspection; never executes or admits gameplay."""
from __future__ import annotations

import argparse
from collections import Counter
import hashlib
from io import StringIO
import json
import os
from pathlib import Path, PurePosixPath
import re
import stat
import struct
import sys

import totctl

ROOT = Path(__file__).resolve().parents[1]
MAX_FILE = 16 * 1024 * 1024
ART_NAMES = ('terrain1', 'terrain2', 'units', 'cities', 'city', 'icons', 'people', 'dialog')
SCAN_ENTRY_LIMITS = {'diagnostics': 4096, 'vocabulary': 128, 'lines': 100000}
SCAN_REPORT_LIMITS = {'diagnostics': 8192, 'vocabulary': 1024, 'lines': 500000}
STRUCTURAL_DIAGNOSTICS = frozenset({
    'nested_event', 'misplaced_condition', 'malformed_then', 'malformed_endif',
    'unclosed_event', 'unclosed_payload', 'malformed_location',
    'stray_location_end', 'stray_text_end', 'misplaced_directive',
    'content_outside_event', 'content_after_endevents',
})


class ScanLimitReached(Exception):
    """Stop before retaining more scanner work than the declared budget."""

    def __init__(self, scope, category):
        self.scope = scope
        self.category = category


class ScanBudget:
    def __init__(self):
        self.used = Counter()

    def take(self, category, local):
        for scope, used, limits in [('report', self.used, SCAN_REPORT_LIMITS),
                                    ('entry', local, SCAN_ENTRY_LIMITS)]:
            if used[category] >= limits[category]:
                raise ScanLimitReached(scope, category)
        self.used[category] += 1
        local[category] += 1


def digest(data):
    return hashlib.sha256(data).hexdigest()


def relative(value, directory=False):
    if not isinstance(value, str) or not value or '\\' in value or ':' in value or any(ord(c) < 32 for c in value):
        raise ValueError('expected a contained POSIX relative path')
    if value == '.' and directory:
        return value
    p = PurePosixPath(value)
    if p.is_absolute() or any(x in ('', '.', '..') for x in value.split('/')):
        raise ValueError('absolute, empty and dot path components are forbidden')
    return value


def snapshot(home, name):
    relative(name)
    try:
        p = home.joinpath(*PurePosixPath(name).parts).resolve(strict=True)
    except RuntimeError as error:
        raise ValueError('source path cannot be resolved') from error
    if not p.is_relative_to(home):
        raise ValueError('source symlink escapes the installation')
    if not stat.S_ISREG(p.stat().st_mode):
        raise ValueError('source is not a regular file')
    with p.open('rb') as stream:
        data = stream.read(MAX_FILE + 1)
    if len(data) > MAX_FILE:
        raise ValueError('source exceeds 16 MiB')
    return data


def load_manifest(data):
    m = json.loads(data)
    if not isinstance(m, dict) or type(m.get('schema')) is not int or m['schema'] != 1 or not isinstance(m.get('entries'), list) or not 1 <= len(m['entries']) <= 64:
        raise ValueError('expected schema 1 with 1–64 entries')
    seen = set()
    for e in m['entries']:
        if not isinstance(e, dict) or not isinstance(e.get('id'), str) or not re.fullmatch('[a-z0-9][a-z0-9-]{0,63}', e['id']) or e['id'] in seen:
            raise ValueError('entry IDs must be unique bounded identifiers')
        seen.add(e['id'])
        if not isinstance(e.get('kind'), str) or not re.fullmatch('[a-z][a-z0-9-]{0,63}', e['kind']):
            raise ValueError('invalid entry kind')
        if 'first_entry_candidate' in e and not isinstance(e['first_entry_candidate'], bool):
            raise ValueError('candidate selection must be boolean')
        for key in ('save', 'rules'):
            relative(e.get(key))
        if 'events' not in e:
            raise ValueError('events must be an explicit path or null (not an inferred absence)')
        if e['events'] is not None:
            relative(e['events'])
        for key, maximum in [('lua', 16), ('art_roots', 8), ('declared_features', 16)]:
            if not isinstance(e.get(key), list) or len(e[key]) > maximum:
                raise ValueError('invalid bounded list: '+key)
        for path in e['lua']:
            relative(path)
        for path in e['art_roots']:
            relative(path, directory=True)
        if not all(isinstance(x, str) and re.fullmatch('[a-z_]{1,48}', x) for x in e['declared_features']):
            raise ValueError('invalid declared feature')
    if not isinstance(m.get('first_entry_profile'), str) or not re.fullmatch('[a-z0-9-]{1,64}', m['first_entry_profile']):
        raise ValueError('invalid first entry profile identifier')
    if sum(bool(e.get('first_entry_candidate')) for e in m['entries']) > 1:
        raise ValueError('only one first entry candidate may be selected')
    return m


def load_baseline(data):
    baseline = json.loads(data)
    if not isinstance(baseline, dict) or type(baseline.get('schema')) is not int or baseline['schema'] != 1:
        raise ValueError('unknown engine audit schema')
    if not isinstance(baseline.get('meaning'), str):
        raise ValueError('engine audit meaning must be a string')
    sources = baseline.get('source_sha256')
    if not isinstance(sources, dict) or not 1 <= len(sources) <= 64:
        raise ValueError('invalid bounded engine source audit')
    for name, expected in sources.items():
        relative(name)
        if not isinstance(expected, str) or not re.fullmatch('[0-9a-f]{64}', expected):
            raise ValueError('invalid engine source digest')
    if not isinstance(baseline.get('audit_ref'), str) or not re.fullmatch('[0-9a-f]{40}', str(baseline.get('audit_sha', ''))):
        raise ValueError('invalid engine audit identity')
    for key in ['triggers', 'actions']:
        table = baseline.get(key)
        if not isinstance(table, dict) or len(table) > 128:
            raise ValueError('invalid bounded engine vocabulary')
        for keyword, value in table.items():
            if not re.fullmatch('[A-Z][A-Z0-9_]{0,63}', keyword) or not isinstance(value, dict):
                raise ValueError('invalid engine vocabulary entry')
            if value.get('status') not in ['modelled_subset', 'partial', 'emitted_but_skipped', 'no_game_state_effect']:
                raise ValueError('invalid engine coverage status')
            params = value.get('params')
            if not isinstance(params, list) or len(params) > 32 or not all(isinstance(p, str) and re.fullmatch('[a-z][a-z0-9_]{0,63}', p) for p in params):
                raise ValueError('invalid engine parameter inventory')
    return baseline


def audit_current(baseline, root=ROOT):
    changed = []
    for name, expected in baseline['source_sha256'].items():
        relative(name)
        try:
            source = (root/name).resolve(strict=True)
            if not source.is_relative_to(root.resolve()) or digest(source.read_bytes()) != expected:
                changed.append(name)
        except OSError:
            changed.append(name)
    return {'ref': baseline['audit_ref'], 'sha': baseline['audit_sha'],
            'matches_current_source': not changed, 'changed_or_missing': sorted(changed),
            'meaning': baseline['meaning']}


def scan_events(text, baseline, budget=None):
    """Lexical inventory, not a replacement macro parser or compatibility proof."""
    counts = {'triggers': Counter(), 'actions': Counter(), 'logical': Counter()}
    diagnostics = []
    current = None
    phase = None
    want_trigger = False
    block = False
    payload = None
    events = 0
    began = False
    ended = False
    budget = budget if budget is not None else ScanBudget()
    local = Counter()
    overflow = None
    def diagnostic(code, line, token):
        budget.take('diagnostics', local)
        if not re.fullmatch(r'@?[A-Za-z][A-Za-z0-9_]{0,63}|<[^<>]{1,24}>', token):
            token = '<unrecognized>'
        diagnostics.append({'code': code, 'line': line, 'token': token})
    def count(phase, token):
        if token not in counts[phase]:
            budget.take('vocabulary', local)
        counts[phase][token] += 1
    try:
        for number, raw in enumerate(StringIO(text, newline=None), 1):
            budget.take('lines', local)
            stripped = raw.strip()
            upper = stripped.upper()
            if payload:
                if upper == payload:
                    payload = None
                elif payload == 'ENDLOCATIONS' and stripped and not stripped.startswith(';'):
                    fields = stripped.split(';', 1)[0].split(',')
                    if len(fields) not in (2, 3) or not all(re.fullmatch(r'-?[0-9]{1,10}', x.strip()) for x in fields):
                        diagnostic('malformed_location', number, 'LOCATIONS')
                    elif len(fields) == 3 and fields[2].strip().lstrip('-').strip('0'):
                        diagnostic('non_surface_location', number, 'LOCATIONS')
                continue
            t = stripped.split(';', 1)[0].strip()
            if not t:
                continue
            upper = t.upper()
            if upper in ('ENDTEXT', 'ENDLOCATIONS'):
                diagnostic('stray_text_end' if upper == 'ENDTEXT' else 'stray_location_end', number, upper)
                continue
            if ended:
                diagnostic('content_after_endevents', number,
                           upper if upper.startswith('@') else '<unrecognized>')
                continue
            if upper == '@BEGINEVENTS':
                if block or events or began:
                    diagnostic('misplaced_directive', number, upper)
                began = True
                continue
            if upper == '@DEBUG':
                if block:
                    diagnostic('misplaced_directive', number, upper)
                continue
            if upper == '@IF':
                if block:
                    diagnostic('nested_event', number, upper)
                block = True
                phase = 'triggers'
                want_trigger = True
                current = None
                events += 1
                continue
            if upper in ('@AND', '@OR'):
                count('logical', upper)
                diagnostic('unsupported_compound_condition', number, upper)
                if not block or phase != 'triggers':
                    diagnostic('misplaced_condition', number, upper)
                want_trigger = True
                current = None
                continue
            if upper == '@THEN':
                if not block or phase != 'triggers' or want_trigger:
                    diagnostic('malformed_then', number, upper)
                phase = 'actions'
                current = None
                continue
            if upper == '@ENDIF':
                if not block or phase != 'actions':
                    diagnostic('malformed_endif', number, upper)
                block = False
                phase = None
                current = None
                continue
            if upper == '@ENDEVENTS':
                if block:
                    diagnostic('unclosed_event', number, upper)
                block = False
                phase = None
                current = None
                ended = True
                continue
            if upper.startswith('@'):
                diagnostic('unsupported_directive', number, upper)
                continue
            if not block:
                diagnostic('content_outside_event', number, '<content>')
                continue
            if '=' in t:
                key = t.split('=', 1)[0].strip().lower()
                if not re.fullmatch('[a-z][a-z0-9_]{0,63}', key):
                    diagnostic('unrecognised_parameter', number, '<unrecognized>')
                    continue
                coverage = baseline.get(phase, {}).get(current, {})
                if key not in coverage.get('params', []):
                    diagnostic('unmodelled_parameter', number, key)
                continue
            if phase == 'triggers' and want_trigger:
                if not re.fullmatch('[A-Z][A-Z0-9_]{0,63}', upper):
                    diagnostic('unrecognised_trigger_line', number, '<unrecognized>')
                    current = '<unrecognized>'
                else:
                    current = upper
                count(phase, current)
                want_trigger = False
            elif phase == 'actions':
                if upper == 'LOCATIONS':
                    if current != 'CREATEUNIT':
                        diagnostic('placement_context_unmodelled', number, 'LOCATIONS')
                    payload = 'ENDLOCATIONS'
                    continue
                if re.fullmatch(r'-?\d+', t) and current == 'PLAYCDTRACK':
                    continue
                if not re.fullmatch('[A-Z][A-Z0-9_]{0,63}', upper):
                    diagnostic('unmodelled_argument', number, current or '<none>')
                    continue
                current = upper
                count(phase, current)
                if current == 'TEXT':
                    payload = 'ENDTEXT'
            else:
                diagnostic('unmodelled_trigger_argument', number, current or '<none>')
                continue
            coverage = baseline.get(phase, {}).get(current)
            if coverage is None:
                diagnostic('unimplemented_keyword', number, current)
            elif coverage['status'] != 'modelled_subset':
                diagnostic(coverage['status'], number, current)
        if payload:
            diagnostic('unclosed_payload', number if 'number' in locals() else 1, payload)
        if block:
            diagnostic('unclosed_event', number if 'number' in locals() else 1, '@IF')
    except ScanLimitReached as limit:
        overflow = {'scope': limit.scope, 'category': limit.category}
        # One reserved marker per entry remains possible after the shared budget.
        diagnostics.append({'code': 'scan_limit_exceeded', 'line': number if 'number' in locals() else 1,
                            'token': '<limit>', **overflow})
    observed = {}
    for phase in ('triggers', 'actions'):
        observed[phase] = {key: baseline.get(phase, {}).get(key, {'status': 'unimplemented', 'params': [], 'note': 'No audited handler.'}) for key in sorted(counts[phase])}
    return {'event_blocks': events, **{key: dict(sorted(value.items())) for key, value in counts.items()},
            'audited_baseline_coverage': observed, 'diagnostics': diagnostics,
            'scan_complete': overflow is None, 'overflow': overflow,
            'meaning': 'Lexical counts/line diagnostics only; recognition does not prove native semantics or an active runtime hook.'}


def inventory_art(home, roots):
    found, missing, errors = {}, [], []
    for stem in ART_NAMES:
        selected = None
        ambiguous = False
        for directory in roots:
            try:
                p = (home/directory).resolve()
            except (OSError, RuntimeError):
                errors.append({'code': 'resource_root_unavailable', 'path': directory})
                continue
            if not p.is_relative_to(home):
                errors.append({'code': 'resource_root_escape', 'path': directory})
                continue
            if not p.is_dir():
                continue
            children = []
            try:
                for child in p.iterdir():
                    children.append(child)
                    if len(children) > 4096:
                        break
            except OSError:
                errors.append({'code': 'resource_root_unreadable', 'path': directory})
                continue
            if len(children) > 4096:
                errors.append({'code': 'resource_directory_limit', 'path': directory})
                break
            for extension in ('.bmp', '.gif'):
                matches = [c for c in children if c.name.casefold() == stem+extension]
                if len(matches) > 1:
                    errors.append({'code': 'ambiguous_art', 'path': directory, 'stem': stem})
                    ambiguous = True
                    break
                if matches:
                    selected = matches[0]
                    break
            if selected or ambiguous:
                break
        if selected:
            name = selected.relative_to(home).as_posix()
            try:
                data = snapshot(home, name)
                found[stem] = {'path': name, 'bytes': len(data), 'sha256': digest(data)}
            except (OSError, ValueError) as e:
                errors.append({'code': 'art_read_error', 'stem': stem, 'reason': str(e).split(':')[0]})
        else:
            missing.append(stem)
    return {'files': found, 'missing': missing, 'diagnostics': errors,
            'policy': 'Declared roots in order; BMP then GIF in each. Inspector policy, not native scenario lookup proof.',
            'decoded': False}


def inspect_entry(home, e, baseline, current, scan_budget=None):
    result = {'id': e['id'], 'kind': e.get('kind', 'unspecified'), 'declared_features': e['declared_features'],
              'first_entry_candidate': bool(e.get('first_entry_candidate')), 'files': {}, 'diagnostics': []}
    data = {}
    paths = {'save': e['save'], 'rules': e['rules']}
    if e['events'] is not None:
        paths['events'] = e['events']
    paths.update({'lua_'+str(i): p for i, p in enumerate(e['lua'])})
    for role, name in paths.items():
        try:
            data[role] = snapshot(home, name)
            result['files'][role] = {'path': name, 'bytes': len(data[role]), 'sha256': digest(data[role])}
        except (OSError, ValueError) as error:
            result['diagnostics'].append({'code': 'source_unavailable', 'role': role, 'path': name, 'reason': str(error) if isinstance(error, ValueError) else type(error).__name__})
    probe = None
    if 'save' in data:
        try:
            if len(data['save']) < 12:
                raise ValueError('short save header')
            probe = totctl.walk_save_bytes(data['save'], e['save'])
            if probe.get('error') or 'map_header' not in probe:
                raise ValueError('not an inspected ToT save')
            result['save_probe'] = {'status': 'structurally_probed', 'version_raw': probe['version_raw'],
                'game': {k: probe.get('game', {}).get(k) for k in ('turn', 'year')},
                'map_header_raw': probe['map_header']['values'],
                'probe_surface_blocks': probe.get('sections_derived', {}).get('map_data', {}).get('map_blocks'),
                'transporter_records': probe.get('transporters', {}).get('record_count_u32@168'),
                'unit_count': probe.get('units', {}).get('count'), 'city_count': probe.get('cities', {}).get('count'),
                'note': 'Existing format probe, not full validation. Zero-based header index 3 and probe_surface_blocks are NOT an established active-map count.'}
            if any(isinstance(result['save_probe'][key], int) and result['save_probe'][key] < 0 for key in ('unit_count', 'city_count')):
                raise ValueError('negative record count')
            if result['save_probe']['unit_count'] is None or result['save_probe']['city_count'] is None:
                result['save_probe']['status'] = 'partial_layout'
        except (ValueError, IndexError, struct.error, KeyError, OverflowError):
            result['save_probe'] = {'status': 'failed', 'version_raw': probe.get('version_raw') if probe else None}
            reason = 'unsupported_version' if probe and probe.get('version_raw') is not None and probe['version_raw'] not in ('0x0032', '0x0033') else 'malformed_or_truncated'
            result['diagnostics'].append({'code': 'save_probe_failed', 'path': e['save'], 'reason': reason})
    if 'rules' in data:
        sections = totctl.parse_rules_text(data['rules'].decode('latin-1'))
        if len(sections) > 256 or any(not re.fullmatch('[A-Za-z][A-Za-z0-9_]{0,63}', k) for k in sections):
            result['diagnostics'].append({'code': 'rules_section_inventory_invalid'})
        result['rules_probe'] = {'section_rows': {k: len(v['rows']) for k, v in list(sections.items())[:256] if re.fullmatch('[A-Za-z][A-Za-z0-9_]{0,63}', k)},
                                 'semantics_validated': False}
        for required in ('UNITS', 'TERRAIN', 'CIVILIZE'):
            if not sections.get(required, {}).get('rows'):
                result['diagnostics'].append({'code': 'missing_rules_section', 'section': required})
    if 'events' in data:
        result['macro'] = scan_events(data['events'].decode('latin-1'), baseline, scan_budget)
        errors = sorted({d['code'] for d in result['macro']['diagnostics']
                         if d['code'] in STRUCTURAL_DIAGNOSTICS or d['code'] == 'scan_limit_exceeded'})
        if errors:
            result['diagnostics'].append({'code': 'macro_inspection_incomplete', 'path': e['events'], 'reasons': errors})
    result['dependency_closure'] = 'not_established: listed logic/sheets are pinned; full text/audio/video/Lua-module dependencies need a separate audit'
    result['lua'] = {'declared_files': len(e['lua']), 'executed': False, 'requirements_inferred_from_code': False}
    result['art'] = inventory_art(home, e['art_roots'])
    result['diagnostics'].extend(result['art']['diagnostics'])
    result['support'] = {'inspection': 'incomplete' if result['diagnostics'] else 'report_available',
        'engine_boot': 'not_run', 'simulation': 'not_admitted', 'observation': 'not_validated_for_input',
        'human_entry': 'not_implemented' if current['matches_current_source'] else 'audit_required',
        'human_campaign': 'not_validated', 'native_export': 'not_admitted'}
    blockers = ['no_runtime_conformance_or_import_acceptance', 'objectives_and_house_rules_not_inferred']
    if not current['matches_current_source']:
        blockers.append('engine_source_audit_stale')
    if e['lua'] or 'lua' in e['declared_features']:
        blockers.append('lua_execution_not_implemented')
    if 'multiple_maps' in e['declared_features']:
        blockers.append('multiple_map_simulation_not_implemented')
    if probe and probe.get('version_raw') == '0x0033':
        blockers.append('extended_save_not_admitted')
    if result.get('macro', {}).get('diagnostics'):
        blockers.append('macro_requires_unsupported_or_partial_semantics')
    if e['events'] is None:
        blockers.append('macro_absence_not_proven_by_manifest_null')
    if probe and probe.get('transporters', {}).get('record_count_u32@168', 0):
        blockers.append('native_transport_state_requires_map_audit')
    result['first_profile_structure'] = {
        'classic_032': bool(probe and probe.get('version_raw') == '0x0032'),
        'surface_declared': 'surface' in e['declared_features'] and 'multiple_maps' not in e['declared_features'],
        'classic_counts_probed': bool(probe and all(isinstance(probe.get(key, {}).get('count'), int) and probe[key]['count'] >= 0 for key in ('units', 'cities'))),
        'surface_geometry_probed': bool(probe and probe.get('map_decode', {}).get('dense') and all(v > 0 for v in probe['map_decode']['stored_dims']) and probe['map_header']['values'][0] % 2 == 0),
        'no_transport_records': bool(probe and probe.get('transporters', {}).get('record_count_u32@168') == 0),
        'no_lua_declared': not e['lua'] and 'lua' not in e['declared_features']}
    result['blockers'] = blockers
    return result


def report(home, manifest_bytes, baseline_bytes, source_root=ROOT):
    home = Path(home).resolve(strict=True)
    if not home.is_dir():
        raise ValueError('home must be an installation directory')
    if len(manifest_bytes) > 1024 * 1024 or len(baseline_bytes) > 1024 * 1024:
        raise ValueError('manifest/baseline exceeds 1 MiB')
    manifest = load_manifest(manifest_bytes)
    baseline = load_baseline(baseline_bytes)
    current = audit_current(baseline, source_root)
    scan_budget = ScanBudget()
    entries = [inspect_entry(home, e, baseline, current, scan_budget) for e in manifest['entries']]
    return {'schema': 1, 'view': 'full', 'purpose': 'Read-only inspection. No game/scripts are executed and no runtime admission is granted.',
        'producer': {'tool': 'tools/scenario_inventory.py', 'sha256': digest(Path(__file__).read_bytes()), 'format_probe_sha256': digest((ROOT/'tools/totctl.py').read_bytes())},
        'manifest_sha256': digest(manifest_bytes), 'baseline_sha256': digest(baseline_bytes), 'engine_audit': current,
        'profile_candidate': manifest.get('first_entry_profile'), 'entries': entries,
        'summary': {'entries': len(entries), 'inspection_errors': sum(bool(e['diagnostics']) for e in entries), 'simulation_admitted': 0}}


def compact_report(full):
    """Bound diagnostic examples while retaining counts and the full report hash."""
    text = json.dumps(full, indent=2, sort_keys=True)+'\n'
    entries = []
    for entry in full['entries']:
        item = dict(entry)
        if 'macro' in item:
            macro = dict(item['macro'])
            diagnostics = macro['diagnostics']
            macro['diagnostic_counts'] = dict(sorted(Counter(x['code'] for x in diagnostics).items()))
            macro['diagnostics_total'] = len(diagnostics)
            macro['diagnostics'] = diagnostics[:8]
            macro['diagnostics_truncated'] = len(diagnostics) > 8
            item['macro'] = macro
        entries.append(item)
    return full | {'view': 'compact', 'entries': entries,
                   'full_report_sha256': digest(text.encode()), 'full_report_bytes': len(text.encode())}


def read_document(path):
    if not stat.S_ISREG(path.stat().st_mode):
        raise ValueError('manifest/baseline must be a regular file')
    with path.open('rb') as stream:
        data = stream.read(1024 * 1024 + 1)
    if len(data) > 1024 * 1024:
        raise ValueError('manifest/baseline exceeds 1 MiB')
    return data


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--home', default=os.environ.get('CIVTOT_HOME'))
    parser.add_argument('--manifest', type=Path, default=ROOT/'docs/scenario-corpus.json')
    parser.add_argument('--out', type=Path, help='New report path; existing files are never overwritten')
    parser.add_argument('--compact', action='store_true', help='Bound diagnostic examples and include the full-report hash')
    parser.add_argument('--require', choices=['inspect', 'simulation'], default='inspect')
    args = parser.parse_args(argv)
    if not args.home:
        parser.error('set CIVTOT_HOME or pass --home')
    try:
        output = report(args.home, read_document(args.manifest), read_document(ROOT/'docs/scenario-engine-baseline.json'))
        if args.compact:
            output = compact_report(output)
        text = json.dumps(output, indent=2, sort_keys=True)+'\n'
        if args.out:
            with args.out.open('x') as stream:
                stream.write(text)
        else:
            print(text, end='')
        return int(bool(output['summary']['inspection_errors']) or args.require == 'simulation')
    except (OSError, ValueError, TypeError, RuntimeError) as e:
        print('scenario inventory: '+str(e), file=sys.stderr)
        return 2


if __name__ == '__main__':
    raise SystemExit(main())
