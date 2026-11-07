#!/usr/bin/env python3
"""Capture the five original normalized city happiness stages without UI hooks.

Uses the existing isolated original comparison setup. Its garrison count is
explicitly supplied for governments <4; this does not prove map lookup parity.
Only numeric synthetic inputs/results are retained; CIVTOT_HOME + Unicorn needed.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import random
import struct

from happiness_check import OriginalHappiness, happiness

# Return addresses of the five original normalization calls, stages 0..4.
RETURNS = (0x4c4b1b, 0x4c4b36, 0x4c4cc4, 0x4c4f0c, 0x4c4fa6)


def main():
    from unicorn import UC_HOOK_CODE
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--gold', type=Path, required=True)
    parser.add_argument('--vectors', type=Path, required=True)
    args = parser.parse_args()
    binary = (Path(os.environ['CIVTOT_HOME']) / 'civ2.bak').read_bytes()
    original = OriginalHappiness(binary)
    stages = []
    def capture(cpu, address, size, data):
        stages.append((RETURNS.index(address), tuple(struct.unpack('<i', cpu.mem_read(a, 4))[0]
                      for a in (0x6537d0, 0x653828, 0x65381c))))
    for address in RETURNS:
        original.cpu.hook_add(UC_HOOK_CODE, capture, begin=address, end=address)
    rng = random.Random(157)
    lines = []
    changing = 0
    for _ in range(256):
        size = rng.randrange(1, 40)
        local = rng.getrandbits(28)
        v = (rng.randrange(20), size, rng.randrange(7), rng.randrange(6),
             rng.randrange(2), rng.randrange(1, 80), rng.choice((5, 7, 9)), 14,
             rng.choice((0, 4)), rng.randrange(min(size, 16) + 1), rng.randrange(80),
             rng.randrange(8), rng.randrange(8), rng.randrange(8),
             sum(1 << i for i in (1, 4, 7, 11, 14, 33) if rng.randrange(2)),
             rng.getrandbits(100), rng.getrandbits(28) | local, local)
        stages.clear()
        result = original.run(v)
        assert [i for i, _ in stages] == list(range(5)), stages
        assert stages[-1][1] == result == happiness(v)
        changing += len({s for _, s in stages}) > 1
        lines.append(' '.join(map(str, (*v, *(n for _, s in stages for n in s)))))
    args.vectors.write_text('# index size government difficulty human cities content empire flags specialists luxuries garrison away home buildings techs own local; five happy unhappy angry triples: base luxury building military wonder\n' + '\n'.join(lines) + '\n')
    report = {'schema': 1, 'binary_sha256': hashlib.sha256(binary).hexdigest(),
              'contexts': 256, 'stage_snapshots': 1280, 'contexts_with_stage_changes': changing,
              'native_normalize_return_addresses': [hex(a) for a in RETURNS],
              'stages': ['base', 'luxuries', 'buildings', 'military', 'wonders'],
              'fields': ['happy', 'unhappy', 'angry'],
              'scope': 'Unmodified original arithmetic and normalization with numeric synthetic inputs. Existing OriginalHappiness setup supplies government<4 garrison count and skips its map enumeration; wonders/tech/building helpers execute original code. This is not full map-input derivation, original citizen icon interpretation or complete city UI parity.'}
    args.gold.write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps(report))


if __name__ == '__main__':
    main()
