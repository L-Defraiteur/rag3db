"""Synthetic metadata only. No real proc scan, subprocess, target or Cargo."""
import ast
from pathlib import Path
import unittest

source = Path(__file__).with_name('qualify-indexed-false-green.py')
tree = ast.parse(source.read_text())
functions = [node for node in tree.body if isinstance(node, ast.FunctionDef) and node.name == 'classify_scan']
namespace = {}
exec(compile(ast.Module(body=functions, type_ignores=[]), str(source), 'exec'), namespace)
classify = namespace['classify_scan']


def record(**changes):
    value = dict(pid=1247, uid=1000, start='1234', cgroup='/baseline', present=True,
                 owned=False, attributed=False, errors=[])
    value.update(changes)
    return value


class QuiescenceContract(unittest.TestCase):
    def test_known_owned_activity_blocks(self):
        result = classify([record(owned=True)], [])
        self.assertFalse(result['admitted'])
        self.assertEqual(result['blocked'][0]['reason'], 'known_activity')

    def test_new_attributed_unknown_blocks_even_if_identity_was_baseline(self):
        opaque = record(errors=[dict(field='exe', errno=13)])
        result = classify([dict(opaque, attributed=True)], [opaque])
        self.assertFalse(result['admitted'])
        self.assertEqual(result['blocked'][0]['reason'], 'attributed_unknown')

    def test_opaque_unattributed_baseline_is_retained_as_unknown(self):
        opaque = record(errors=[dict(field='exe', errno=13)])
        result = classify([opaque], [opaque])
        self.assertTrue(result['admitted'])
        self.assertEqual(result['unknown'], [opaque])
        self.assertFalse(result['universal_quiescence_claimed'])


if __name__ == '__main__':
    unittest.main(failfast=True)
