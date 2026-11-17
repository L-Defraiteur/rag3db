"""Actual driver inventory gate, synthetic inventories; no writer or DB.

Evidence/clock preconditions need separate witnesses before implementation.
Negative inventory cases can be rejected by the old wrong guard first;
their passing alone never qualifies a future post-writer branch.
"""
from pathlib import Path
from types import SimpleNamespace
from unittest.mock import patch
import ast,copy,tempfile,unittest
import lifecycle as c

class PostwriterInventory(unittest.TestCase):
 def setUp(self):
  self.tmp=tempfile.TemporaryDirectory();self.addCleanup(self.tmp.cleanup)
  self.root=Path(self.tmp.name);self.base=self.root/'code-principal.rag3db'
  def row(name,tag='a'):return dict(path=name,size=100,mode=420,sha256=tag*64)
  self.before=[row(self.base.name),row(self.base.name+'.wal'),row(self.base.name+'.ecrivain'),row('fts/shard_0')]
  self.after=[row(self.base.name,'b'),row('fts/shard_0')]
  self.dirs=[dict(path='fts',mode=493)]
 def gate(self,after=None,before=None,dirs=None):
  evidence=Path(tempfile.mkdtemp(prefix='observation-',dir=self.root))
  tree=ast.parse(Path(c.__file__).read_text());driver=next(n for n in tree.body if isinstance(n,ast.FunctionDef) and n.name=='driver')
  start=next(i for i,n in enumerate(driver.body) if isinstance(n,ast.Assign) and any(isinstance(t,ast.Name) and t.id=='after_writer' for t in n.targets))
  end=next(i for i,n in enumerate(driver.body[start:],start) if isinstance(n,ast.Expr) and isinstance(n.value,ast.Call) and isinstance(n.value.func,ast.Name) and n.value.func.id=='save' and n.value.args and isinstance(n.value.args[0],ast.Constant) and n.value.args[0].value=='copy-after-shutdown-quiet.json')
  body=driver.body[start:end]+[ast.Return(value=ast.Constant(True))]
  function=ast.FunctionDef(name='actual_postwriter_inventory_gate',args=ast.arguments(posonlyargs=[],args=[],vararg=None,kwonlyargs=[],kw_defaults=[],kwarg=None,defaults=[]),body=body,decorator_list=[],type_params=[])
  namespace=dict(vars(c),BASE=self.base,before_writer=copy.deepcopy(self.before if before is None else before),initial=dict(fin_dirs=self.dirs),fs=SimpleNamespace(file_inventory=lambda _:copy.deepcopy(self.after if after is None else after)))
  exec(compile(ast.fix_missing_locations(ast.Module(body=[function],type_ignores=[])),'<actual-postwriter-inventory>','exec'),namespace)
  with patch.object(c,'P',evidence),patch.object(c.a,'P',evidence),patch.object(c.a,'BASE',self.base),patch.object(c.q,'P',evidence),patch.object(c.q,'directories',return_value=self.dirs if dirs is None else dirs):
   try:return namespace['actual_postwriter_inventory_gate'](),None
   except ValueError as error:return False,str(error)
 def test_exact_writer_auxiliary_removal_is_not_a_readonly_mutation(self):
  accepted,reason=self.gate();self.assertTrue(accepted,reason)
 def test_unrelated_fts_removal_is_refused(self):
  accepted,_=self.gate(after=self.after[:1]);self.assertFalse(accepted)
 def test_unrelated_fts_bytes_or_mode_change_is_refused(self):
  for key,value in [('sha256','c'*64),('mode',384)]:
   with self.subTest(field=key):
    after=copy.deepcopy(self.after);after[1][key]=value;accepted,_=self.gate(after=after);self.assertFalse(accepted)
 def test_residual_or_new_wal_is_refused(self):
  for suffix in ['.wal','.wal.unexpected']:
   with self.subTest(suffix=suffix):
    after=copy.deepcopy(self.after)+[dict(self.before[1],path=self.base.name+suffix)];accepted,_=self.gate(after=after);self.assertFalse(accepted)
 def test_unrelated_addition_or_directory_change_is_refused(self):
  accepted,_=self.gate(after=self.after+[dict(self.before[1],path='unexpected')]);self.assertFalse(accepted)
  accepted,_=self.gate(dirs=[]);self.assertFalse(accepted)
 def test_duplicate_inventory_paths_cannot_be_silently_collapsed(self):
  # Unlike the nominal writer case this keeps all auxiliaries: the existing
  # dictionary conversion silently collapses the duplicate instead of refusing.
  accepted,_=self.gate(before=self.before,after=self.before+[copy.deepcopy(self.before[-1])]);self.assertFalse(accepted)

if __name__=='__main__':unittest.main()
