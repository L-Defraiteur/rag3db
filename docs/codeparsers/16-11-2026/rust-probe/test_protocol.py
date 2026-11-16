"""Contrats CPU hermétiques ; aucun Cargo, ELF, bus, /proc réel ou reçu réel."""
import importlib.util
import json
import os
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

HERE = Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location('probe_pilot_test', HERE / 'pilot.py')
pilot = importlib.util.module_from_spec(spec)
spec.loader.exec_module(pilot)
guard = pilot.physical


class ProtocolTests(unittest.TestCase):
    def artifact(self):
        return {'reason': 'compiler-artifact', 'target': {'name': 'sonde_marque_nom', 'kind': ['test'],
            'src_path': '/private/tests/sonde_marque_nom.rs'}, 'profile': {'test': True},
            'features': ['code', 'rag3db-native'], 'executable': '/target/probe'}

    def test_artifact_is_unique_source_and_feature_bound(self):
        row = self.artifact()
        source = row['target']['src_path']
        self.assertEqual(pilot.choose_artifact(json.dumps(row), source), row)
        for rows in [[], [row, row]]:
            with self.assertRaises(AssertionError):
                pilot.choose_artifact('\n'.join(map(json.dumps, rows)), source)
        with self.assertRaises(AssertionError):
            pilot.choose_artifact(json.dumps(row), source + '.other')
        row['features'].append('postgres')
        with self.assertRaises(AssertionError):
            pilot.choose_artifact(json.dumps(row), source)

    def test_outer_covers_both_phases_with_explicit_1798_plus_2(self):
        pair = {'control': '/proofs/control', 'timeout': '/usr/bin/timeout', 'proofs': '/proofs'}
        argv = pilot.outer_command(pair, 'candidate')
        self.assertEqual(argv[:6], ['/proofs/control/poste', 'lourd', '/usr/bin/timeout',
            '--signal=TERM', '--kill-after=2s', '1798s'])
        self.assertEqual(argv[-2:], ['--phase', 'candidate'])
        self.assertEqual(2 * (1798 + 2), 3600)

    def test_expired_publication_never_keeps_phase_green(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'result.json'
            with patch.object(guard.time, 'monotonic', side_effect=[10, 11]):
                result = guard.publish_result(path, {'verdict': 'VERT', 'phase_completed': True}, 10)
            self.assertEqual(result['verdict'], 'ROUGE')
            self.assertFalse(json.loads(path.read_text())['phase_completed'])

    def test_fsync_crossing_deadline_rewrites_red(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'result.json'
            with patch.object(guard.time, 'monotonic', side_effect=[9, 10]):
                result = guard.publish_result(path, {'verdict': 'VERT', 'phase_completed': True}, 10)
            self.assertEqual(result['verdict'], 'ROUGE')
            self.assertFalse(json.loads(path.read_text())['phase_completed'])

    def make_proc(self, root, argv=(), cgroup='/scope'):
        proc = root / '101'
        proc.mkdir()
        fields = ['S', '1'] + ['0'] * 17 + ['1234']
        (proc / 'stat').write_text('101 (fake process) ' + ' '.join(fields))
        (proc / 'exe').symlink_to('/usr/bin/python3')
        (proc / 'cwd').symlink_to('/outside')
        (proc / 'cmdline').write_bytes(b'\0'.join(os.fsencode(v) for v in argv) + b'\0')
        (proc / 'cgroup').write_text('0::' + cgroup)
        (proc / 'fd').mkdir()
        return proc

    def test_argv_and_cgroup_attribute_without_exe_or_cwd(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            self.make_proc(root, ['python', '--manifest-path=/protected/Cargo.toml'])
            result = guard.scan(root, ['/protected'], ['/scope'])
            evidence = {row['field'] for row in result['found'][0]['evidence']}
            self.assertEqual(evidence, {'argv', 'cgroup'})

    def test_no_ancestor_fd_or_suffix_exemption(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            proc = self.make_proc(root, ['python'])
            (proc / 'fd/7').symlink_to('/protected/scripts/poste')
            control = guard.process_record(101, root)
            result = guard.scan(root, ['/protected'], ['/scope'], [control])
            self.assertEqual(result['found'][0]['evidence'][0]['field'], 'fd/7')

    def test_control_cgroup_exception_requires_exact_frozen_identity(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            proc = self.make_proc(root, ['python', '/outside/controller.py'])
            control = guard.process_record(101, root)
            self.assertEqual(guard.scan(root, ['/protected'], ['/scope'], [control])['found'], [])
            control['start'] = 'reused'
            self.assertTrue(guard.scan(root, ['/protected'], ['/scope'], [control])['found'])
            control = guard.process_record(101, root)
            (proc / 'cmdline').write_bytes(b'other\0')
            self.assertTrue(guard.scan(root, ['/protected'], ['/scope'], [control])['found'])

    def test_partial_permission_keeps_prior_attribution(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            proc = self.make_proc(root, ['python', '/protected/job'])
            original = Path.iterdir
            def iterdir(path):
                if path == proc / 'fd':
                    raise PermissionError('fixture denied fd')
                return original(path)
            with patch.object(Path, 'iterdir', iterdir):
                result = guard.scan(root, ['/protected'])
            self.assertTrue(result['found'])
            self.assertTrue(result['unknown'][0]['attributed'])
            self.assertIn('fd', result['unknown'][0]['unknown_fields'])

    def test_disappearance_does_not_erase_prior_evidence(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            proc = self.make_proc(root, ['python', '/protected/job'])
            before = guard.identity(proc)
            with patch.object(guard, 'identity', side_effect=[before, FileNotFoundError()]):
                result = guard.scan(root, ['/protected'])
            self.assertTrue(result['found'])
            self.assertIn('disappeared_after_evidence', result['unknown'][0]['unknown_fields'])

    def test_missing_peer_ack_expires_without_running_any_process(self):
        with tempfile.TemporaryDirectory() as directory:
            pair = {'proofs': directory}
            with patch.object(pilot.time, 'monotonic', return_value=10):
                with self.assertRaisesRegex(AssertionError, 'peer publication deadline'):
                    pilot.wait_publication(pair, 'baseline', 'build', {}, 10)

    def test_first_red_is_immutable(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            pilot.first_red(root, ValueError('first'))
            pilot.first_red(root, ValueError('second'))
            self.assertIn('first', (root / 'first-red.json').read_text())
            self.assertNotIn('second', (root / 'first-red.json').read_text())


if __name__ == '__main__':
    unittest.main()
