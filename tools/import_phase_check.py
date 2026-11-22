#!/usr/bin/env python3
"""Execute bounded saved-round and research-entry regions of civ2.bak.

No callee is replaced. The loader suffix starts AFTER network setup. The civ
guard stops before turn-start/restart/human/AI preparation. A separate pinned
source prefix executes human preparation and stops at unit setup 0x472ada;
it is not a whole human-input loop. Research stops before the first cost/advice/project-selection
callee, or returns normally. The original empty helper 0x40106b executes.
No chooser, recommendation, modal UI, city phase or live loader is executed.
Only derived numeric vectors and identities are emitted; no profile is admitted.
"""
import argparse
import hashlib
import itertools
import json
import os
from pathlib import Path
import struct

from unicorn import UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_EBP

from yields_check import OriginalCity
from totctl import walk_save

FRAME = 0x108800
CIV = 0x5F68A0
STRIDE = 3348
ROUND_STOPS = {0x474BB9: 0, 0x474B9A: 1, 0x474BE4: 2, 0x474C63: 3}
SCIENCE_STOPS = {0x100000: 0, 0x4A1AE0: 1, 0x477030: 2, 0x4A1436: 3}


def read(o, at, fmt='<i'):
    return struct.unpack(fmt, o.cpu.mem_read(at, struct.calcsize(fmt)))[0]


def run(o, start, stops, *, calls=(), instruction_limit=100000):
    reached = []
    allowed = set(calls)

    def watch(cpu, at, size, _):
        if at in stops:
            reached.append(at)
            cpu.emu_stop()
            return
        instruction = bytes(cpu.mem_read(at, size))
        if instruction[0] == 0xE8:
            target = at + 5 + struct.unpack('<i', instruction[1:5])[0]
            assert target in allowed, f'unexpected native call {at:x} -> {target:x}'

    hook = o.cpu.hook_add(UC_HOOK_CODE, watch)
    try:
        o.cpu.emu_start(start, 0, count=instruction_limit)
    finally:
        o.cpu.hook_del(hook)
    assert len(reached) == 1, hex(o.cpu.reg_read(o.ip))
    return reached[0]


def frame(o):
    o.cpu.reg_write(UC_X86_REG_EBP, FRAME)
    o.cpu.reg_write(o.sp, FRAME - 0x1000)


def loader(o, player, active, coords):
    frame(o)
    o.write(0x67A6E0, player, '<I')
    o.write(0x607655, active, '<B')
    o.write(0x5D1570, 255, '<B')
    o.write(0x5D0A0C, 0x12345678, '<I')
    for at in (0x602E84, 0x6031BC, 0x6031BE, 0x61E5D2):
        o.write(at, 0x4321, '<H')
    for at, value in zip((0x61E5CC, 0x61E5CE, 0x61E5D0), coords):
        o.write(at, value, '<h')
    run(o, 0x461D15, {0x461D7B})
    result = [read(o, 0x5D1570, '<B'), read(o, 0x5D0A0C),
              read(o, 0x602E84, '<H')]
    result += [read(o, at, '<h') for at in (0x602E86, 0x602E88, 0x602E8A)]
    result += [read(o, at, '<H') for at in (0x6031BC, 0x6031BE, 0x61E5D2)]
    assert result == [player, 0, 1, *coords, 0, 0, 0]
    return result


def round_guard(o, resume, current, restart, mask):
    frame(o)
    o.write(0x5D1570, resume, '<B')
    o.write(0x5D4FA0, restart, '<I')
    o.write(0x60765B, mask, '<B')
    o.write(FRAME - 0x6AC, current, '<I')
    return ROUND_STOPS[run(o, 0x474B62, ROUND_STOPS)]


def science(o, civ, progress, target, points, human, advice, acquired):
    block = CIV + STRIDE * civ
    o.write(block + 16, progress, '<h')
    o.write(block + 18, target, '<h')
    o.write(block + 24, acquired, '<B')
    o.write(0x60765B, (1 << civ) if human else 0, '<B')
    o.write(0x60763B, int(advice), '<B')
    before = bytes(o.cpu.mem_read(CIV, STRIDE * 8))
    o.cpu.mem_write(0x108000, struct.pack('<Iii', 0x100000, civ, points))
    o.cpu.reg_write(o.sp, 0x108000)
    stop = run(o, 0x4A1EBF, SCIENCE_STOPS,
               calls=(0x40106B, 0x4A1AE0, 0x477030, 0x4A1436))
    value = read(o, block + 16, '<h')
    after = bytearray(before)
    struct.pack_into('<h', after, STRIDE * civ + 16, value)
    assert bytes(o.cpu.mem_read(CIV, STRIDE * 8)) == bytes(after), 'unexpected civ mutation'
    if stop != 0x100000:
        assert read(o, o.cpu.reg_read(o.sp) + 4) == civ or stop == 0x477030
    return [value, SCIENCE_STOPS[stop]]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--out', type=Path, required=True)
    parser.add_argument('--vectors', type=Path, required=True)
    args = parser.parse_args()
    home = Path(os.environ['CIVTOT_HOME'])
    binary = (home / 'civ2.bak').read_bytes()
    source = home / 'Original/Ph_Auto2.SAV'
    data = source.read_bytes()
    reference = json.loads((Path(__file__).resolve().parent.parent /
                            'docs/evidence/import-knowledge.json').read_text())
    expected = reference['source']
    assert hashlib.sha256(binary).hexdigest() == reference['binary_sha256'], 'unpatched reference binary differs'
    assert hashlib.sha256(data).hexdigest() == expected['sha256'], 'pinned input differs'
    o = OriginalCity(binary)
    _, walk, _, _ = o.load_saved(home, 'Original/Ph_Auto2.SAV')
    preserved = [(CIV, STRIDE * 8), (0x607C58, walk['units']['count'] * 40),
                 (0x5FD150, walk['cities']['count'] * 92)]
    before = [bytes(o.cpu.mem_read(at, n)) for at, n in preserved]
    rows = [
        '# L player active x y z | resume round_start word602e84 x y z word6031bc word6031be word61e5d2',
        '# R resume current restart human_mask | 0 turn-start / 1 restart-setup / 2 human-preparation / 3 opponent-dispatch',
        '# S civ progress target points human advice acquired | resulting_progress boundary (0 return / 1 cost / 2 advice / 3 selector)',
        '# P saved_active_unit progress target | active_unit_at_native_unit_setup progress target (pinned input only)',
    ]
    counts = {'loader': 0, 'round_guard': 0, 'science': 0}
    for player, active, coords in itertools.product(range(1, 8), range(1, 8), ((0, 0, 0), (5, 7, 9))):
        out = loader(o, player, active, coords)
        rows.append('L ' + ' '.join(map(str, (player, active, *coords))) + ' | ' + ' '.join(map(str, out)))
        counts['loader'] += 1
    assert before == [bytes(o.cpu.mem_read(at, n)) for at, n in preserved]
    for resume in range(8):
        for current, restart, own in itertools.product(range(max(1, resume), 8), range(2), range(4)):
            bit = 1 << current
            mask = (0, bit, 255, 255 ^ bit)[own]
            out = round_guard(o, resume, current, restart, mask)
            want = 0 if resume == 0 else 1 if restart else 2 if mask & bit else 3
            assert out == want
            rows.append(f'R {resume} {current} {restart} {mask} | {out}')
            counts['round_guard'] += 1
    assert before == [bytes(o.cpu.mem_read(at, n)) for at, n in preserved]
    for i, values in enumerate(itertools.product(range(1, 8), (-32768, -1, 0, 32767),
                                                 (-1, 0, 89), (-1, 0, 1, 32768, 65536, 2147483647),
                                                 range(2), range(2))):
        civ, progress, target, points, human, advice = values
        acquired = i % 3
        out = science(o, *values, acquired)
        value = progress if points <= 0 else ((progress + points + 32768) % 65536) - 32768
        boundary = 0 if points <= 0 else 1 if target >= 0 else 0 if value == 0 else 2 if human and advice and acquired <= 1 else 3
        assert out == [value, boundary], (values, acquired, out)
        rows.append('S ' + ' '.join(map(str, (*values, acquired))) + ' | ' + ' '.join(map(str, out)))
        counts['science'] += 1
    # Fresh saved inputs for the positive witness, not residual synthetic state.
    o.load_saved(home, 'Original/Ph_Auto2.SAV')
    player, active = data[680], data[681]
    progress, target = struct.unpack_from('<hh', data, 2944 + STRIDE * player + 16)
    acquired = data[2944 + STRIDE * player + 24]
    witness = {'player': player, 'active': active, 'human_mask': data[687],
               'progress': progress, 'target': target,
               'round_boundary': round_guard(o, player, active, 0, data[687]),
               'science_0': science(o, player, progress, target, 0, True, bool(data[655] & 1), acquired),
               'science_1': science(o, player, progress, target, 1, True, bool(data[655] & 1), acquired)}
    # Source-only path: its advice bit is clear, so native human preparation
    # returns without a UI call. Stop before unit setup, not at an assumed input
    # loop or dialog. No other caller/callee is replaced.
    o.load_saved(home, 'Original/Ph_Auto2.SAV')
    loader(o, player, active, (0, 0, 0))
    round_guard(o, player, active, 0, data[687])
    frame(o)
    saved_unit = read(o, 0x60764E, '<h')
    records = [bytes(o.cpu.mem_read(at, n)) for at, n in preserved]
    stop = run(o, 0x474B62, {0x472ADA}, calls=(0x473BBE, 0x47355B, 0x472ADA))
    assert records == [bytes(o.cpu.mem_read(at, n)) for at, n in preserved]
    active_unit = read(o, 0x60764E, '<h')
    assert active_unit == -1
    witness['unit_setup_prefix'] = {'stop': f'{stop:08x}', 'saved_active_unit': saved_unit,
                                   'active_unit_at_boundary': active_unit,
                                   'civ_unit_city_records_unchanged': True,
                                   'advice_enabled': bool(data[655] & 1)}
    rows.append(f'P {saved_unit} {progress} {target} | {active_unit} {progress} {target}')
    counts['source_prefix'] = 1
    body = '\n'.join(rows) + '\n'
    ranges = [(0x461D15, 0x461D7B), (0x474B62, 0x474C63), (0x4A1EBF, 0x4A2098),
              (0x40106B, 0x40107B), (0x473BBE, 0x473E75), (0x47355B, 0x473585)]
    report = {'schema': 1, 'binary_sha256': hashlib.sha256(binary).hexdigest(),
              'source': {'path': expected['path'], 'sha256': expected['sha256']},
              'code_ranges': [{'start': f'{a:08x}', 'end': f'{b:08x}', 'sha256': hashlib.sha256(o.cpu.mem_read(a, b-a)).hexdigest()} for a,b in ranges],
              'cases': counts, 'vectors_sha256': hashlib.sha256(body.encode()).hexdigest(),
              'replaced_callees': [], 'runtime_admitted': False,
              'stop_addresses': {'round': {f'{a:08x}': b for a,b in ROUND_STOPS.items()},
                                 'science': {f'{a:08x}': b for a,b in SCIENCE_STOPS.items()}},
              'source_witness': witness, 'scope': __doc__.strip()}
    args.vectors.write_text(body)
    args.out.write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps({'cases': counts, 'source_witness': witness}))


if __name__ == '__main__':
    main()
