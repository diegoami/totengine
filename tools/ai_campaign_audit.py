#!/usr/bin/env python3
"""Numeric campaign cohort; engine comparisons, not native-turn parity.

Every run independently replays its complete order log; generated runs also
restore a halfway checkpoint and require byte-identical continuation. No
original assets are written. Requires built ai_policy_audit example.
"""
import argparse
from concurrent.futures import ThreadPoolExecutor
import hashlib
import json
import os
from pathlib import Path
import subprocess


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--engine-example', type=Path, required=True)
    p.add_argument('--report', type=Path, required=True)
    args = p.parse_args()
    home = Path(os.environ['CIVTOT_HOME'])
    sources = [('generated', home/'Original/Rules.txt', (7, 19, 43), 100, range(6)),
               ('standard', home/'Original/Ph_Auto2.SAV', (7, 19), 100, range(6)),
               ('fallrome', home/'FallRome/ROMFALL.scn', (7,), 25, (0, 5))]
    jobs = [(label, path, seed, difficulty, policy, turns)
            for label, path, seeds, turns, difficulties in sources for seed in seeds
            for difficulty in difficulties for policy in ('ai-v1', 'builtin-1')]
    def run(job):
        label, path, seed, difficulty, policy, turns = job
        output = subprocess.check_output([str(args.engine_example.resolve()), str(path), str(turns), str(seed), str(difficulty), policy], text=True)
        result = json.loads(output)
        result['source'] = label
        return result
    with ThreadPoolExecutor(max_workers=4) as pool:
        results = []
        for result in pool.map(run, jobs):
            results.append(result)
            args.report.with_suffix(".partial.json").write_text(json.dumps(results, indent=2)+"\n")
            print(f"{len(results)}/{len(jobs)} {result['source']} {result['policy']} seed={result['seed']} difficulty={result['difficulty']}", flush=True)
    provenance = []
    for label, path, seeds, _, _ in sources:
        item = {'source': label, 'input_sha256': hashlib.sha256(path.read_bytes()).hexdigest()}
        if label != 'generated':
            item['rules_sha256'] = hashlib.sha256((path.parent/'Rules.txt').read_bytes()).hexdigest()
        provenance.append(item)
    report = {'scope': 'Engine policy trajectories on 100-round generated/standard and 25-round FallRome positions, '
                       'with difficulty edited only in memory through the typed save editor. '
                       'Original native turns are not executed; these are sustainability and replay checks, '
                       'not measurements of original campaign parity. Saved games retain their mechanics human mask; '
                       'every live civ is driven by the named AI policy. Events are disabled in this mechanics cohort.',
              'metrics': ['cities', 'population', 'units', 'acquired_advances', 'improvement_bits', 'treasury', 'living_civs'],
              'sources': provenance, 'runs': results}
    args.report.write_text(json.dumps(report, indent=2)+'\n')
    args.report.with_suffix('.partial.json').unlink()
    for label, _, _, _, _ in sources:
        for policy in ('ai-v1', 'builtin-1'):
            group = [r for r in results if r['source'] == label and r['policy'] == policy]
            print(label, policy, len(group), 'runs;', 'attacks', sum(r['attacks'] for r in group),
                  'occupations', sum(r['occupations'] for r in group),
                  'cities', min(r['final'][0] for r in group), max(r['final'][0] for r in group), flush=True)


if __name__ == '__main__':
    main()
