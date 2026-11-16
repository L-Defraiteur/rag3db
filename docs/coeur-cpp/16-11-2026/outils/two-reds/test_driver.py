"""CPU-only counterexamples; no native build, poste, bwrap or real systemd unit."""
import ast
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import time
import unittest
from unittest.mock import Mock, patch

import processes
from receipt import CASES, PROFILE, admitted_cpu, inspect_case, verify_artifact_manifests

ENGINE = '8' * 40
MARKER = 'rag3db-build-' + ENGINE + PROFILE
HERE = Path(__file__).resolve().parent


def fixture(path, failed=False):
    path.mkdir(parents=True, exist_ok=True)
    (path / 'stdout.log').write_text('BUILD_ID=' + MARKER + '\n')
    suite, name = CASES['strings'].split('.')
    count = int(failed)
    detail = '<failure message="same query error">same query error</failure>' if failed else ''
    (path / 'result.xml').write_text(
        f'<testsuites tests="1" failures="{count}" disabled="0" errors="0">'
        f'<testsuite name="{suite}" tests="1" failures="{count}" disabled="0" errors="0" skipped="0">'
        f'<testcase classname="{suite}" name="{name}" status="run" result="completed">'
        f'{detail}</testcase></testsuite></testsuites>')


class ReceiptTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.path = Path(self.temp.name)
        fixture(self.path)

    def test_green_exact_case(self):
        self.assertEqual(inspect_case(self.path, 'strings', ENGINE, 0)['verdict'], 'VERT')

    def test_actual_repository_native_xml(self):
        import shutil
        fixture = HERE / 'fixtures/offsets-green'
        for name in ['result.xml', 'stdout.log']:
            shutil.copy2(fixture / name, self.path / name)
        with patch.dict(CASES, {'offsets_fixture':
                'DictionaryAppendException.OffsetAllocationFailureKeepsExistingStringsUnchanged'}):
            result = inspect_case(self.path, 'offsets_fixture',
                '888c81825e4cd6556261233f0e238af3b95375c9', 0)
        self.assertEqual(result['verdict'], 'VERT')

    def test_complete_red_never_green(self):
        fixture(self.path, True)
        self.assertEqual(inspect_case(self.path, 'strings', ENGINE, 1)['verdict'], 'ROUGE')
        with self.assertRaises(ValueError):
            inspect_case(self.path, 'strings', ENGINE, 0)

    def test_identity_absent_foreign_repeated_dirty(self):
        for text in ['', 'BUILD_ID=' + MARKER + '\nBUILD_ID=' + MARKER + '\n',
                     'BUILD_ID=' + MARKER.replace(ENGINE, '9' * 40) + '\n',
                     'BUILD_ID=' + MARKER.replace('-Release', '-dirty-abcdef-Release') + '\n']:
            with self.subTest(text=text):
                (self.path / 'stdout.log').write_text(text)
                with self.assertRaises(ValueError):
                    inspect_case(self.path, 'strings', ENGINE, 0)

    def test_missing_extra_skip_disabled_wrong_case(self):
        original = (self.path / 'result.xml').read_text()
        variants = [original.replace('status="run"', 'status="notrun"'),
                    original.replace('disabled="0"', 'disabled="1"'),
                    original.replace('skipped="0"', 'skipped="1"'),
                    original.replace('errors="0"', 'errors="1"'),
                    original.replace('tests="1"', 'tests="0"'),
                    original.replace('</testcase>', '<skipped/></testcase>'),
                    original.replace(CASES['strings'].split('.')[1], 'OtherCase')]
        for text in variants:
            (self.path / 'result.xml').write_text(text)
            with self.assertRaises(ValueError):
                inspect_case(self.path, 'strings', ENGINE, 0)

    def test_quota_effective(self):
        self.assertEqual(admitted_cpu('500000 100000', '8')['jobs'], 5)
        self.assertEqual(admitted_cpu('800000 100000', '3')['jobs'], 3)
        for quota in ['max 100000', '0 100000', '99999 100000']:
            with self.assertRaises(ValueError):
                admitted_cpu(quota, '8')

    def test_partial_red_artifacts_rehashed(self):
        (self.path / 'recipe.json').write_text(json.dumps({'variants': [
            {'name': 'candidate', 'source': 'a', 'engine': ENGINE},
            {'name': 'reference', 'source': 'b', 'engine': '9' * 40}]}))
        artifacts = self.path / 'raw/candidate/artifacts'
        artifacts.mkdir(parents=True)
        (artifacts / 'ELF').write_bytes(b'preserved bytes')
        digest = lambda path: hashlib.sha256(path.read_bytes()).hexdigest()
        (artifacts / 'manifest.json').write_text(json.dumps({'source': 'a', 'build_id': MARKER,
            'files': {'ELF': {'sha256': digest(artifacts / 'ELF')}}}))
        self.assertEqual(verify_artifact_manifests(self.path, digest, False), ['candidate'])
        with self.assertRaises(FileNotFoundError):
            verify_artifact_manifests(self.path, digest)
        (artifacts / 'ELF').write_bytes(b'changed')
        with self.assertRaises(AssertionError):
            verify_artifact_manifests(self.path, digest, False)

    def test_first_red_stops_actual_controller_loop(self):
        tree = ast.parse((HERE / 'controller.py').read_text())
        loop = next(node for node in ast.walk(tree) if isinstance(node, ast.For)
                    and isinstance(node.target, ast.Tuple)
                    and [x.id for x in node.target.elts] == ['case', 'case_name'])
        raw = self.path / 'raw'
        for case in CASES:
            (raw / 'candidate' / case).mkdir(parents=True)
        called = []
        def run(argv, label, cap, **kwargs):
            called.append(label)
            fixture(raw / label, True)
            return 1
        ledger = [{'variant': 'candidate', 'case': case, 'status': 'NON JOUE'} for case in CASES]
        namespace = {'CASES': CASES, 'name': 'candidate', 'ledger': ledger,
                     'save_ledger': lambda: None, 'test_env': {}, 'raw': raw,
                     'write': lambda p, v: p.write_text(json.dumps(v)), 'run': run,
                     'confined': lambda argv, testing, environment=None: argv, 'files': {'concurrence': 'not-executed'},
                     'inspect_case': inspect_case, 'variant': {'engine': ENGINE}}
        with self.assertRaisesRegex(RuntimeError, 'first native red'):
            exec(compile(ast.Module(body=[loop], type_ignores=[]), '<actual controller loop>', 'exec'), namespace)
        self.assertEqual(called, ['candidate/strings'])
        self.assertEqual([x['status'] for x in ledger], ['ROUGE', 'NON JOUE'])

    def test_actual_constructor_keeps_explicit_probe_environment(self):
        tree = ast.parse((HERE / 'controller.py').read_text())
        constructor = next(n for n in tree.body if isinstance(n, ast.FunctionDef) and n.name == 'confined')
        loop = next(n for n in ast.walk(tree) if isinstance(n, ast.For)
                    and isinstance(n.target, ast.Tuple)
                    and [x.id for x in n.target.elts] == ['case', 'case_name'])
        case = CASES['strings']
        raw = self.path / 'raw'
        (raw / 'candidate/strings').mkdir(parents=True)
        baseline = {'PATH': '/usr/bin:/bin', 'MAX_NUM_THREADS': '2'}
        called = []
        def run(argv, label, cap, **kwargs):
            called.append(label)
            self.assertEqual(kwargs['env']['RAG3DB_STRING_DIAGNOSTIC'], '1')
            self.assertIn('RAG3DB_STRING_DIAGNOSTIC=1', argv)
            # Every inner env-i field must be the same explicitly recorded test environment.
            start = argv.index('/usr/bin/env') + 2
            actual = dict(x.split('=', 1) for x in argv[start:argv.index('not-executed')])
            self.assertEqual(actual, kwargs['env'])
            return 0
        namespace = {'CASES': {'strings': case}, 'name': 'candidate',
                     'ledger': [{'variant': 'candidate', 'case': 'strings', 'status': 'NON JOUE'}],
                     'save_ledger': lambda: None, 'test_env': baseline, 'raw': raw,
                     'write': lambda p, v: p.write_text(json.dumps(v)), 'run': run,
                     'files': {'concurrence': 'not-executed'}, 'variant': {'engine': ENGINE},
                     'inspect_case': lambda *args: {'verdict': 'VERT'},
                     **{name: self.path / name for name in ['source', 'build', 'tmp', 'home', 'out']}}
        exec(compile(ast.Module(body=[constructor], type_ignores=[]), '<actual constructor>', 'exec'), namespace)
        with patch.dict(os.environ, {'RAG3DB_HOST_ONLY_SECRET': 'never-copy'}):
            exec(compile(ast.Module(body=[loop], type_ignores=[]), '<actual case loop>', 'exec'), namespace)
        self.assertEqual(called, ['candidate/strings'])
        self.assertNotIn('RAG3DB_STRING_DIAGNOSTIC', baseline)

    def test_preflight_refusal_no_unit(self):
        (self.path / 'recipe.json').write_text(json.dumps({
            'unit': 'cx-coeur-two-reds-01', 'scripts': {'missing.py': 'bad'}, 'variants': []}))
        result = subprocess.run([sys.executable, str(HERE / 'entry.py'), str(self.path)],
                                stdout=subprocess.PIPE, stderr=subprocess.STDOUT, timeout=5)
        self.assertEqual(result.returncode, 1)
        data = json.loads((self.path / 'raw/driver/quiescence.json').read_text())
        self.assertFalse(data['invoked'])
        self.assertEqual(data['first_refusal']['type'], 'FileNotFoundError')
        self.assertTrue((self.path / 'resume.log').read_text().endswith('ROUGE\n'))

    def test_dead_leader_group_still_signalled(self):
        process = Mock(pid=123, returncode=0)
        process.poll.return_value = 0
        with patch.object(processes, 'process', process), \
             patch.object(processes, 'group_exists', side_effect=[True, True, False, False]), \
             patch.object(processes.os, 'killpg') as kill:
            self.assertTrue(processes.finish_child())
            kill.assert_called_once_with(123, processes.signal.SIGTERM)

    def test_real_python_timeout_reaped(self):
        (self.path / 'cgroup.procs').write_text('')
        processes.raw = self.path
        processes.cgp = self.path
        processes.deadline = time.monotonic() + 15
        with self.assertRaises(TimeoutError):
            processes.run([sys.executable, '-c', 'import time;time.sleep(5)'], 'child', .1)
        result = json.loads((self.path / 'child/exit.json').read_text())
        self.assertTrue(result['timeout'])
        self.assertTrue(result['waited'])
        self.assertEqual(result['remaining_phase_pids'], [])
        self.assertFalse(processes.group_exists(result['pid']))


if __name__ == '__main__':
    unittest.main(verbosity=2)
