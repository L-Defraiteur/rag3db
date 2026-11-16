#!/usr/bin/env python3
"""Hermetic reader tests: fixture files only, no supervisor or Cargo invocation."""
import importlib.util
import json
import os
from pathlib import Path
import tempfile
import unittest

spec = importlib.util.spec_from_file_location('rust_red_pilot', Path(__file__).with_name('pilot.py'))
pilot = importlib.util.module_from_spec(spec)
spec.loader.exec_module(pilot)
common_path = Path(__file__).resolve().parents[4] / 'extension/rag3weaver/scripts/confiner_validation.py'
common_spec = importlib.util.spec_from_file_location('common_fixture_layout', common_path)
common = importlib.util.module_from_spec(common_spec)
common_spec.loader.exec_module(common)


class ScopeReaderTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix='rust-red-reader-')
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.scope = common.repertoire_preuves(str(self.root / 'scope'))
        self.verdict = dict(verdict='ROUGE', code=101, erreur=None, restes=[],
                            prive=str(self.root / 'removed-private'), nettoye=True)
        self.authority = dict(pid=os.getpid(), etat='termine', echeance=123.0, cgroup='/fixture-only')
        self.write(self.scope / 'verdict.json', self.verdict)
        self.write(self.scope / 'autorite.json', self.authority)
        self.write(self.root / 'authority-path.json', dict(path=str(self.scope / 'autorite.json'), deadline=123.0))
        self.write(self.root / 'red-pair.result.json', dict(rc=101))
        self.write(self.root / 'red-pair.command.json', ['bash', './run_e2e.sh', '--test', 'e2e_usages', 'on_the_same_line'])
        summary = '\n'.join('    e2e_usages::' + name for name in pilot.NAMES)
        self.stdout = 'failures:\n' + summary + '\ntest result: FAILED. 0 passed; 2 failed; 0 ignored; 40 filtered out;\n'
        self.stderr = ('assertion failed: the project call must remain visible\n  left: 0\n right: 1\n') * 2
        (self.root / 'red-pair.stdout').write_text(self.stdout)
        (self.root / 'red-pair.stderr').write_text(self.stderr)

    def write(self, path, value):
        path.write_text(json.dumps(value))

    def test_reads_the_real_common_directory_shape(self):
        scope = pilot.read_scope_verdict(self.root, 1)
        self.assertEqual(scope.get('code'), 101)
        self.assertEqual(scope.get('verdict'), 'ROUGE')


    def test_no_verdict_is_refused(self):
        (self.scope / 'verdict.json').unlink()
        with self.assertRaises(ValueError):
            pilot.read_scope_verdict(self.root, 1)

    def test_two_verdicts_are_refused(self):
        other = self.root / 'scope' / 'validation-other'
        other.mkdir()
        self.write(other / 'verdict.json', self.verdict)
        with self.assertRaises(ValueError):
            pilot.read_scope_verdict(self.root, 1)

    def test_active_or_foreign_authority_is_refused(self):
        self.authority['etat'] = 'actif'
        self.write(self.scope / 'autorite.json', self.authority)
        with self.assertRaises(ValueError):
            pilot.read_scope_verdict(self.root, 1)
        self.authority['etat'] = 'termine'
        self.write(self.scope / 'autorite.json', self.authority)
        self.write(self.root / 'authority-path.json', dict(path='/another/invocation', deadline=123.0))
        with self.assertRaises(ValueError):
            pilot.read_scope_verdict(self.root, 1)

    def test_wrong_codes_cleanup_or_remaining_descendants_are_refused(self):
        for changes in [dict(code=0), dict(code=1), dict(restes=[42]), dict(nettoye=False),
                        dict(erreur='deadline'), dict(verdict='VERT')]:
            with self.subTest(changes=changes):
                self.write(self.scope / 'verdict.json', dict(self.verdict, **changes))
                with self.assertRaises(ValueError):
                    pilot.read_scope_verdict(self.root, 1)
        self.write(self.scope / 'verdict.json', self.verdict)
        with self.assertRaises(ValueError):
            pilot.read_scope_verdict(self.root, 0)
        self.write(self.root / 'red-pair.result.json', dict(rc=1))
        with self.assertRaises(ValueError):
            pilot.read_scope_verdict(self.root, 1)

    def test_live_private_scope_is_refused(self):
        Path(self.verdict['prive']).mkdir()
        with self.assertRaises(ValueError):
            pilot.read_scope_verdict(self.root, 1)

    def test_wrong_or_ignored_tests_and_wrong_failure_are_refused(self):
        for stdout, stderr in [
            (self.stdout.replace('2 failed; 0 ignored', '1 failed; 1 ignored'), self.stderr),
            (self.stdout.replace(pilot.NAMES[0], 'another_test'), self.stderr),
            (self.stdout, self.stderr.replace('the project call must remain visible', 'unrelated failure'))]:
            with self.subTest(stdout=stdout):
                (self.root / 'red-pair.stdout').write_text(stdout)
                (self.root / 'red-pair.stderr').write_text(stderr)
                with self.assertRaises(ValueError):
                    pilot.read_scope_verdict(self.root, 1)

    def test_linked_verdict_is_refused(self):
        path = self.scope / 'verdict.json'
        other = self.root / 'linked-target.json'
        path.rename(other)
        path.symlink_to(other)
        with self.assertRaises(ValueError):
            pilot.read_scope_verdict(self.root, 1)


if __name__ == '__main__':
    unittest.main()
