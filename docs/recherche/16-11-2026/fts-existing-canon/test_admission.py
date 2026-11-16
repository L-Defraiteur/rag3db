"""Exercise actual admission AST with a fake clock and exec, no real poste."""
import ast,json,math,tempfile,types,unittest
from pathlib import Path
class Admission(unittest.TestCase):
 def test_unique_fresh_deadline_after_poste_never_resets(self):
  source=Path(__file__).with_name('admit.py');tree=ast.parse(source.read_text())
  body=[node for node in tree.body if not isinstance(node,(ast.Import,ast.ImportFrom))]
  code=compile(ast.Module(body=body,type_ignores=[]),'actual-admission','exec')
  with tempfile.TemporaryDirectory(prefix='fts-admission-fixture-') as tmp:
   root=Path(tmp);(root/'prepared.json').write_text(json.dumps(dict(budget=6000,scope='/fixture/scope')))
   calls=[]
   def execute():
    times=iter([100.,100.001])
    ns=dict(__file__=str(root/'admit.py'),Path=Path,json=json,math=math,time=types.SimpleNamespace(monotonic=lambda:next(times)),os=types.SimpleNamespace(execv=lambda *args:calls.append(args)),sys=types.SimpleNamespace(executable='/fixture/python'))
    exec(code,ns)
   execute();row=json.loads((root/'remaining-after-poste.json').read_text())
   self.assertEqual(row['original_start'],100.);self.assertEqual(row['original_end'],6100.)
   self.assertLess(row['duration'],6000);self.assertGreater(row['duration'],60)
   self.assertEqual(calls[0][1][2],'lancer')
   with self.assertRaises(FileExistsError):execute()
   self.assertEqual(len(calls),1,'no second authority on the same allocation')
if __name__=='__main__':unittest.main()
