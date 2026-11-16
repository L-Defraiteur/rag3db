"""Sidecar policy on private CPU fixtures, without opening any database."""
from pathlib import Path
import importlib.util,tempfile,unittest
spec=importlib.util.spec_from_file_location('cold_copy',Path(__file__).with_name('inspect-cold-copy.py'));m=importlib.util.module_from_spec(spec);spec.loader.exec_module(m)
class ReadersPolicy(unittest.TestCase):
 def test_exact_empty_regular_sidecar_is_recorded(self):
  with tempfile.TemporaryDirectory() as d:
   root=Path(d);before=m.fs.file_inventory(root);(root/'base.readers').write_bytes(b'')
   value=m.validate_copy(before,m.fs.file_inventory(root),root,'base');self.assertTrue(value['created'])
 def test_nonzero_sidecar_refused(self):
  with tempfile.TemporaryDirectory() as d:
   root=Path(d);before=m.fs.file_inventory(root);(root/'base.readers').write_bytes(b'x')
   with self.assertRaises(ValueError):m.validate_copy(before,m.fs.file_inventory(root),root,'base')
 def test_symlink_sidecar_refused(self):
  with tempfile.TemporaryDirectory() as d:
   root=Path(d);(root/'target').write_bytes(b'');before=m.fs.file_inventory(root);(root/'base.readers').symlink_to(root/'target')
   with self.assertRaises(ValueError):m.validate_copy(before,before,root,'base')
 def test_unexpected_wal_refused(self):
  with tempfile.TemporaryDirectory() as d:
   root=Path(d);before=m.fs.file_inventory(root);(root/'base.wal').write_bytes(b'')
   with self.assertRaises(ValueError):m.validate_copy(before,m.fs.file_inventory(root),root,'base')
 def test_source_change_never_gets_exception(self):
  with self.assertRaises(ValueError):m.validate_source([], [dict(path='base.readers')])
if __name__=='__main__':unittest.main(failfast=True)
