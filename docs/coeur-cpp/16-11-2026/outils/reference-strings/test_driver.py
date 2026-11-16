"""CPU-only checks of the actual one-case constructor and strict native receipts."""
import ast
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import time
import unittest
from unittest.mock import patch

import processes
from receipt import CASES, PROFILE, admitted_cpu, inspect_case, verify_artifact_manifests

HERE = Path(__file__).resolve().parent
ENGINE = '59ac9c86bc2b3529b590a7462ab7051e3ad26b8a'


def fixture(path, failed=False):
    path.mkdir(parents=True, exist_ok=True)
    suite, name = CASES['strings'].split('.')
    (path / 'stdout.log').write_text('[ RUN      ] ' + CASES['strings'] + '\n')
    count = int(failed)
    detail = '<failure message="value mismatch">value mismatch</failure>' if failed else ''
    (path / 'result.xml').write_text(
        f'<testsuites tests="1" failures="{count}" disabled="0" errors="0">'
        f'<testsuite name="{suite}" tests="1" failures="{count}" disabled="0" errors="0" skipped="0">'
        f'<testcase classname="{suite}" name="{name}" status="run" result="completed">'
        f'{detail}</testcase></testsuite></testsuites>')


class Tests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.path = Path(self.temp.name)
        fixture(self.path)

    def test_no_runtime_identity_is_invented(self):
        result = inspect_case(self.path, 'strings', ENGINE, 0)
        self.assertEqual(result['verdict'], 'VERT')
        self.assertFalse(result['native_build_id_emitted'])
        self.assertIn('expected_header_elf_build_id', result)

    def test_complete_red_preserved(self):
        fixture(self.path, True)
        result = inspect_case(self.path, 'strings', ENGINE, 1)
        self.assertEqual(result['verdict'], 'ROUGE')
        with self.assertRaises(ValueError):
            inspect_case(self.path, 'strings', ENGINE, 0)

    def test_unexpected_runtime_identity_rejected(self):
        with (self.path / 'stdout.log').open('a') as stream:
            stream.write('BUILD_ID=rag3db-build-' + ENGINE + PROFILE + '\n')
        with self.assertRaises(ValueError):
            inspect_case(self.path, 'strings', ENGINE, 0)

    def test_missing_case_skip_disabled_invalid_xml(self):
        original = (self.path / 'result.xml').read_text()
        for modified in [original.replace('tests="1"', 'tests="0"'),
                         original.replace('skipped="0"', 'skipped="1"'),
                         original.replace('disabled="0"', 'disabled="1"'),
                         original.replace('status="run"', 'status="notrun"'),
                         original.replace('errors="0"', 'errors="1"'),
                         original.replace(' skipped="0"', ''),
                         original.replace('</testcase>', '<skipped/></testcase>')]:
            (self.path / 'result.xml').write_text(modified)
            with self.assertRaises(ValueError):
                inspect_case(self.path, 'strings', ENGINE, 0)

    def test_actual_argv_absent_probe_even_with_host_flag(self):
        tree = ast.parse((HERE / 'controller.py').read_text())
        constructor = next(n for n in tree.body if isinstance(n, ast.FunctionDef) and n.name == 'confined')
        loop = next(n for n in ast.walk(tree) if isinstance(n, ast.For)
                    and isinstance(n.target, ast.Tuple)
                    and [x.id for x in n.target.elts] == ['case', 'case_name'])
        raw = self.path / 'raw'
        phase = raw / 'reference/strings'
        phase.mkdir(parents=True)
        calls = []
        baseline = {'PATH': '/usr/bin:/bin', 'LANG': 'C', 'LC_ALL': 'C', 'TZ': 'UTC',
                    'MAX_NUM_THREADS': '2', 'OMP_NUM_THREADS': '5', 'OPENBLAS_NUM_THREADS': '5'}
        def run(argv, label, cap, **kwargs):
            calls.append(argv)
            start = argv.index('/usr/bin/env') + 2
            actual = dict(x.split('=', 1) for x in argv[start:argv.index('native-not-executed')])
            self.assertEqual(actual, kwargs['env'])
            self.assertNotIn('RAG3DB_STRING_DIAGNOSTIC', actual)
            self.assertEqual(cap, 60)
            self.assertIn('--gtest_filter=' + CASES['strings'], argv)
            self.assertIn('--gtest_fail_fast', argv)
            fixture(raw / label)
            return 0
        ledger = [{'variant': 'reference', 'case': 'strings', 'status': 'NON JOUE'}]
        namespace = {'CASES': CASES, 'name': 'reference', 'ledger': ledger, 'save_ledger': lambda: None,
                     'test_env': baseline, 'raw': raw, 'json': json,
                     'write': lambda p, v: p.write_text(json.dumps(v)), 'run': run,
                     'files': {'concurrence': 'native-not-executed'}, 'inspect_case': inspect_case,
                     'variant': {'engine': ENGINE},
                     **{n: self.path / n for n in ['source', 'build', 'tmp', 'home', 'out']}}
        exec(compile(ast.Module(body=[constructor], type_ignores=[]), '<actual constructor>', 'exec'), namespace)
        with patch.dict(os.environ, {'RAG3DB_STRING_DIAGNOSTIC': '1'}):
            exec(compile(ast.Module(body=[loop], type_ignores=[]), '<actual one-case loop>', 'exec'), namespace)
        self.assertEqual(len(calls), 1)
        self.assertEqual(ledger[0]['status'], 'VERT')
        self.assertNotIn('RAG3DB_STRING_DIAGNOSTIC', baseline)

    def test_actual_loop_first_red_no_retry(self):
        tree = ast.parse((HERE / 'controller.py').read_text())
        loop = next(n for n in ast.walk(tree) if isinstance(n, ast.For)
                    and isinstance(n.target, ast.Tuple)
                    and [x.id for x in n.target.elts] == ['case', 'case_name'])
        raw = self.path / 'raw'
        (raw / 'reference/strings').mkdir(parents=True)
        calls = []
        def run(argv, label, cap, **kwargs):
            calls.append(label)
            fixture(raw / label, True)
            return 1
        namespace = {'CASES': CASES, 'name': 'reference',
                     'ledger': [{'variant': 'reference', 'case': 'strings', 'status': 'NON JOUE'}],
                     'save_ledger': lambda: None, 'test_env': {}, 'raw': raw, 'json': json,
                     'write': lambda p, v: p.write_text(json.dumps(v)), 'run': run,
                     'confined': lambda argv, *a, **k: argv,
                     'files': {'concurrence': 'not-executed'}, 'inspect_case': inspect_case,
                     'variant': {'engine': ENGINE}}
        with self.assertRaisesRegex(RuntimeError, 'first native red'):
            exec(compile(ast.Module(body=[loop], type_ignores=[]), '<actual one-case loop>', 'exec'), namespace)
        self.assertEqual(calls, ['reference/strings'])
        self.assertEqual(namespace['ledger'][0]['status'], 'ROUGE')

    def test_actual_loop_crash_classified_without_signal_assumption(self):
        tree = ast.parse((HERE / 'controller.py').read_text())
        loop = next(n for n in ast.walk(tree) if isinstance(n, ast.For)
                    and isinstance(n.target, ast.Tuple)
                    and [x.id for x in n.target.elts] == ['case', 'case_name'])
        raw = self.path / 'raw'
        target = raw / 'reference/strings'
        target.mkdir(parents=True)
        def run(argv, label, cap, **kwargs):
            (target / 'exit.json').write_text(json.dumps({'code': 139, 'timeout': False}))
            raise RuntimeError('wrapper139')
        namespace = {'CASES': CASES, 'name': 'reference',
                     'ledger': [{'variant': 'reference', 'case': 'strings', 'status': 'NON JOUE'}],
                     'save_ledger': lambda: None, 'test_env': {}, 'raw': raw, 'json': json,
                     'write': lambda p, v: p.write_text(json.dumps(v)), 'run': run,
                     'confined': lambda argv, *a, **k: argv,
                     'files': {'concurrence': 'not-executed'}, 'inspect_case': inspect_case,
                     'variant': {'engine': ENGINE}}
        with self.assertRaisesRegex(RuntimeError, 'wrapper139'):
            exec(compile(ast.Module(body=[loop], type_ignores=[]), '<actual one-case loop>', 'exec'), namespace)
        detail = json.loads((target / 'classification.json').read_text())
        self.assertFalse(detail['native_signal_confirmed'])
        self.assertTrue(detail['signal_requires_existing_core_or_journal'])

    def test_effective_quota(self):
        self.assertEqual(admitted_cpu('500000 100000', '5')['jobs'], 5)
        for text in ['max 100000', '0 100000']:
            with self.assertRaises(ValueError):
                admitted_cpu(text, '5')

    def test_owned_python_timeout_stopped(self):
        (self.path / 'cgroup.procs').write_text('')
        processes.cgp = processes.raw = self.path
        processes.deadline = time.monotonic() + 15
        with self.assertRaises(TimeoutError):
            processes.run([sys.executable, '-c', 'import time;time.sleep(5)'], 'child', .05)
        detail = json.loads((self.path / 'child/exit.json').read_text())
        self.assertTrue(detail['timeout'] and detail['waited'])
        self.assertFalse(processes.group_exists(detail['pid']))

    def test_preflight_missing_script_no_unit(self):
        (self.path / 'recipe.json').write_text(json.dumps({
            'unit': 'cx-coeur-reference-strings-01', 'scripts': {'absent.py': 'bad'}, 'variants': []}))
        result = subprocess.run([sys.executable, str(HERE / 'entry.py'), str(self.path)],
                                stdout=subprocess.PIPE, stderr=subprocess.STDOUT, timeout=5)
        self.assertEqual(result.returncode, 1)
        detail = json.loads((self.path / 'raw/driver/quiescence.json').read_text())
        self.assertFalse(detail['invoked'])
        self.assertEqual(detail['first_refusal']['type'], 'FileNotFoundError')
        self.assertTrue((self.path / 'resume.log').read_text().endswith('ROUGE\n'))


if __name__ == '__main__':
    unittest.main(verbosity=2)
