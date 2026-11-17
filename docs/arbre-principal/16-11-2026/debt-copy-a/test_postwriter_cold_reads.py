"""Actual final cold-read gate, separate from physical writer admission."""
from pathlib import Path
from types import SimpleNamespace
from unittest.mock import patch
import ast,hashlib,unittest
import lifecycle as c

class PostwriterColdReads(unittest.TestCase):
 def gate(self,after):
  driver=next(n for n in ast.parse(Path(c.__file__).read_text()).body if isinstance(n,ast.FunctionDef) and n.name=='driver')
  start=next(i for i,n in enumerate(driver.body) if isinstance(n,ast.Assign) and any(isinstance(t,ast.Name) and t.id=='after_reads' for t in n.targets))
  end=next(i for i,n in enumerate(driver.body[start:],start) if isinstance(n,ast.Expr) and isinstance(n.value,ast.Call) and isinstance(n.value.func,ast.Name) and n.value.func.id=='save' and isinstance(n.value.args[0],ast.Constant) and n.value.args[0].value=='sources-after.json')
  initial=dict(fin_dirs=[]);base=Path('/fixture/code-principal.rag3db');before=[dict(path=base.name,size=100,mode=420,sha256='a'*64)]
  fn=ast.FunctionDef(name='actual_cold_read_gate',args=ast.arguments(posonlyargs=[],args=[],vararg=None,kwonlyargs=[],kw_defaults=[],kwarg=None,defaults=[]),body=driver.body[start:end]+[ast.Return(value=ast.Constant(True))],decorator_list=[],type_params=[])
  namespace=dict(vars(c),BASE=base,initial=initial,after_writer=before,fs=SimpleNamespace(file_inventory=lambda _:after),sources=lambda _:initial)
  exec(compile(ast.fix_missing_locations(ast.Module(body=[fn],type_ignores=[])),'<actual-postwriter-cold-read-gate>','exec'),namespace)
  with patch.object(c.a,'BASE',base),patch.object(c.q,'directories',return_value=[]):
   try:return namespace['actual_cold_read_gate'](),None
   except ValueError as error:return False,str(error)
 def test_actual_cold_read_gate_allows_only_new_empty_readers(self):
  after=[dict(path='code-principal.rag3db',size=100,mode=420,sha256='a'*64),dict(path='code-principal.rag3db.readers',size=0,mode=420,sha256=hashlib.sha256(b'').hexdigest())]
  accepted,reason=self.gate(after);self.assertTrue(accepted,reason)
 def test_actual_cold_read_gate_refuses_nonempty_readers(self):
  after=[dict(path='code-principal.rag3db',size=100,mode=420,sha256='a'*64),dict(path='code-principal.rag3db.readers',size=1,mode=420,sha256='b'*64)]
  accepted,_=self.gate(after);self.assertFalse(accepted)
 def test_actual_cold_read_gate_keeps_database_bytes_strict(self):
  accepted,_=self.gate([dict(path='code-principal.rag3db',size=100,mode=420,sha256='b'*64)]);self.assertFalse(accepted)

if __name__=='__main__':unittest.main()
