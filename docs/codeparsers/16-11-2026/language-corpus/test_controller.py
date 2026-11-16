"""Contrats du transport, avec nos propres processus légers, sans corpus ni Cargo."""
import json
import tempfile
import importlib.util
from pathlib import Path
import subprocess
import sys
import time
import unittest

spec = importlib.util.spec_from_file_location('probe_control', Path(__file__).with_name('control_probe.py'))
control = importlib.util.module_from_spec(spec)
spec.loader.exec_module(control)


class WorkerTransportTests(unittest.TestCase):
    def worker(self, source):
        worker = subprocess.Popen([sys.executable, '-c', source], stdin=subprocess.PIPE,
            stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, start_new_session=True)
        self.addCleanup(control.stop, worker)
        return worker

    def test_actual_canonical_supervisor_path_exists(self):
        self.assertTrue(control.COMMON.is_file(), str(control.COMMON))
        self.assertEqual(control.COMMON.name, "confiner_validation.py")

    def test_fragmented_json_is_read_until_newline(self):
        worker = self.worker("import os,time; os.write(1,b'{\"ok\":'); time.sleep(.02); os.write(1,b'true}\\n')")
        row, failure = control.response(worker, time.monotonic() + 2)
        self.assertEqual(row, {'ok': True})
        self.assertIsNone(failure)

    def test_silent_worker_is_bounded_and_reaped(self):
        worker = self.worker('import time; time.sleep(30)')
        started = time.monotonic()
        row, failure = control.response(worker, started + .05)
        self.assertIsNone(row)
        self.assertEqual(failure, 'timeout')
        control.stop(worker)
        self.assertIsNotNone(worker.poll())
        self.assertLess(time.monotonic() - started, 2)

    def test_abrupt_exit_is_distinct_from_timeout(self):
        worker = self.worker('import os; os._exit(17)')
        row, failure = control.response(worker, time.monotonic() + 2)
        self.assertIsNone(row)
        self.assertEqual(failure, 'worker_exit')
        worker.wait()
        self.assertEqual(worker.returncode, 17)

    def test_invalid_protocol_is_a_harness_error(self):
        worker = self.worker("print('not-json')")
        with self.assertRaises(ValueError):
            control.response(worker, time.monotonic() + 2)


class ArchiveProvenanceTests(unittest.TestCase):
    def fixture(self):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        parent = Path(temporary.name).resolve()
        root = parent / 'source'
        root.mkdir()
        (parent / 'source.tar.gz').write_bytes(b'archive fixture only, never extracted')
        (parent / 'inventory.jsonl').write_bytes(b'{}\n')
        identity = 'github.com/example/corpus@' + 'a' * 40
        freeze = {'repository_id': identity, 'repository_commit': 'a' * 40,
            'root': str(root), 'executed_repository_code': False,
            'archive_sha256': control.digest(parent / 'source.tar.gz'),
            'inventory_sha256': control.digest(parent / 'inventory.jsonl')}
        (parent / 'freeze.json').write_text(json.dumps(freeze))
        return parent, {'schema_version': 1, 'repositories': {identity: str(root)}}

    def test_frozen_archive_does_not_require_git(self):
        parent, roots = self.fixture()
        control.verify_roots(roots)
        self.assertFalse((parent / 'source/.git').exists())

    def test_changed_archive_is_rejected(self):
        parent, roots = self.fixture()
        (parent / 'source.tar.gz').write_bytes(b'changed')
        with self.assertRaises(ValueError):
            control.verify_roots(roots)


if __name__ == '__main__':
    unittest.main()
