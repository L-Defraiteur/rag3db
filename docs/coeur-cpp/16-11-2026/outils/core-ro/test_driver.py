"""CPU-only supervision checks; never invoke GDB, zstd or a native test."""
import ast
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import time
import unittest
from unittest.mock import Mock, patch

import processes
from receipt import admitted_cpu, gdb_command, verify_artifact_manifests

HERE = Path(__file__).resolve().parent


class Tests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.path = Path(self.temp.name)

    def test_gdb_commands_only_existing_core(self):
        command = gdb_command('/usr/bin/gdb', '/private/ELF', '/private/core', '/private/empty')
        self.assertIn('--core=/private/core', command)
        self.assertIn('--se=/private/ELF', command)
        self.assertTrue({'--nx', '--nh', '--batch', '--quiet'}.issubset(command))
        initial = [command[i + 1] for i, x in enumerate(command) if x == '-iex']
        self.assertIn('set auto-load off', initial)
        self.assertIn('set auto-solib-add off', initial)
        self.assertIn('set debuginfod enabled off', initial)
        self.assertIn('set sysroot /private/empty', initial)
        self.assertIn('set solib-search-path /private/empty', initial)
        commands = [command[i + 1] for i, x in enumerate(command) if x == '-ex']
        self.assertEqual(commands, ['info files', 'info sharedlibrary', 'thread apply all bt 12',
                                    'info registers rip rsp', 'quit'])

    def test_actual_decompression_argv_matches_captured_usage(self):
        usage = (HERE / 'fixtures/zstd-output-refusal.stderr.log').read_text()
        self.assertIn('-o OUTPUT', usage)
        self.assertIn('Incorrect parameter: --output=', usage)
        tree = ast.parse((HERE / 'controller.py').read_text())
        call = next(node for node in ast.walk(tree) if isinstance(node, ast.Call)
                    and isinstance(node.func, ast.Name) and node.func.id == 'run'
                    and isinstance(node.args[0], ast.List)
                    and "inputs['zstd']" in ast.unparse(node.args[0]))
        commands = []
        namespace = {'inputs': {'zstd': Path('/usr/bin/zstd'),
                                'compressed_core': Path('/private/input.zst')},
                     'core': Path('/private/core'), 'phase': 'decompress', 'env': {},
                     'run': lambda argv, *args: commands.append(argv)}
        exec(compile(ast.Expression(body=call), '<actual decompress constructor>', 'eval'), namespace)
        self.assertEqual(commands, [['/usr/bin/zstd', '--decompress', '--sparse', '--no-progress',
                                     '-o', '/private/core', '/private/input.zst']])

    def test_effective_cpu_not_guessed(self):
        self.assertEqual(admitted_cpu('500000 100000', '8')['jobs'], 5)
        for value in ['max 100000', '50000 100000', '0 0']:
            with self.assertRaises(ValueError):
                admitted_cpu(value, '8')

    def test_input_rehash_even_after_red(self):
        input_file = self.path / 'input'
        input_file.write_bytes(b'original core')
        sha = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
        (self.path / 'recipe.json').write_text(json.dumps({'inputs': [
            {'name': 'core', 'path': str(input_file), 'sha256': sha(input_file)}]}))
        self.assertEqual(verify_artifact_manifests(self.path, sha, False), [])
        input_file.write_bytes(b'changed core')
        with self.assertRaises(AssertionError):
            verify_artifact_manifests(self.path, sha, False)

    def test_missing_final_evidence_never_green(self):
        (self.path / 'recipe.json').write_text(json.dumps({'inputs': []}))
        with self.assertRaises(FileNotFoundError):
            verify_artifact_manifests(self.path, lambda p: '', True)

    def test_stderr_preserved_separately(self):
        (self.path / 'cgroup.procs').write_text('')
        processes.cgp = processes.raw = self.path
        processes.deadline = time.monotonic() + 15
        processes.run([sys.executable, '-c',
                       'import sys;print("stdout only");print("stderr only",file=sys.stderr)'],
                      'python', 1, env={'PATH': '/usr/bin:/bin'})
        self.assertEqual((self.path / 'python/stdout.log').read_text(), 'stdout only\n')
        self.assertEqual((self.path / 'python/stderr.log').read_text(), 'stderr only\n')

    def test_real_python_timeout_stopped(self):
        (self.path / 'cgroup.procs').write_text('')
        processes.cgp = processes.raw = self.path
        processes.deadline = time.monotonic() + 15
        with self.assertRaises(TimeoutError):
            processes.run([sys.executable, '-c', 'import time;time.sleep(5)'], 'python', .05)
        record = json.loads((self.path / 'python/exit.json').read_text())
        self.assertTrue(record['timeout'] and record['waited'])
        self.assertFalse(processes.group_exists(record['pid']))

    def test_dead_leader_group_is_still_stopped(self):
        process = Mock(pid=123, returncode=0)
        process.poll.return_value = 0
        with patch.object(processes, 'process', process), \
             patch.object(processes, 'group_exists', side_effect=[True, True, False, False]), \
             patch.object(processes.os, 'killpg') as kill:
            self.assertTrue(processes.finish_child())
            kill.assert_called_once_with(123, processes.signal.SIGTERM)

    def test_preflight_error_does_not_create_unit(self):
        (self.path / 'recipe.json').write_text(json.dumps({
            'unit': 'cx-coeur-core-ro-02', 'scripts': {'absent.py': 'bad'}, 'inputs': []}))
        result = subprocess.run([sys.executable, str(HERE / 'entry.py'), str(self.path)],
                                stdout=subprocess.PIPE, stderr=subprocess.STDOUT, timeout=5)
        self.assertEqual(result.returncode, 1)
        record = json.loads((self.path / 'raw/driver/quiescence.json').read_text())
        self.assertFalse(record['invoked'])
        self.assertEqual(record['first_refusal']['type'], 'FileNotFoundError')
        self.assertTrue((self.path / 'resume.log').read_text().endswith('ROUGE\n'))


if __name__ == '__main__':
    unittest.main(verbosity=2)
