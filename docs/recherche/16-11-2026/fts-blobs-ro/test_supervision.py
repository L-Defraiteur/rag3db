"""Outer supervision witnesses, synthetic process replies and no native run."""
import ast,json,tempfile,types,unittest
from pathlib import Path
SOURCE=Path(__file__).with_name('qualify.py')
def extracted(names,ns):
 tree=ast.parse(SOURCE.read_text());nodes=[n for n in tree.body if isinstance(n,ast.FunctionDef) and n.name in names]
 assert len(nodes)==len(names)
 exec(compile(ast.Module(body=nodes,type_ignores=[]),'actual-supervision','exec'),ns);return ns
class Supervision(unittest.TestCase):
 def test_outer_timeout_covers_composer_after_poste_with_total_bound600(self):
  q=Path('/fixture/control');ns=extracted({'outer_command'},dict(Q=q,sys=types.SimpleNamespace(executable='/python')))
  argv=ns['outer_command']({'timeout':'/usr/bin/timeout'})
  self.assertEqual(argv[:6],[str(q/'poste'),'lourd','/usr/bin/timeout','--signal=TERM','--kill-after=2s','598s'])
  self.assertEqual(argv[-1],'--compose');self.assertNotIn('--driver',argv);self.assertNotIn('lancer',argv)
  self.assertEqual(598+2,600)
 def test_actual_outer_return_does_no_final_hash_or_inventory(self):
  with tempfile.TemporaryDirectory(prefix='fts-outer-fixture-') as tmp:
   q=Path(tmp);calls=[]
   def run(argv,**kwargs):calls.append(argv);return types.SimpleNamespace(returncode=124)
   ns=extracted({'run_outer'},dict(Q=q,subprocess=types.SimpleNamespace(run=run,STDOUT=-2),outer_command=lambda cfg:['poste','lourd','timeout','compose']))
   self.assertEqual(ns['run_outer']({},{}),124);self.assertEqual(len(calls),1)
 def test_parent_signature_rejects_unbounded_or_foreign_parent(self):
  ns=extracted({'verify_outer_signature'},dict(Q=Path('/fixture/control'),sys=types.SimpleNamespace(executable='/python')))
  cfg={'timeout':'/usr/bin/timeout'};expected=['/usr/bin/timeout','--signal=TERM','--kill-after=2s','598s','/python','/fixture/control/qualify.py','--compose']
  ns['verify_outer_signature'](dict(exe='/usr/bin/timeout',argv=expected,poste=True),cfg)
  for bad in [dict(exe='/other',argv=expected,poste=True),dict(exe='/usr/bin/timeout',argv=expected,poste=False),dict(exe='/usr/bin/timeout',argv=['/usr/bin/timeout','630s','other'],poste=True)]:
   with self.subTest(bad=bad),self.assertRaises(AssertionError):ns['verify_outer_signature'](bad,cfg)
if __name__=='__main__':unittest.main()
