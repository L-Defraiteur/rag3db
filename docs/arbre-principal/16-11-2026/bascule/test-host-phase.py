"""Pure safety witnesses: no authentication, HTTP, host or database."""
from pathlib import Path
import ast,json,re,unittest
p=Path(__file__).with_name('host-phase.py');tree=ast.parse(p.read_text());names={'maintenance_payload','find_journal'}
functions=[n for n in tree.body if isinstance(n,ast.FunctionDef) and n.name in names];ns=dict(Path=Path,json=json,re=re);exec(compile(ast.Module(body=functions,type_ignores=[]),'host-phase-pure','exec'),ns)
class Safety(unittest.TestCase):
 def test_one_percent_is_exact_and_never_widened(self):
  self.assertEqual(ns['maintenance_payload'](dict(live_docs=100,orphans=[7],token='t'))['max_purge_ratio'],.01)
  with self.assertRaises(AssertionError):ns['maintenance_payload'](dict(live_docs=99,orphans=[7],token='t'))
 def test_empty_index_does_not_invent_an_orphan(self):
  self.assertEqual(ns['maintenance_payload'](dict(live_docs=0,orphans=[],token=''))['purge_ids'],[])
  with self.assertRaises(AssertionError):ns['maintenance_payload'](dict(live_docs=0,orphans=[7],token=''))
 def test_duplicate_or_bool_ids_are_refused(self):
  for ids in [[7,7],[True],[-1]]:
   with self.assertRaises(AssertionError):ns['maintenance_payload'](dict(live_docs=1000,orphans=ids,token='t'))
 def test_missing_token_is_not_a_valid_request(self):
  with self.assertRaises(AssertionError):ns['maintenance_payload'](dict(live_docs=10,orphans=[]))
 def test_busy_is_not_journal_completion(self):
  with self.assertRaises(AssertionError):ns['find_journal'](dict(ok=True,result=dict(busy=False)))
 def test_structured_journal(self):
  self.assertEqual(ns['find_journal'](dict(result=dict(journal='/private/index/1.out'))),Path('/private/index/1.out'))
 def test_embedded_json_presentation_is_decoded(self):
  value=dict(result=dict(content=[dict(type='text',text=json.dumps(dict(presentation='indexation lancée\njournal : /private/index/2.out\nsuite')))]))
  self.assertEqual(ns['find_journal'](value),Path('/private/index/2.out'))
 def test_multiple_or_secret_paths_are_refused(self):
  for value in [[dict(journal='/a.out'),dict(journal='/b.out')],dict(journal='/private/.vault/a.out'),dict(journal='relative.out')]:
   with self.assertRaises(AssertionError):ns['find_journal'](value)
if __name__=='__main__':unittest.main(failfast=True)
