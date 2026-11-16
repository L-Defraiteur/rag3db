"""Prepare a source-pinned harness package; the previous native allocation is closed."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import time

ORIGINAL_END = 852082.791694931
PRODUCT = '6d886375ea4627c88d28aef918b948a23f628ab1'


def sha(path):
    return hashlib.file_digest(Path(path).open('rb'), 'sha256').hexdigest()


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--source', required=True)
    parser.add_argument('--prepare-only', action='store_true')
    args = parser.parse_args()
    w = Path(__file__).resolve().parents[3]
    git = lambda *parts: subprocess.check_output(['git', '-C', str(w), *parts], text=True).strip()
    assert git('rev-parse', 'HEAD') == args.source and not git('status', '--porcelain')
    base = Path.home() / '.cache/rag3weaver-build'
    q = base / 'cx-tty-harness-screen-review-v2'
    if not args.prepare_only:
        # This gate cannot turn an expired allocation into permission to run.
        assert time.monotonic() < ORIGINAL_END, 'original allocation expired; native run closed'
        raise RuntimeError('old GO closed; preparation is not a new allocation')
    assert not q.exists()
    q.mkdir(mode=0o700)
    # Compare all production sources, allowing only the named Python harness correction.
    changed = git('diff', '--name-only', PRODUCT, '--', 'extension/rag3weaver').splitlines()
    allowed = {'extension/rag3weaver/tty/mesures/mesure.py',
               'extension/rag3weaver/tty/mesures/harness_observation.py',
               'extension/rag3weaver/tty/mesures/test_harness_observation.py'}
    assert set(changed) <= allowed
    carried = base / 'cx-tty-english-aliases-full-v5'
    logs = sorted((carried / 'features').glob('*.log'))
    assert len(logs) == 8 and (carried / 'f8.log').read_text().splitlines()[-1] == 'CHECK_FEATURES VERT 8/8'
    build = base / 'cx-tty-aliases-continuation-v1'
    assert (build / 'build.rc').read_text().strip() == '0'
    elf = base / 'target-cx-tty-english-aliases-2/debug/rag3weaver-tty'
    elf_sha = (build / 'tty-elf-sha.txt').read_text().split()[0]
    assert sha(elf) == elf_sha
    scope = base / 'cx-tty-aliases-continuation-common-v1/validation-flsb10fe'
    authority = json.loads((scope / 'autorite.json').read_text())
    verdict = json.loads((scope / 'verdict.json').read_text())
    assert authority['etat'] == 'termine' and verdict['restes'] == [] and verdict['nettoye']
    inputs = [w / 'extension/rag3weaver/tty/mesures' / name for name in
              ['mesure.py', 'faux_hote.py', 'harness_observation.py', 'test_harness_observation.py']]
    inputs += [w / 'docs/tty/16-11-2026/run_private_harness.py', Path(__file__)]
    copies = {}
    for path in inputs:
        shutil.copy2(path, q / path.name)
        assert sha(path) == sha(q / path.name)
        copies[path.name] = sha(path)
    config = dict(source=args.source, product=PRODUCT, changed_harness_only=changed,
                  files=copies, previous_end=ORIGINAL_END, prepared_at=time.monotonic(),
                  hostname=os.uname().nodename, native_gate='CLOSED_EXPIRED_ALLOCATION',
                  f8_carried={str(p):sha(p) for p in [carried / 'f8.log', carried / 'source.txt', *logs]},
                  build_elf=str(elf), build_elf_sha=elf_sha,
                  raw_does_not_requalify_lost_run=True, runtime_executed=False,
                  next_scope='separate protocol and visible-screen assertions on private tmux; fresh GO required')
    (q / 'prepared.json').write_text(json.dumps(config, indent=2))
    print('PREPARED_ONLY', q)


if __name__ == '__main__':
    main()
