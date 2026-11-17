"""Short true-CLI coupling witness, not an OS timer/kill qualification.

No source rewriting or virtual clock. Transport is mocked and a sentinel stops
before the first post-ARM write. Source047 has no physical wrapper before it.
"""
import argparse
from contextlib import redirect_stdout
import hashlib
import io
import json
import os
from pathlib import Path
import runpy
import subprocess
import sys
import tempfile
from types import SimpleNamespace
import unittest
from unittest.mock import patch

BASELINE_SHA = '0471118fe427d8e5a52685d02c6f30c51ae5ffa2b91937f78e2caa86adf63f6f'
SOURCE = '8b9435363dbd8ec0e3973f2368b641189f0726eb'
NONCE = '0123456789abcdef0123456789abcdef'
OPTIONS = None


class Boundary(BaseException):
    """Control sentinel: never passed off as a CLI/infrastructure exception."""


class WatchdogEntry(unittest.TestCase):
    def test_physical_watchdog_precedes_post_arm_io_in_real_cli(self):
        original = Path(OPTIONS.exporter).absolute()
        raw = original.read_bytes()
        self.assertEqual(hashlib.sha256(raw).hexdigest(), BASELINE_SHA)
        proof = Path(OPTIONS.proof).absolute()
        proof.mkdir(mode=0o700, exist_ok=False)
        with tempfile.TemporaryDirectory(prefix='lucivy-fs-watchdog-entry-') as temp:
            root = Path(temp)
            exporter = root / 'export-fs-red-ledger.py'
            exporter.write_bytes(raw)  # Byte-for-byte frozen CLI, no AST rewriting.
            preflight = root / 'preflight.json'
            preflight.write_text(json.dumps(dict(
                package='/home/lucied/.cache/rag3weaver-build/cx-lucivy-fs-admission-red-17nov-v1',
                source=SOURCE, nonce=NONCE,
                copy_hashes={exporter.name: BASELINE_SHA})))
            destination = root / 'elf-ledger.json'
            launch_log = root / 'launch-ssh.log'
            events = []
            reads = set()
            boundary = []
            stdout = io.StringIO()
            path_open = Path.open

            def open_path(path, mode='r', *args, **kwargs):
                if path in (preflight, exporter) and mode in ('r', 'rb'):
                    reads.add(path.name)
                if path in (launch_log, destination) and any(c in mode for c in 'wax+'):
                    events.append('post-arm-write-frontier')
                    boundary.append(dict(path=path.name, mode=mode, actual_write=False))
                    raise Boundary()
                return path_open(path, mode, *args, **kwargs)

            def run_transport(argv, **kwargs):
                self.assertEqual(argv[:4], ['ssh', '-o', 'BatchMode=yes', 'luciepc'])
                body = kwargs['input']
                self.assertIn('expected=json.loads', body)
                self.assertIn('exporter-armed.json', body)
                # No execution of this body; emulate only the ARM response.
                events.append('arm-transport-mocked')
                return SimpleNamespace(returncode=0, stdout=json.dumps(dict(nonce=NONCE)), stderr='')

            def unexpected_popen(*args, **kwargs):
                self.fail('Popen reached before the post-ARM write sentinel')

            def exec_watchdog(path, argv):
                # Future wrapper can compose the real timeout primitive. Emulate
                # its child entry only; no process/timer is started by this test.
                self.assertEqual(path, '/usr/bin/timeout')
                self.assertIn('--signal=TERM', argv)
                self.assertIn('--kill-after=5s', argv)
                self.assertEqual(events.count('physical-watchdog-wrapper'), 0)
                interpreter = argv.index(sys.executable)
                self.assertEqual(Path(argv[interpreter + 1]), exporter)
                term = argv[interpreter - 1].removesuffix('s')
                self.assertGreater(float(term), 0)
                self.assertLessEqual(float(term) + 5, 300)
                events.append('physical-watchdog-wrapper')
                with patch.object(sys, 'argv', argv[interpreter + 1:]):
                    runpy.run_path(str(exporter), run_name='__main__')
                self.fail('Child entry unexpectedly returned before sentinel')

            argv = [str(exporter), '--preflight', str(preflight), '--output', str(destination),
                    '--launch-log', str(launch_log)]
            with patch.dict(os.environ), patch.object(sys, 'argv', argv), \
                 patch.object(Path, 'open', open_path), \
                 patch.object(subprocess, 'run', run_transport), \
                 patch.object(subprocess, 'Popen', unexpected_popen), \
                 patch.object(os, 'execv', exec_watchdog), redirect_stdout(stdout):
                try:
                    runpy.run_path(str(exporter), run_name='__main__')
                except Boundary:
                    pass
            observation = dict(source_sha256=BASELINE_SHA, real_cli=True,
                source_rewritten=False, virtual_clock=False, events=events,
                real_argparse_preflight_hash_json_reached=
                    reads == {preflight.name, exporter.name} and 'arm-transport-mocked' in events,
                boundary=boundary, network_executed=False, post_arm_write_executed=False,
                popen_executed=False, os_timer_or_kill_qualified=False,
                measured_300_seconds=False, captured_stdout=stdout.getvalue())
            (proof / 'observation.json').write_text(json.dumps(observation, indent=2))
            # These setup assertions must pass before the semantic coupling red.
            self.assertTrue(observation['real_argparse_preflight_hash_json_reached'])
            self.assertEqual(len(boundary), 1)
            self.assertEqual(boundary[0]['path'], launch_log.name)
            self.assertFalse(launch_log.exists())
            self.assertFalse(destination.exists())
            self.assertEqual(events[0], 'physical-watchdog-wrapper',
                'real CLI reached post-ARM I/O without an earlier physical watchdog wrapper')
            self.assertEqual(events, ['physical-watchdog-wrapper', 'arm-transport-mocked',
                                      'post-arm-write-frontier'])


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--exporter', required=True)
    parser.add_argument('--proof', required=True)
    OPTIONS = parser.parse_args()
    unittest.main(argv=[sys.argv[0]], verbosity=2)
