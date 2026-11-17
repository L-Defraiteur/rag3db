"""Real compose tail and real filesystem publication; clock/fsync fakes only."""
from pathlib import Path
from unittest.mock import patch
import ast,copy,json,tempfile,unittest
import lifecycle as c

class LifecycleFinalization(unittest.TestCase):
 def setUp(self):
  self.tmp=tempfile.TemporaryDirectory();self.addCleanup(self.tmp.cleanup);self.root=Path(self.tmp.name);self.now=99.;self.fired=False
  self.record=dict(status='LIFECYCLE_FIXTURE_OBSERVED',deadline=100.,scope=dict(verdict='VERT',code=0,restes=[],nettoye=True),clean_shutdown_observed=True,fixture_only=True,durability_qualified=False,live_authorized=False)
 def finalize(self,mode='normal'):
  tree=ast.parse(Path(c.__file__).read_text());compose=next(n for n in tree.body if isinstance(n,ast.FunctionDef) and n.name=='compose');start=max(i for i,n in enumerate(compose.body) if isinstance(n,ast.Try))+1
  tail=compose.body[start:];self.assertTrue(tail,'exercise the actual compose final block')
  function=ast.FunctionDef(name='actual_compose_tail',args=ast.arguments(posonlyargs=[],args=[],vararg=None,kwonlyargs=[],kw_defaults=[],kwarg=None,defaults=[]),body=tail,decorator_list=[],type_params=[])
  namespace=dict(vars(c),P=self.root,record=copy.deepcopy(self.record));exec(compile(ast.fix_missing_locations(ast.Module(body=[function],type_ignores=[])),'<actual-compose-final-tail>','exec'),namespace)
  real_sync=c.os.fsync;real_link=c.os.link
  def sync(fd):
   real_sync(fd)
   if not self.fired:
    self.fired=True
    if mode=='late-fsync':self.now=100.1
    if mode=='first-red-during-fsync':(self.root/'first-red.json').write_bytes(b'{"reason":"original refusal"}')
  def link(src,dst,*args,**kwargs):
   value=real_link(src,dst,*args,**kwargs)
   if mode=='late-link' and Path(dst).name=='composed.json':self.now=100.1
   return value
  with patch.object(c,'P',self.root),patch.object(c.a,'P',self.root),patch.object(c.q,'P',self.root),patch.object(c.q,'ACTION_GUARD',None),patch.object(c.time,'monotonic',side_effect=lambda:self.now),patch.object(c.os,'fsync',side_effect=sync),patch.object(c.os,'link',side_effect=link):
   return namespace['actual_compose_tail']()
 def result(self):return json.loads((self.root/'composed.json').read_text())
 def test_ontime_publication_and_return_are_positive(self):
  self.assertEqual(self.finalize(),0);self.assertEqual(self.result()['status'],'LIFECYCLE_FIXTURE_OBSERVED')
 def test_actual_fsync_crossing_common_end_cannot_publish_positive_and_return_zero(self):
  self.assertNotEqual(self.finalize('late-fsync'),0);self.assertEqual(self.result()['status'],'RED');self.assertTrue((self.root/'first-red.json').exists())
 def test_deadline_already_passed_is_a_red_result_and_nonzero_return(self):
  self.now=100.1;self.assertNotEqual(self.finalize(),0);self.assertEqual(self.result()['status'],'RED')
 def test_first_red_created_during_final_fsync_is_preserved(self):
  self.assertNotEqual(self.finalize('first-red-during-fsync'),0);self.assertEqual(self.result()['status'],'RED');self.assertEqual((self.root/'first-red.json').read_bytes(),b'{"reason":"original refusal"}')
 def test_late_atomic_publication_is_reclassified_before_return(self):
  self.assertNotEqual(self.finalize('late-link'),0);self.assertEqual(self.result()['status'],'RED')

if __name__=='__main__':unittest.main()
