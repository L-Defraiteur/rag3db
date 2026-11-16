"""Behavioural fixtures for the three preparation review findings."""
import os
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
import strict_target_guard as guard


class TargetGuardTests(unittest.TestCase):
    def fixture(self, root):
        proc = root / 'proc' / '10'
        (proc / 'fd').mkdir(parents=True)
        (proc / 'stat').write_text('10 (controller) S 1' + ' 0' * 17 + ' 100')
        outside = root / 'controllers'
        outside.mkdir()
        (proc / 'exe').symlink_to(outside / 'python')
        (proc / 'cwd').symlink_to(outside)
        (proc / 'fd/255').symlink_to(outside / 'frozen-pilot.sh')
        return proc

    def test_legitimate_controller_needs_no_blanket_pid_exception(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            self.fixture(root)
            result = guard.scan(root / 'proc', [root / 'target', root / 'worktree'])
            self.assertEqual(result['found'], [])
            self.assertEqual(result['identities'][0]['start'], '100')

    def test_ancestor_or_controller_target_and_worktree_fds_are_rejected(self):
        for name in ['target', 'worktree']:
            with self.subTest(name=name), tempfile.TemporaryDirectory() as temp:
                root = Path(temp)
                proc = self.fixture(root)
                (proc / 'fd/9').symlink_to(root / name / 'artifact')
                result = guard.scan(root / 'proc', [root / 'target', root / 'worktree'])
                self.assertEqual(len(result['found']), 1)
                self.assertEqual(result['found'][0]['field'], 'fd/9')
                self.assertEqual(result['found'][0]['pid'], 10)

    def test_pid_reuse_is_not_an_exemption(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            self.fixture(root)
            values = [dict(pid=10, start='100', uid=os.getuid()), dict(pid=10, start='200', uid=os.getuid())]
            with patch.object(guard, 'identity', side_effect=values):
                result = guard.scan(root / 'proc', [root / 'target'])
            self.assertIn('PID identity changed', result['found'][0]['reason'])

    def test_identical_owner_text_does_not_admit_substituted_or_modified_target(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            target = root / 'target'
            target.mkdir()
            (target / '.arbre-du-target').write_text('same-owner')
            (target / 'elf').write_bytes(b'original')
            ledger = guard.snapshot(target)
            guard.require_provenance(target, ledger)
            (target / 'elf').write_bytes(b'changed')
            with self.assertRaises(ValueError):
                guard.require_provenance(target, ledger)
            target.rename(root / 'old')
            target.mkdir()
            (target / '.arbre-du-target').write_text('same-owner')
            (target / 'elf').write_bytes(b'original')
            with self.assertRaises(ValueError):
                guard.require_provenance(target, ledger)

    def test_only_terminated_clean_scope_before_original_deadline_is_green(self):
        authority = dict(etat='termine')
        scope = dict(verdict='VERT', code=0, restes=[], nettoye=True)
        self.assertEqual(guard.finish(authority, scope, 'VERT', 9, 10, True), 'VERT')
        for a, s, now, hashes in [
            (dict(etat='actif'), scope, 9, True), (authority, scope, 11, True),
            (authority, dict(scope, restes=[123]), 9, True),
            (authority, scope, 9, False),
        ]:
            with self.assertRaises(AssertionError):
                guard.finish(a, s, 'VERT', now, 10, hashes)


if __name__ == '__main__':
    unittest.main()
