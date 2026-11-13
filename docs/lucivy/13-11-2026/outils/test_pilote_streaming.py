"""CPU-only preparation tests: extract driver functions, fake every subprocess."""
import ast
import io
import json
import os
from pathlib import Path
import re
import tempfile
import time
import types
import unittest

SOURCE = Path(__file__).with_name('qualifier-barriere-complete.py')

class Child:
    pid = 424242
    def __init__(self, output, code=0):
        self.stdout = io.StringIO(output)
        self.code = code
    def wait(self, timeout=None): return self.code

class StreamingTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.out = Path(self.tmp.name)
        self.children = []
        self.stops = []
        tree = ast.parse(SOURCE.read_text())
        selected = [node for node in tree.body if isinstance(node, ast.FunctionDef)
                    and node.name in ['first_red', 'run']]
        selected += [node for node in tree.body if isinstance(node, ast.Assign)
                     and any(isinstance(t, ast.Name) and t.id == 'failed' for t in node.targets)]
        def popen(*args, **kwargs): return self.children.pop(0)
        def stop(child, start):
            self.assertTrue((self.out / 'premier-rouge.json').exists())
            self.stops.append((child.pid, start))
            child.code = -15
        self.ns = dict(out=self.out, json=json, os=os, time=time, re=re,
                       budget=lambda: None, pid_start=lambda pid: 'fake-start', stop_owned=stop,
                       subprocess=types.SimpleNamespace(Popen=popen, PIPE=object(), STDOUT=object()))
        exec(compile(ast.Module(body=selected, type_ignores=[]), str(SOURCE), 'exec'), self.ns)
    def tearDown(self): self.tmp.cleanup()
    def test_expected_panic_and_should_panic_are_not_red(self):
        self.children.append(Child("thread 'expected' panicked at fixture\ntest a - should panic ... ok\n"))
        self.ns['run'](['fake-cargo'], 'ok.log')
        self.assertEqual(self.stops, [])
        self.assertFalse((self.out / 'premier-rouge.json').exists())
    def test_failed_line_latches_before_stop_and_preserves_first_red(self):
        self.children.append(Child('test a ... FAILED\nthis must not be consumed\n'))
        with self.assertRaises(SystemExit): self.ns['run'](['fake-cargo'], 'failed.log')
        before = (self.out / 'premier-rouge.json').read_bytes()
        self.ns['first_red']('later cleanup')
        self.assertEqual(before, (self.out / 'premier-rouge.json').read_bytes())
        self.assertEqual(self.stops, [(424242, 'fake-start')])
        self.assertNotIn('must not', (self.out / 'failed.log').read_text())
    def test_split_standalone_failed_is_red(self):
        self.children.append(Child('test a ... user output\nFAILED\n'))
        with self.assertRaises(SystemExit): self.ns['run'](['fake'], 'split.log')
        self.assertEqual(len(self.stops), 1)
    def test_nonzero_compile_is_red_without_test_failure(self):
        self.children.append(Child('error E0046\n', 101))
        with self.assertRaises(SystemExit): self.ns['run'](['fake-build'], 'compile.log')
        marker = json.loads((self.out / 'premier-rouge.json').read_text())
        self.assertEqual(marker['code'], 101)
        self.assertEqual(self.stops, [])

if __name__ == '__main__': unittest.main(failfast=True)
