#!/usr/bin/env python3
"""Structure inventory of every Civilopedia `Describe.txt` in an install (#201).

Independent of the Rust decoder (`tot_kit::Pedia`): the committed summary holds only
counts and SHA-256 hashes, never text, and `crates/tot-kit/tests/pedia_install.rs`
checks the Rust parser against it. Usage:
  CIVTOT_HOME=/path/to/CIVTOT python3 tools/pedia_inventory.py --out docs/evidence/pedia-structure-v1.json
"""
import argparse
import hashlib
import json
import os
from pathlib import Path


def summarize(path):
    raw = path.read_bytes()
    text = raw.decode('latin-1').replace('\r\n', '\n').replace('\r', '\n')
    sections, cur, in_index, entry_open = [], None, False, False
    for line in text.split('\n'):
        if line.startswith(';'):
            continue
        if in_index:
            body = line.split(';')[0].strip()
            if not body:
                continue
            n = int(body.rstrip(',').strip())
            if n == -2:
                in_index = False
            else:
                cur['index'].append(n)
            continue
        if line.startswith('@@'):
            if line.endswith('_INDEX'):
                cur['index'] = []
                in_index = True
            else:
                cur['entries'] += 1
        elif line.startswith('@'):
            if line.startswith('@This must be here'):
                break
            name = line[1:].strip()
            cur = {'name': name[:-len('_DESCRIPTIONS')] if name.endswith('_DESCRIPTIONS') else name,
                   'index': None, 'entries': 0}
            sections.append(cur)
    out = []
    for s in sections:
        idx = s['index']
        out.append({'name': s['name'], 'entries': s['entries'],
                    'index_len': None if idx is None else len(idx),
                    'index_described': None if idx is None else sum(1 for v in idx if v >= 0),
                    'index_unresolved': None if idx is None else sum(1 for v in idx if v >= s['entries'])})
    return {'bytes': len(raw), 'sha256': hashlib.sha256(raw).hexdigest(), 'sections': out}


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('--home', default=os.environ.get('CIVTOT_HOME'))
    ap.add_argument('--out')
    a = ap.parse_args()
    if not a.home:
        raise SystemExit('set CIVTOT_HOME')
    home = Path(a.home)
    files = {}
    for p in sorted(home.rglob('*')):
        if p.is_file() and p.name.lower() == 'describe.txt' and len(p.relative_to(home).parts) <= 3:
            files[p.relative_to(home).as_posix()] = summarize(p)
    result = {'schema': 1, 'files': files,
              'note': 'Counts and hashes only; Describe.txt text is copyrighted and never committed.'}
    text = json.dumps(result, indent=2, sort_keys=True) + '\n'
    if a.out:
        if os.path.exists(a.out):
            raise SystemExit('refusing to overwrite ' + a.out)
        Path(a.out).write_text(text)
    else:
        print(text)


if __name__ == '__main__':
    main()
