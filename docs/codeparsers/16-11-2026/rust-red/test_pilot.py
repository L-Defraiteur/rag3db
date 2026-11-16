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


class ScopeReaderTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix='rust-red-reader-')
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.scope = self.root / 'scope' / 'validation-fixture'
        self.scope.mkdir(parents=True)
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


if __name__ == '__main__':
    unittest.main()
