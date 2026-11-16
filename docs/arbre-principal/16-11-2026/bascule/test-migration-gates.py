"""Pure gates for the private official-CLI runner. No native database or service."""
from pathlib import Path
import importlib.util,re,unittest
spec=importlib.util.spec_from_file_location('migration_gates',Path(__file__).with_name('migrate-closed-base.py'));m=importlib.util.module_from_spec(spec);spec.loader.exec_module(m)
A='00000000-0000-0000-0000-000000000001';B='00000000-0000-0000-0000-000000000002'
class MigrationGates(unittest.TestCase):
 def test_stored_form_comes_from_actual_source_contract(self):
  source=Path(__file__).resolve().parents[4]/'extension/rag3weaver/src/code.rs'
  self.assertEqual(re.search(r'pub const SCOPE_FORM_GENERATIONS: &str = "([^"]+)"',source.read_text())[1],m.STORED_GENERATIONS_FORM)
 def test_inventory_order_does_not_change_identity(self):
  self.assertEqual(m.normalize_inventory([dict(path='z'),dict(path='a')]),m.normalize_inventory([dict(path='a'),dict(path='z')]))
 def test_duplicate_inventory_path_refused(self):
  with self.assertRaises(ValueError):m.normalize_inventory([dict(path='a'),dict(path='a')])
 def test_mirror_multiplicity_preserved(self):
  self.assertNotEqual(m.mirror_pairs([A+'|'+B,A+'|'+B]),m.mirror_pairs([A+'|'+B]))
 def test_invalid_mirror_id_refused(self):
  with self.assertRaises(ValueError):m.mirror_pairs(['not-an-id|'+B])
 def test_unknown_model_storage_refused(self):
  with self.assertRaises(ValueError):m.model_columns('invented','granite_278m')
 def test_storage_names_are_exact(self):
  self.assertEqual(m.model_columns('legacy','granite_278m'),('embedding','_embed_hash'))
  self.assertEqual(m.model_columns('suffixed','granite_278m'),('embedding__granite_278m','_embed_hash__granite_278m'))
 def test_truncated_query_refused(self):
  with self.assertRaises(ValueError):m.one_query_rows('BEGIN_QUERY\t0\nROW\t1')
if __name__=='__main__':unittest.main(failfast=True)
