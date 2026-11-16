"""FIN routing and actual launch argv with a fake subprocess; no native call."""
import ast,json,tempfile,types,unittest
from pathlib import Path
from unittest.mock import patch
import finish_result as f

class FinishContract(unittest.TestCase):
 def test_distinct_600_outer_is_used_without_old_deadline_reset(self):
  with tempfile.TemporaryDirectory() as d:
   root=Path(d);q=root/'Q';q.mkdir();(q/'prepared.json').write_text(json.dumps(dict(pilot='exact',timeout='/fake/timeout')))
   with patch.object(f,'Q',q),patch.object(f,'P',root/'P'),patch.object(f,'S',root/'S'),patch.object(f,'guard'),patch.object(f.subprocess,'run',return_value=types.SimpleNamespace(returncode=0)) as run:
    self.assertEqual(f.launch('exact','new-FIN-GO'),0)
   argv=run.call_args.args[0]
   self.assertEqual(argv[:6],[str(q/'poste'),'lourd','/fake/timeout','--signal=TERM','--kill-after=2s','598s'])
   allocation=json.loads((q/'allocation.json').read_text());self.assertEqual(allocation['distinct_budget'],600);self.assertEqual(allocation['history']['audit_deadline'],1190939.194661205)
   self.assertFalse((root/'P').exists());self.assertFalse((root/'S').exists())
 def test_actual_driver_copies_frozen_bundle_before_only_remaining_judges(self):
  tree=ast.parse(Path(f.__file__).read_text());driver=next(n for n in tree.body if isinstance(n,ast.FunctionDef) and n.name=='driver');calls=[ast.unparse(n.func) for n in ast.walk(driver) if isinstance(n,ast.Call)]
  self.assertIn('a.same_graph_publication',calls);self.assertIn('q.fts',calls);self.assertIn('a.audit.finished',calls)
  for forbidden in ['a.audit.snapshot','subprocess.Popen','q.health','q.driver']:self.assertNotIn(forbidden,calls)
  assignment=next(n for n in ast.walk(driver) if isinstance(n,ast.Assign) and any(isinstance(t,ast.Name) and t.id=='argv' for t in n.targets))
  self.assertEqual([x.value for x in assignment.value.elts[:4]],['cp','-a','--reflink=always','--']);self.assertEqual(ast.unparse(assignment.value.elts[4]),'str(FROZEN)')
  lines={ast.unparse(n.func):n.lineno for n in ast.walk(driver) if isinstance(n,ast.Call) and ast.unparse(n.func) in ['subprocess.run','q.Reader']};self.assertLess(lines['subprocess.run'],lines['q.Reader'])

if __name__=='__main__':unittest.main(failfast=True)
