"""Bounded Linux ETXTBSY witness. No Rust build or product process."""
import argparse
import errno
import json
import os
from pathlib import Path
import re
import select
import subprocess
import tempfile
import unittest

SOURCE = Path(__file__).resolve().parents[3] / 'extension/rag3weaver/src/codex_threads.rs'
BODY = '#!/bin/sh\nprintf fixture-ok\\n\n'


def run_with_inherited_writer(use_child_writer, rename=False):
    with tempfile.TemporaryDirectory(prefix='script-fixture-') as tmp:
        path = Path(tmp) / 'fixture with spaces'
        writer = None
        if not use_child_writer:
            writer = path.open('w')
            writer.write(BODY)
            writer.flush()
            path.chmod(0o700)
        ready_r, ready_w = os.pipe()
        release_r, release_w = os.pipe()
        pid = os.fork()
        if pid == 0:
            os.close(ready_r)
            os.close(release_w)
            os.write(ready_w, b'R')
            os.close(ready_w)
            os.read(release_r, 1)
            os._exit(0)
        os.close(ready_w)
        os.close(release_r)
        try:
            assert select.select([ready_r], [], [], 2)[0], 'fork witness was not ready'
            assert os.read(ready_r, 1) == b'R'
            if writer is not None:
                writer.close()
                if rename:
                    renamed = path.with_name('renamed executable')
                    path.rename(renamed)
                    path = renamed
            else:
                source = SOURCE.read_text()
                match = re.search(r'const WRITE_EXECUTABLE_SCRIPT: &str = r#"(.*?)"#;', source, re.S)
                assert match, 'reviewed test-only writer command is missing'
                subprocess.run(['/bin/sh', '-c', match[1], 'write-fixture', str(path), BODY],
                               check=True, timeout=2, capture_output=True)
            try:
                result = subprocess.run([str(path)], timeout=2, capture_output=True)
                return {'status': result.returncode, 'stdout': result.stdout.decode()}
            except OSError as error:
                return {'errno': error.errno, 'error': str(error)}
        finally:
            os.write(release_w, b'X')
            os.close(release_w)
            os.close(ready_r)
            os.waitpid(pid, 0)
            if writer is not None:
                writer.close()


class ScriptFixtures(unittest.TestCase):
    def test_inherited_writer_reproduces_etxtbsy(self):
        self.assertEqual(run_with_inherited_writer(False)['errno'], errno.ETXTBSY)

    def test_rename_keeps_the_busy_inode(self):
        self.assertEqual(run_with_inherited_writer(False, rename=True)['errno'], errno.ETXTBSY)

    def test_script_writer_runs_outside_the_forking_parent(self):
        result = run_with_inherited_writer(USE_CHILD_WRITER)
        self.assertEqual(result.get('status'), 0, result)
        self.assertIn('fixture-ok', result['stdout'])


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--legacy', action='store_true')
    args = parser.parse_args()
    USE_CHILD_WRITER = not args.legacy
    unittest.main(argv=['script_fixture_probe'], verbosity=2)
