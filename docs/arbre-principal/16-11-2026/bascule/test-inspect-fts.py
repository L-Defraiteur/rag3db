"""Private filesystem fixtures; no host, Catalog, database, Cargo or writer."""
from pathlib import Path
import importlib.util,json,tempfile,unittest
spec=importlib.util.spec_from_file_location('inspect_fts',Path(__file__).with_name('inspect-fts.py'));module=importlib.util.module_from_spec(spec);spec.loader.exec_module(module)
class ReadOnlyInventory(unittest.TestCase):
 def test_absent_root_is_not_created(self):
  with tempfile.TemporaryDirectory() as d:
   root=Path(d)/'absent'
   with self.assertRaises(ValueError):module.inspect(root)
   self.assertFalse(root.exists())
 def test_corrupt_config_is_not_repaired(self):
  with tempfile.TemporaryDirectory() as d:
   root=Path(d);index=root/'Lucivy_File';index.mkdir();(index/'_shard_config.json').write_bytes(b'broken')
   before=module.file_inventory(root)
   with self.assertRaises(json.JSONDecodeError):module.inspect(root)
   self.assertEqual(before,module.file_inventory(root))
 def test_valid_layout_remains_inventory_only(self):
  with tempfile.TemporaryDirectory() as d:
   root=Path(d);index=root/'Lucivy_File';index.mkdir();(index/'_shard_config.json').write_text(json.dumps(dict(fields=[],shards=1)));(index/'_generation').write_text('7');shard=index/'shard_0';shard.mkdir();(shard/'meta.json').write_text('{}')
   before=module.file_inventory(root);value=module.inspect(root)
   self.assertFalse(value['lucivy_open_qualified']);self.assertEqual(value['indexes'][0]['generation_file'],7);self.assertEqual(before,module.file_inventory(root))
if __name__=='__main__':unittest.main(failfast=True)
