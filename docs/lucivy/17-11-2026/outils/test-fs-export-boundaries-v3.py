"""CLI order, elapsed-budget transport and error cleanup; no real OS timer/SSH.

All process/parent metadata are doubles. The real source/args/hash/JSON and
small owned file writes run; no clock is replaced and no remote body runs.
"""
import argparse
import ast
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
import time
from types import SimpleNamespace
import unittest
from unittest.mock import patch

OPTIONS = None
SOURCE = '8b9435363dbd8ec0e3973f2368b641189f0726eb'
NONCE = '0123456789abcdef0123456789abcdef'


class Frontier(BaseException):
    pass


def exercise(scenario, parent_age):
    source = Path(OPTIONS.exporter).absolute()
    raw = source.read_bytes()
    assert hashlib.sha256(raw).hexdigest() == OPTIONS.exporter_sha256
    proof = Path(OPTIONS.proof).absolute() / scenario
    proof.mkdir(mode=0o700, parents=True, exist_ok=False)
    events, reads, durations, wrapper = [], set(), [], []
    captured = io.StringIO()
    error = None
    with tempfile.TemporaryDirectory(prefix='lucivy-fs-boundaries-') as temp:
        root = Path(temp)
        exporter = root / 'export-fs-red-ledger.py'
        exporter.write_bytes(raw)
        preflight = root / 'preflight.json'
        # Both frozen baselines and proposed v2 are supported without rewriting.
        version = 'v2' if b'fs-admission-red-17nov-v2' in raw else 'v1'
        preflight.write_text(json.dumps(dict(source=SOURCE, nonce=NONCE,
            package='/home/lucied/.cache/rag3weaver-build/cx-lucivy-fs-admission-red-17nov-' + version,
            copy_hashes={exporter.name: OPTIONS.exporter_sha256})))
        ledger = root / 'elf-ledger.json'
        launch_log = root / 'launch-ssh.log'
        real_open, real_fsync, real_readlink = Path.open, os.fsync, os.readlink
        fake_parent = 424242
        parent_tick = int((time.clock_gettime(time.CLOCK_BOOTTIME) - parent_age) * os.sysconf('SC_CLK_TCK'))

        class Launch:
            def __init__(self):
                self.stdin = io.BytesIO()
                self.running = True
            def poll(self):
                return None if self.running else 0
            def terminate(self):
                events.append('terminate-local-child-mocked')
                self.running = False
            def kill(self):
                events.append('kill-local-child-mocked')
                self.running = False
            def wait(self, timeout):
                events.append('wait-local-child-mocked')
                assert not self.running, 'unexpected wait before injected I/O error'
                assert timeout > 0
                return 0

        launch = Launch()

        def open_path(path, mode='r', *args, **kwargs):
            if path == Path(f'/proc/{fake_parent}/cmdline'):
                assert wrapper
                return io.BytesIO(bytes([0]).join(x.encode() for x in wrapper) + bytes([0]))
            if path == Path(f'/proc/{fake_parent}/stat'):
                fields = ['S'] + ['0'] * 18 + [str(parent_tick)]
                return io.StringIO(f'{fake_parent} (timeout) ' + ' '.join(fields))
            if path in (preflight, exporter) and mode in ('r', 'rb'):
                reads.add(path.name)
            if path == launch_log and any(c in mode for c in 'wax+') and scenario != 'io-error':
                events.append('post-arm-write-frontier')
                raise Frontier()
            return real_open(path, mode, *args, **kwargs)

        def readlink(path, *args, **kwargs):
            if str(path) == f'/proc/{fake_parent}/exe':
                return '/usr/bin/timeout'
            return real_readlink(path, *args, **kwargs)

        def exec_watchdog(path, argv):
            assert path == '/usr/bin/timeout'
            assert '--signal=TERM' in argv and '--kill-after=5s' in argv
            assert not wrapper, 'unexpected recursive wrapping'
            interpreter = argv.index(sys.executable)
            assert Path(argv[interpreter + 1]) == exporter
            assert 0 < float(argv[interpreter - 1].removesuffix('s')) <= 295
            wrapper.extend(argv)
            events.append('physical-watchdog-wrapper')
            with patch.object(sys, 'argv', argv[interpreter + 1:]), \
                 patch.object(os, 'getppid', lambda: fake_parent):
                runpy.run_path(str(exporter), run_name='__main__')
            raise AssertionError('worker returned before expected boundary')

        def transport(argv, **kwargs):
            assert argv[:4] == ['ssh', '-o', 'BatchMode=yes', 'luciepc']
            assert 0 < kwargs['timeout'] <= 8
            body = kwargs['input']
            if 'expected=json.loads' in body:
                # Inspect the duration expression; never execute the remote body.
                tree = ast.parse(body)
                values = [kw.value for node in ast.walk(tree) if isinstance(node, ast.Call)
                          for kw in node.keywords if kw.arg == 'physical_deadline']
                assert len(values) == 1
                expression = values[0]
                assert isinstance(expression, ast.BinOp) and isinstance(expression.op, ast.Add)
                assert isinstance(expression.left, ast.Name) and expression.left.id == 'now'
                assert isinstance(expression.right, ast.Constant)
                duration = expression.right.value
                assert isinstance(duration, (float, int)) and duration > 0
                durations.append(duration)
                events.append('arm-transport-mocked')
                return SimpleNamespace(returncode=0,
                    stdout=json.dumps(dict(nonce=NONCE, budget_seconds=duration)), stderr='')
            if 'elf-ledger-ready.json' in body:
                events.append('ledger-transport-mocked')
                text = json.dumps(dict(source=SOURCE, artifacts={}))
                return SimpleNamespace(returncode=0, stdout=json.dumps(dict(nonce=NONCE,
                    raw=text, sha256=hashlib.sha256(text.encode()).hexdigest())), stderr='')
            raise AssertionError('ACK/final transport reached after injected fsync error')

        def popen(argv, **kwargs):
            assert scenario == 'io-error'
            assert argv[:4] == ['ssh', '-o', 'BatchMode=yes', 'luciepc']
            events.append('launch-child-mocked')
            return launch

        def fsync(fd):
            if ledger.exists() and os.fstat(fd).st_ino == ledger.stat().st_ino:
                events.append('ledger-fsync-error-injected')
                raise OSError('controlled ledger fsync error')
            return real_fsync(fd)

        argv = [str(exporter), '--preflight', str(preflight), '--output', str(ledger),
                '--launch-log', str(launch_log)]
        with patch.dict(os.environ), patch.object(sys, 'argv', argv), \
             patch.object(Path, 'open', open_path), patch.object(os, 'readlink', readlink), \
             patch.object(os, 'execv', exec_watchdog), patch.object(os, 'fsync', fsync), \
             patch.object(subprocess, 'run', transport), patch.object(subprocess, 'Popen', popen), \
             redirect_stdout(captured):
            try:
                runpy.run_path(str(exporter), run_name='__main__')
            except Frontier:
                error = 'post-arm-write-frontier'
            except OSError as caught:
                assert str(caught) == 'controlled ledger fsync error'
                error = str(caught)
        observation = dict(source_sha256=OPTIONS.exporter_sha256, mode=scenario,
            events=events, real_args_hash_json_reached=reads == {preflight.name, exporter.name},
            parent_metadata_age_seconds=parent_age, transmitted_durations=durations,
            error=error, local_mock_child_running=launch.running if scenario == 'io-error' else None,
            source_rewritten=False, virtual_clock=False, real_proc_read=False,
            real_ssh=False, real_popen=False, real_signal=False, real_remote_body=False,
            os_timer_kill_or_cleanup_qualified=False, captured_stdout=captured.getvalue())
        (proof / 'observation.json').write_text(json.dumps(observation, indent=2))
    return observation


class Boundaries(unittest.TestCase):
    def test_1_watchdog_precedes_post_arm_write(self):
        result = exercise('order', 0)
        self.assertTrue(result['real_args_hash_json_reached'])
        self.assertEqual(result['error'], 'post-arm-write-frontier')
        self.assertEqual(result['events'], ['physical-watchdog-wrapper',
            'arm-transport-mocked', 'post-arm-write-frontier'])

    def test_2_elapsed_budget_is_not_reset_when_sent_to_pc(self):
        result = exercise('duration', 40)
        self.assertTrue(result['real_args_hash_json_reached'])
        self.assertEqual(result['error'], 'post-arm-write-frontier')
        self.assertEqual(len(result['transmitted_durations']), 1)
        self.assertGreater(result['transmitted_durations'][0], 0)
        self.assertLessEqual(result['transmitted_durations'][0], 285 - 40 - 8,
            'PC duration reset instead of consuming existing local budget and transport allowance')

    def test_3_post_launch_fsync_error_cleans_local_child(self):
        result = exercise('io-error', 0)
        self.assertTrue(result['real_args_hash_json_reached'])
        self.assertEqual(result['error'], 'controlled ledger fsync error')
        self.assertIn('launch-child-mocked', result['events'])
        self.assertIn('ledger-fsync-error-injected', result['events'])
        self.assertEqual(result['events'][-2:], ['terminate-local-child-mocked', 'wait-local-child-mocked'])
        self.assertFalse(result['local_mock_child_running'])


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--exporter', required=True)
    parser.add_argument('--exporter-sha256', required=True)
    parser.add_argument('--proof', required=True)
    parser.add_argument('--case', choices=['new-probes', 'all'], default='all')
    OPTIONS = parser.parse_args()
    cases = [] if OPTIONS.case == 'all' else [
        'Boundaries.test_2_elapsed_budget_is_not_reset_when_sent_to_pc',
        'Boundaries.test_3_post_launch_fsync_error_cleans_local_child']
    unittest.main(argv=[sys.argv[0]] + cases, verbosity=2)
