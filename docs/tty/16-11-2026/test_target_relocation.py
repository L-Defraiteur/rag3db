"""Témoin de sémantique filesystem uniquement, sans Cargo ni target réel."""
import hashlib
import os
from pathlib import Path
import tempfile
import unittest


def relocate(source, destination, expected, owned_writers):
    if owned_writers:
        raise ValueError('owned writer')
    if source.is_symlink() or destination.exists() or destination.is_symlink():
        raise ValueError('unexpected path')
    before = source.stat()
    if before.st_uid != os.getuid() or before.st_dev != destination.parent.stat().st_dev:
        raise ValueError('ownership or device')
    if (before.st_dev, before.st_ino) != expected:
        raise ValueError('source changed')
    os.rename(source, destination)


class RelocationTests(unittest.TestCase):
    def test_open_descriptor_keeps_original_inode_after_relocation(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            source = root / 'target'
            source.mkdir()
            binary = source / 'artifact'
            binary.write_bytes(b'original artifact')
            binary.chmod(0o640)
            old = binary.stat()
            expected = (source.stat().st_dev, source.stat().st_ino)
            with binary.open('rb') as reader:
                relocate(source, root / 'archive', expected, [])
                self.assertFalse(source.exists())
                archived = root / 'archive' / 'artifact'
                self.assertEqual(archived.stat().st_ino, old.st_ino)
                self.assertEqual(archived.stat().st_mode, old.st_mode)
                self.assertEqual(hashlib.sha256(reader.read()).digest(), hashlib.sha256(archived.read_bytes()).digest())
                source.mkdir()  # nouvelle racine, aucun import d'artefacts
                self.assertNotEqual(source.stat().st_ino, expected[1])
                self.assertEqual(list(source.iterdir()), [])

    def test_owned_writer_or_existing_destination_refuses(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            source = root / 'target'
            source.mkdir()
            expected = (source.stat().st_dev, source.stat().st_ino)
            with self.assertRaisesRegex(ValueError, 'owned writer'):
                relocate(source, root / 'archive', expected, [123])
            (root / 'archive').mkdir()
            with self.assertRaisesRegex(ValueError, 'unexpected path'):
                relocate(source, root / 'archive', expected, [])
            self.assertTrue(source.is_dir())


if __name__ == '__main__':
    unittest.main()
