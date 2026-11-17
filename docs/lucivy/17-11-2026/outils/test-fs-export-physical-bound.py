"""Proposed RED on the exact published CLI, no real SSH or corpus.

Run only after the witness itself has been published/reviewed. This takes the
actual 300s policy, without a virtual clock or a newly invented CLI option.
The independent emergency cleanup is 302s, and is NEVER called a product bound.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import signal
import subprocess
import sys
import tempfile
import time
import unittest

BASELINE_SHA = '0471118fe427d8e5a52685d02c6f30c51ae5ffa2b91937f78e2caa86adf63f6f'
SOURCE = '8b9435363dbd8ec0e3973f2368b641189f0726eb'
NONCE = '0123456789abcdef0123456789abcdef'
OPTIONS = None


class PhysicalBound(unittest.TestCase):
    def test_real_cli_stops_within_300s_when_local_fsync_blocks(self):
        source = Path(OPTIONS.exporter).absolute()
        raw = source.read_bytes()
        self.assertEqual(hashlib.sha256(raw).hexdigest(), BASELINE_SHA)
        proof = Path(OPTIONS.proof).absolute()
        proof.mkdir(mode=0o700, exist_ok=False)
        with tempfile.TemporaryDirectory(prefix='lucivy-fs-export-bound-') as temp:
            root = Path(temp)
            exporter = root / 'export-fs-red-ledger.py'
            exporter.write_bytes(raw)
            preflight = root / 'preflight.json'
            preflight.write_text(json.dumps(dict(
                package='/home/lucied/.cache/rag3weaver-build/cx-lucivy-fs-admission-red-17nov-v1',
                source=SOURCE, nonce=NONCE,
                copy_hashes={exporter.name: BASELINE_SHA})))
            # This is a transport double, NOT an ssh connection or remote executor.
            fake = root / 'ssh'
            fake.write_text('''#!/usr/bin/env python3
import hashlib,json,os,pathlib,sys,time
root=pathlib.Path(os.environ["FS_BOUND_FIXTURE"])
body=sys.stdin.read()
with (root/"transport-events.jsonl").open("a") as f:
 f.write(json.dumps(dict(argv=sys.argv[1:],pid=os.getpid(),body_kind=
   "launch" if sys.argv[-1].startswith("cd ") else
   "arm" if "expected=json.loads" in body else
   "ledger" if "elf-ledger-ready.json" in body else "unexpected"))+"\\n")
if sys.argv[-1].startswith("cd "):
 (root/"fake-launch-pid.json").write_text(json.dumps(dict(pid=os.getpid())))
 time.sleep(1200)
elif "expected=json.loads" in body:
 print(json.dumps(dict(nonce=os.environ["FS_BOUND_NONCE"])))
elif "elf-ledger-ready.json" in body:
 raw=json.dumps(dict(source=os.environ["FS_BOUND_SOURCE"],artifacts={}))
 print(json.dumps(dict(nonce=os.environ["FS_BOUND_NONCE"],raw=raw,
  sha256=hashlib.sha256(raw.encode()).hexdigest())))
else:
 sys.exit("Unexpected transport stage: no ACK/final is expected while fsync blocks")
''')
            fake.chmod(0o700)
            # Inject only the local destination's fsync. All earlier CLI checks,
            # arm/launch requests, JSON parsing, SHA and the write are real code.
            (root / 'sitecustomize.py').write_text('''import json,os,pathlib,threading,time
original=os.fsync
def fsync(fd):
 root=pathlib.Path(os.environ["FS_BOUND_FIXTURE"])
 target=root/"elf-ledger.json"
 if target.exists() and os.fstat(fd).st_ino==target.stat().st_ino:
  (root/"fsync-entered.json").write_text(json.dumps(dict(
   pid=os.getpid(),entered=time.monotonic(),fault="blocked destination fsync")))
  threading.Event().wait()
 return original(fd)
os.fsync=fsync
''')
            env = dict(os.environ, PATH=str(root) + os.pathsep + os.environ['PATH'],
                       PYTHONPATH=str(root), FS_BOUND_FIXTURE=str(root),
                       FS_BOUND_NONCE=NONCE, FS_BOUND_SOURCE=SOURCE)
            # Use the exact published entry shape, without an added watchdog.
            argv = [sys.executable, str(exporter), '--preflight', str(preflight),
                    '--output', str(root / 'elf-ledger.json'),
                    '--launch-log', str(root / 'launch-ssh.log')]
            start = time.monotonic()
            deadline = start + 300
            emergency = start + 302
            child = None
            exited_by_deadline = False
            observed_after_deadline = None
            try:
                with (proof / 'cli.log').open('xb') as log:
                    child = subprocess.Popen(argv, env=env, stdout=log,
                        stderr=subprocess.STDOUT, start_new_session=True)
                    while time.monotonic() < deadline:
                        if child.poll() is not None:
                            exited_by_deadline = True
                            break
                        time.sleep(min(0.1, max(0, deadline - time.monotonic())))
                    if not exited_by_deadline:
                        observed_after_deadline = dict(observed=time.monotonic(),
                            pid=child.pid, alive=child.poll() is None)
                        while child.poll() is None and time.monotonic() < emergency:
                            time.sleep(0.1)
            finally:
                if child is not None:
                    # Kill the fixture's whole process group, including fake launch.
                    try:
                        os.killpg(child.pid, signal.SIGKILL)
                    except ProcessLookupError:
                        pass
                    child.wait(timeout=2)
                for name in ['transport-events.jsonl', 'fsync-entered.json',
                             'fake-launch-pid.json', 'elf-ledger.json', 'launch-ssh.log']:
                    if (root / name).exists():
                        (proof / name).write_bytes((root / name).read_bytes())
                record = dict(baseline_sha256=BASELINE_SHA, source=SOURCE,
                    start=start, deadline=deadline, emergency_deadline=emergency,
                    observed=time.monotonic(), exited_by_deadline=exited_by_deadline,
                    observed_after_deadline=observed_after_deadline,
                    child_code=None if child is None else child.returncode,
                    fault='local destination fsync blocked', real_cli=True,
                    real_ssh=False, real_corpus=False, cargo=False,
                    emergency_cleanup_is_not_product_bound=True)
                with (proof / 'observation.json').open('x') as file:
                    json.dump(record, file, indent=2); file.flush(); os.fsync(file.fileno())
            # Guard against an API/setup failure masquerading as the desired red.
            self.assertTrue((proof / 'fsync-entered.json').exists(), 'CLI never reached injected fsync')
            events = [json.loads(line) for line in (proof / 'transport-events.jsonl').read_text().splitlines()]
            self.assertEqual(sorted(r['body_kind'] for r in events), ['arm', 'launch', 'ledger'])
            self.assertTrue((proof / 'elf-ledger.json').exists())
            self.assertTrue(exited_by_deadline,
                'exact published CLI still alive at 300s inside local fsync; emergency cleanup was external')


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--exporter', required=True)
    parser.add_argument('--proof', required=True)
    OPTIONS = parser.parse_args()
    unittest.main(argv=[sys.argv[0]], verbosity=2)
