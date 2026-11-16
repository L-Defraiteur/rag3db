"""Actual no-rebuild admission and manifest/driver routing; pure fake rows."""
from pathlib import Path
import ast,json,unittest
from unittest.mock import patch
import lifecycle as c

def rows(values):return [[('STRING',k.encode()),('STRING',v.encode())] for k,v in values]
class Reader:
 def __init__(self,values):self.values=values
 def collect(self,sql):return rows(self.values)

class LifecycleAdmission(unittest.TestCase):
 def test_fts_pending_and_promise_are_judged_before_writer(self):
  with patch.object(c,'save') as save:
   c.no_fts_rebuild(Reader([('fts_promesse',''),('fts_pending:Scope',' ')]),'proof.json')
   self.assertEqual(save.call_args.args[1]['nonempty'],[])
  for values in [[('fts_promesse',' ')],[('fts_promesse','old-process')],[('fts_pending:Scope','1')],[('fts_pending:Scope',''),('fts_pending:Scope','')]]:
   with self.subTest(values=values),patch.object(c,'save'),self.assertRaises(ValueError):c.no_fts_rebuild(Reader(values),'proof.json')
 def test_manifest_has_explicit_none_signals_and_only_public_nonce_query(self):
  root=Path(__file__).parent;manifest=json.loads((root/'lifecycle.json').read_text());tool=manifest['tools']['observer_cycle_vie']
  self.assertEqual(tool['bindings']['options']['signals'],[]);self.assertEqual(tool['bindings']['options']['exige'],['data','dense'])
  self.assertEqual(manifest['database'],str(c.BASE));self.assertFalse(manifest['embeddings']['catch_up_on_open']);self.assertEqual(manifest['workspace']['commands'],'off');self.assertTrue(manifest['workspace']['read_only'])
  graph=(root/'lifecycle.mmd').read_text();self.assertIn('query=$nonce',graph);self.assertIn('%% result: source.meta',graph)
  for name in ['IndexNode','VectorSearchNode','BM25SearchNode','SparseSearchNode']:self.assertNotIn(name,graph)
 def test_real_driver_checks_debt_rebuild_and_bit_digest_before_writer(self):
  tree=ast.parse(Path(c.__file__).read_text());driver=next(n for n in tree.body if isinstance(n,ast.FunctionDef) and n.name=='driver')
  calls=[(ast.unparse(n.func),n.lineno) for n in ast.walk(driver) if isinstance(n,ast.Call)]
  writer_line=next(line for name,line in calls if name=='writer')
  self.assertTrue(any(name=='no_fts_rebuild' and line<writer_line for name,line in calls));self.assertTrue(any(name=='full_vector_digest' and line<writer_line for name,line in calls))
  self.assertTrue(any(name=='full_vector_digest' and line>writer_line for name,line in calls));self.assertTrue(any(name=='a.audit.snapshot' and line>writer_line for name,line in calls))
  for name,line in calls:
   if name=='a.audit.snapshot':self.assertGreater(line,writer_line)

if __name__=='__main__':unittest.main(failfast=True)
