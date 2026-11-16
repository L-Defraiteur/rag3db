"""Discriminants du scan de target, sans lecture des processus réels."""
import unittest
from pathlib import Path
from relocate_target import scan

class TargetScanTests(unittest.TestCase):
    def test_ancestor_target_fd_is_not_invisible(self):
        import inspect
        code = inspect.getsource(scan)
        self.assertNotIn("int(proc.name) in ancestors", code,
                         "an ancestor must be inspected before any scoped exception")

if __name__ == '__main__':
    unittest.main()
