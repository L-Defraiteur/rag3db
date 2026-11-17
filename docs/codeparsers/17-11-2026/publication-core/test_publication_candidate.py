"""Exact regression selection for the corrected source candidate. NOT RUN.

36 inherited +3 plan +8 windows +10 grouped guards +1 causal RPC =58.
No backend, replacement engine or additional executable case lives here.
"""
import importlib.util
from pathlib import Path
import sys
import unittest

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent / 'rust-probe-relay'))
SOURCES = (
    (HERE.parent / 'rust-probe-relay/test_relay.py', 36),
    (HERE / 'test_publication_counterexamples.py', 3),
    (HERE / 'test_publication_windows.py', 8),
    (HERE / 'test_publication_guards_grouped.py', 10),
    (HERE / 'test_publication_rpc_first_cause.py', 1),
)


def load_tests(loader, tests, pattern):
    suite = unittest.TestSuite()
    for index, (path, expected) in enumerate(SOURCES):
        spec = importlib.util.spec_from_file_location('publication_candidate_suite_' + str(index), path)
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        selected = loader.loadTestsFromModule(module)
        assert selected.countTestCases() == expected, 'unexpected regression selection: ' + str(path)
        suite.addTests(selected)
    assert suite.countTestCases() == 58
    return suite


if __name__ == '__main__':
    unittest.main()
