#!/usr/bin/env python3
"""Reproduce the installed saved-human-view integration regression.

This runs the Rust human-session test, not original-game instructions or UI.
Its saved projection uses the separately native-verified reader gates. Source
bytes are checked against the existing pinned corpus identity and stay private.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile


def main():
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('--out',type=Path,required=True)
    args=p.parse_args()
    root=Path(__file__).resolve().parent.parent
    source=Path(os.environ['CIVTOT_HOME'])/'Original/Ph_Auto2.SAV'
    expected=json.loads((root/'docs/evidence/import-knowledge.json').read_text())['source']
    data=source.read_bytes();sha=hashlib.sha256(data).hexdigest()
    assert sha==expected['sha256'],'the pinned Original positive fixture differs'
    with tempfile.TemporaryDirectory(prefix='totengine-saved-view-') as folder:
        raw=Path(folder)/'result.json'
        env=os.environ.copy();env['TOTENGINE_SAVED_VIEW_EVIDENCE']=str(raw)
        cargo=shutil.which('cargo') or str(Path.home()/'.cargo/bin/cargo')
        subprocess.run([cargo,'test','-p','engine','--lib','human::saved_view::tests::installed_original_saved_view_matches_the_pinned_reader_projection','--','--exact'],cwd=root,env=env,check=True)
        report=json.loads(raw.read_text())
    report['method']=__doc__.strip()
    report['initial_decision']='Research required by the existing engine; native prompt timing is not claimed.'
    report['first_gameplay_order']='Research'
    report['source']={'path':expected['path'],'sha256':sha}
    report['dependencies']='Structural engine-test manifest only; not resource/admission closure.'
    report['limits']='Saved initial human map/entities/city sizes, zero-work polling and checkpoints. Subsequent gameplay uses engine observation rules. No faithful profile, native AI/RNG continuation or native UI execution is admitted.'
    args.out.write_text(json.dumps(report,indent=2)+'\n')
    print(f"saved view: {report['initial_seen_tiles']} tiles; legacy/engine continuation: {report['legacy_initial_seen_tiles']}/{report['after_first_gameplay_order_seen_tiles']}")


if __name__=='__main__':
    main()
