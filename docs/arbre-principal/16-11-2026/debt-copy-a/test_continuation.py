"""Original-deadline receipt and continuation routing; no real process."""
import ast,json,tempfile,types,unittest
from pathlib import Path
from unittest.mock import patch
import continue_result as c

class ContinuationContract(unittest.TestCase):
 def test_poste_receipt_uses_only_remaining_original_budget(self):
  common=types.SimpleNamespace(poste_tenu=lambda p:True,processus=lambda:[])
  with tempfile.TemporaryDirectory() as d:
   root=Path(d);(root/'prepared.json').write_text(json.dumps(dict(timeout='/fake/timeout')))
   with patch.object(c,'Q',root),patch.object(c.q,'loadmod',return_value=common),patch.object(c.time,'monotonic',return_value=c.END-100),patch.object(c.os,'execv') as execute:c.post()
   receipt=json.loads((root/'after-poste.json').read_text());self.assertEqual(receipt['remaining'],97);self.assertEqual(receipt['soft']+2,97);self.assertEqual(receipt['common'],77)
   self.assertEqual(execute.call_args.args[1][3],'95s')
 def test_expired_or_short_remaining_refuses_without_exec(self):
  common=types.SimpleNamespace(poste_tenu=lambda p:True,processus=lambda:[])
  with tempfile.TemporaryDirectory() as d:
   root=Path(d);(root/'prepared.json').write_text(json.dumps(dict(timeout='/fake/timeout')))
   for now in [c.END-60,c.END,c.END+1]:
    with patch.object(c,'Q',root),patch.object(c.q,'loadmod',return_value=common),patch.object(c.time,'monotonic',return_value=now),patch.object(c.os,'execv') as execute,self.assertRaises(ValueError):c.post()
    execute.assert_not_called();self.assertFalse((root/'after-poste.json').exists())
 def test_driver_classifies_saved_graph_without_repeating_snapshot(self):
  tree=ast.parse(Path(c.__file__).read_text());driver=next(n for n in tree.body if isinstance(n,ast.FunctionDef) and n.name=='driver');calls=[ast.unparse(n.func) for n in ast.walk(driver) if isinstance(n,ast.Call)]
  self.assertIn('a.same_graph_publication',calls);self.assertIn('q.fts',calls);self.assertIn('a.audit.finished',calls)
  for forbidden in ['a.audit.snapshot','subprocess.run','subprocess.Popen','q.health','q.driver']:self.assertNotIn(forbidden,calls)

if __name__=='__main__':unittest.main(failfast=True)
