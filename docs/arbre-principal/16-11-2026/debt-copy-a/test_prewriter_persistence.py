"""Actual inventory/save/writer statements in source order; writer is a spy.

Other driver statements (health, READY, transport) are intentionally excluded:
this judges inventory persistence order only, never the full driver lifecycle.
"""
from pathlib import Path
from types import SimpleNamespace
from unittest.mock import patch
import ast,json,os,tempfile,unittest
import lifecycle as c

class PrewriterPersistence(unittest.TestCase):
 def test_real_inventory_is_fsynced_before_actual_writer_call(self):
  driver=next(n for n in ast.parse(Path(c.__file__).read_text()).body if isinstance(n,ast.FunctionDef) and n.name=='driver')
  names=['copy-before-writer.json','copy-directories-before-writer.json']
  selected=[]
  for node in driver.body:
   if isinstance(node,ast.Assign) and any(isinstance(t,ast.Name) and t.id=='before_writer' for t in node.targets):selected.append(node)
   if isinstance(node,ast.Expr) and isinstance(node.value,ast.Call) and isinstance(node.value.func,ast.Name):
    call=node.value
    if call.func.id=='save' and call.args and isinstance(call.args[0],ast.Constant) and call.args[0].value in names:selected.append(node)
    if call.func.id=='writer':selected.append(node);break
  self.assertTrue(selected);self.assertEqual(selected[-1].value.func.id,'writer')
  with tempfile.TemporaryDirectory() as d:
   root=Path(d);files=[dict(path='fixture.rag3db',size=100,mode=420,sha256='a'*64)];dirs=[dict(path='fts',mode=493,uid=os.getuid())]
   synced=[];observations=[];real_sync=os.fsync
   def fsync(fd):real_sync(fd);synced.append(os.fstat(fd).st_ino)
   def writer(*args):
    observed={}
    for name,expected in zip(names,[files,dirs]):
     path=root/name
     observed[name]=dict(exists=path.exists(),content=json.loads(path.read_text()) if path.exists() else None,fsynced=path.exists() and path.stat().st_ino in synced,expected=expected)
    observations.append(observed)
   namespace=dict(vars(c),BASE=root/'fixture.rag3db',writer=writer,fs=SimpleNamespace(file_inventory=lambda _:files),cfg={},env={},profile={},grant={},end=100.,nonce='fixture')
   with patch.object(c,'P',root),patch.object(c.a,'P',root),patch.object(c.q,'P',root),patch.object(c.q,'directories',return_value=dirs),patch.object(os,'fsync',side_effect=fsync):
    exec(compile(ast.Module(body=selected,type_ignores=[]),'<actual-prewriter-persistence-and-call>','exec'),namespace)
   self.assertEqual(len(observations),1)
   for name,value in observations[0].items():
    with self.subTest(file=name):self.assertTrue(value['exists'] and value['fsynced'] and value['content']==value['expected'],value)

if __name__=='__main__':unittest.main()
