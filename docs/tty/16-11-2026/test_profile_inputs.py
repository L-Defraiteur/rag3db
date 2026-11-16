"""Git from an external cwd and real physical-file changes must be judged."""
import os
from pathlib import Path
import subprocess
import tempfile
import unittest
import strict_target_guard as guard


class ProfileInputTests(unittest.TestCase):
    def test_git_requires_real_worktree_and_exact_head_from_external_cwd(self):
        worktree = Path(__file__).resolve().parents[3]
        head = subprocess.check_output(['git', '-C', str(worktree), 'rev-parse', 'HEAD'], text=True).strip()
        previous = Path.cwd()
        try:
            with tempfile.TemporaryDirectory() as temp:
                os.chdir(temp)
                self.assertEqual(guard.require_source(worktree, head)['head'], head)
                with self.assertRaises(AssertionError):
                    guard.require_source(worktree, '0' * 40)
                with self.assertRaises(AssertionError):
                    guard.require_source(Path(temp), head)
        finally:
            os.chdir(previous)

    def test_final_hashes_read_actual_files_not_old_hash_text(self):
        with tempfile.TemporaryDirectory() as temp:
            path = Path(temp) / 'library.so'
            path.write_bytes(b'original library')
            expected = {str(path): guard.digest(path)}
            guard.require_hashes(expected)
            path.write_bytes(b'changed library')
            with self.assertRaises(AssertionError):
                guard.require_hashes(expected)


if __name__ == '__main__':
    unittest.main()
