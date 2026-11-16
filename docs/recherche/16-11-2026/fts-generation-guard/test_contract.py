"""Synthetic contracts only; never invokes Cargo, a process or a database."""
import unittest
import qualify as q
class Contracts(unittest.TestCase):
 def test_inventory_requires_two_exact_nonignored_names(self):
  raw='\n'.join(n+': test' for n in q.NAMES)+f'\n{len(q.NAMES)} tests, 0 benchmarks\n'
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

class BeforeActionContracts(unittest.TestCase):
 def test_real_sdk_maps_accept_only_exact_path_and_pinned_bytes(self):
  from sdk_contract import validate_sdk_maps
  expected='/sdk59/librag3db.so';ledger={'sdk':{'librag3db.so':'pin'}}
  def maps(path):return '100-200 r-xp 0000 00:01 42 '+path+'\n'
  good=validate_sdk_maps(maps(expected),ledger,'/sdk59',lambda p:'pin',lambda p:p)
  self.assertEqual(good['path'],expected)
  for path,digest in [('/other/librag3db.so','pin'),(expected,'wrong'),(expected+' (deleted)','pin')]:
   with self.subTest(path=path,digest=digest),self.assertRaises(ValueError):validate_sdk_maps(maps(path),ledger,'/sdk59',lambda p:digest,lambda p:p)
 def test_partial_process_scan_preserves_each_observed_path(self):
  from unittest.mock import patch
  class Proc:
   def __truediv__(self,name):return name
  for failing in ['cwd','fd2']:
   observed=[]
   class Fds:
    def iterdir(self):return iter(['fd1','fd2'])
   class WithFds(Proc):
    def __truediv__(self,name):return Fds() if name=='fd' else name
   def readlink(path):
    if path==failing:raise PermissionError('protected')
    return '/target/'+path
   with patch.object(q.os,'readlink',side_effect=readlink),self.assertRaises(PermissionError):q.collect_process_paths(WithFds(),lambda role,path:observed.append((role,path)))
   self.assertIn(('exe','/target/exe'),observed)
   if failing=='fd2':self.assertIn(('fd','/target/fd1'),observed)

if __name__=='__main__':unittest.main(failfast=True)
