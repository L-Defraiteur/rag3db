"""Vérifier le vrai constructeur de lancement sans acquérir de poste ni autorité."""
import importlib.util
import json
import os
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch

spec = importlib.util.spec_from_file_location('rust_followup', Path(__file__).with_name('pilot.py'))
pilot = importlib.util.module_from_spec(spec)
spec.loader.exec_module(pilot)


class FollowupLaunchTests(unittest.TestCase):
    def test_actual_constructor_runs_common_from_proofs(self):
        original = Path.cwd()
        self.addCleanup(os.chdir, original)
        with tempfile.TemporaryDirectory() as temporary:
            proofs = Path(temporary).resolve()
            (proofs / 'prepared.json').write_text(json.dumps({'worktree': str(original), 'budget': 900}))
            class StopBeforeSupervisor(Exception):
                pass
            def check(root, name, command, cwd, env):
                self.assertEqual(Path.cwd(), proofs)
                self.assertEqual(cwd, proofs)
                self.assertEqual(command[2:5], ['lancer', '--delai', '900'])
                self.assertEqual(command[-3:], ['inside', '--proofs', str(proofs)])
                self.assertEqual(env['POSTE_MEM_MAX'], '32G')
                raise StopBeforeSupervisor()
            with patch.object(pilot, 'verify'), patch.object(pilot.base, 'record', side_effect=check), \
                    patch.object(sys, 'argv', ['pilot.py', 'run', '--proofs', str(proofs), '--allocation', 'unit-test-not-authority']):
                with self.assertRaises(StopBeforeSupervisor):
                    pilot.main()
            os.chdir(original)


if __name__ == '__main__':
    unittest.main()
