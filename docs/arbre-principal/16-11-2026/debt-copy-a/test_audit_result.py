"""Hermetic guards for the RO-only result observation; no native execution."""
import ast,copy,hashlib,unittest
from pathlib import Path
import audit_result as r

class FrozenCopyGuards(unittest.TestCase):
 def setUp(self):
  self.original=[dict(path=r.BASE.name,size=12,mode=420,sha256='original')]
  self.reader=dict(path=r.BASE.name+'.readers',size=0,mode=420,sha256=hashlib.sha256(b'').hexdigest())
 def test_readers_exception_is_new_copy_exact_zero_only(self):
  self.assertFalse(r.stable_readers(self.original,self.original)['readers_zero_added'])
  self.assertTrue(r.stable_readers(self.original,self.original+[self.reader])['readers_zero_added'])
  for key,value in [('size',1),('mode',384),('sha256','changed'),('path','foreign.readers')]:
   bad=dict(self.reader,**{key:value})
   with self.subTest(key=key),self.assertRaises(ValueError):r.stable_readers(self.original,self.original+[bad])
 def test_new_wal_and_existing_bytes_or_readers_changes_refused(self):
  for after in [self.original+[dict(self.reader,path=r.BASE.name+'.wal')],[],[dict(self.original[0],sha256='changed')],self.original+[self.reader,dict(self.reader,path='other')]]:
   with self.subTest(after=after),self.assertRaises(ValueError):r.stable_readers(self.original,after)
  before=self.original+[self.reader]
  with self.assertRaises(ValueError):r.stable_readers(before,self.original)
 def test_driver_uses_full_copy_and_only_ro_primitives(self):
  tree=ast.parse(Path(r.__file__).read_text());driver=next(n for n in tree.body if isinstance(n,ast.FunctionDef) and n.name=='driver')
  calls=[ast.unparse(n.func) for n in ast.walk(driver) if isinstance(n,ast.Call)]
  self.assertIn('audit.snapshot',calls);self.assertIn('q.fts',calls);self.assertIn('audit.finished',calls)
  self.assertNotIn('q.driver',calls);self.assertNotIn('q.health',calls);self.assertNotIn('subprocess.Popen',calls)
  assigns=[n for n in ast.walk(driver) if isinstance(n,ast.Assign) and any(isinstance(t,ast.Name) and t.id=='argv' for t in n.targets)]
  self.assertEqual(len(assigns),1);argv=assigns[0].value
  self.assertEqual([x.value for x in argv.elts[:4]],['cp','-a','--reflink=always','--'])
  self.assertEqual(ast.unparse(argv.elts[4]),'str(RESULT)')
  lines={ast.unparse(n.func):n.lineno for n in ast.walk(driver) if isinstance(n,ast.Call) and ast.unparse(n.func) in ['subprocess.run','q.Reader']}
  self.assertLess(lines['subprocess.run'],lines['q.Reader'])

if __name__=='__main__':unittest.main(failfast=True)
