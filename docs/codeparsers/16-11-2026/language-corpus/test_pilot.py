"""Le vrai constructeur du lancement quitte la source, sans lancer de superviseur."""
import importlib.util
import json
import os
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch

spec = importlib.util.spec_from_file_location('corpus_pilot', Path(__file__).with_name('pilot.py'))
pilot = importlib.util.module_from_spec(spec)
spec.loader.exec_module(pilot)


class LaunchContextTests(unittest.TestCase):
    def test_actual_run_constructor_leaves_source_before_common(self):
        original = Path.cwd()
        self.addCleanup(os.chdir, original)
        with tempfile.TemporaryDirectory() as temporary:
            proofs = Path(temporary).resolve()
            config = {'worktree': str(original), 'budget': 1800}
            (proofs / 'prepared.json').write_text(json.dumps(config))
            class StopBeforeSupervisor(Exception):
                pass
            def inspect_launch(root, name, command, cwd, env):
                self.assertEqual(Path.cwd(), proofs)
                self.assertEqual(cwd, proofs)
                self.assertEqual(name, 'common')
                self.assertEqual(command[2], 'lancer')
                self.assertIn('--delai', command)
                raise StopBeforeSupervisor()
            with patch.object(pilot, 'verify'), patch.object(pilot, 'record', side_effect=inspect_launch), \
                    patch.object(sys, 'argv', ['pilot.py', 'run', '--proofs', str(proofs), '--allocation', 'unit-test-not-authority']):
                with self.assertRaises(StopBeforeSupervisor):
                    pilot.main()
            os.chdir(original)


if __name__ == '__main__':
    unittest.main()
