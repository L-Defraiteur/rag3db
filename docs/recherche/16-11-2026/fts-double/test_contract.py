"""Synthetic contracts only; never invokes Cargo, a process or a database."""
import unittest
import qualify as q
class Contracts(unittest.TestCase):
 def test_inventory_requires_two_exact_nonignored_names(self):
  raw='\n'.join(n+': test' for n in q.NAMES)+'\n2 tests, 0 benchmarks\n'
  self.assertEqual(q.inventory_names(raw),q.NAMES)
  for bad in [raw.replace(q.NAMES[0],'other'),raw+'other: test\n']:
   with self.assertRaises(AssertionError):q.inventory_names(bad)
 def test_artifact_requires_one_unit_test_from_exact_source(self):
  row=dict(reason='compiler-artifact',package_id='rag3weaver',target=dict(name='rag3weaver',kind=['lib'],src_path='/w/src/lib.rs'),profile=dict(test=True),executable='/t/elf',features=['code','rag3db-native'])
  self.assertEqual(q.choose_artifact([row],'/w/src/lib.rs')['executable'],'/t/elf')
  for bad in [[],[row,row],[dict(row,target=dict(row['target'],src_path='/other'))]]:
   with self.assertRaises(AssertionError):q.choose_artifact(bad,'/w/src/lib.rs')
 def test_first_red_detects_failure_and_panic(self):
  for raw in ['test x ... FAILED','FAILED','thread x panicked at x','test result: FAILED.']:
   self.assertTrue(q.FAILED.search(raw))
  for raw in ['test x ... ok','test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1 filtered out;']:
   self.assertFalse(q.FAILED.search(raw))
 def test_outer_timeout_covers_composer_after_poste(self):
  cfg=dict(timeout='/timeout');cmd=q.outer_command(cfg)
  self.assertEqual(cmd[1:6],['lourd','/timeout','--signal=TERM','--kill-after=2s','1198s'])
  self.assertEqual(cmd[-1],'--compose')
if __name__=='__main__':unittest.main(failfast=True)
