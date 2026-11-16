from pathlib import Path
import importlib.util,unittest
spec=importlib.util.spec_from_file_location('compare_ids',Path(__file__).with_name('compare-live-ids.py'));m=importlib.util.module_from_spec(spec);spec.loader.exec_module(m)
class CompleteSets(unittest.TestCase):
 def test_same_count_different_ids_fails_both_directions(self):
  r=m.compare([1,3],[1,2]);self.assertFalse(r['equal']);self.assertEqual(r['orphans'],[3]);self.assertEqual(r['missing'],[2])
 def test_equal_sets_with_different_order(self):self.assertTrue(m.compare([7,1],[1,7])['equal'])
 def test_empty_is_complete_only_when_both_empty(self):
  self.assertTrue(m.compare([],[])['equal']);self.assertFalse(m.compare([],[1])['equal'])
 def test_duplicate_on_either_side_refused(self):
  for a,b in [([1,1],[1]),([1],[1,1])]:
   with self.assertRaises(ValueError):m.compare(a,b)
 def test_invalid_types_or_range_refused(self):
  for value in [True,'1',1.0,-1,1<<64,None]:
   with self.assertRaises(ValueError):m.compare([value],[])
 def test_truncated_fts_refused(self):
  with self.assertRaises(ValueError):m.parse_fts('IMMUTABLE_SNAPSHOT\tshards=4\tdocuments=2\tbytes=4\tcommit=REFUSED\nLIVE_ID\t1\n')
 def test_graph_error_or_missing_trailer_refused(self):
  for text in ['BEGIN_QUERY\t0\nROW\t1','BEGIN_QUERY\t0\nQUERY_ERROR nope\nEND_QUERY\t1','BEGIN_QUERY\t0\nROW\t1\nEND_QUERY\t2']:
   with self.assertRaises(ValueError):m.parse_graph(text)
 def test_real_protocol_decodes_without_silent_dedup(self):
  self.assertEqual(m.parse_graph('BEGIN_QUERY\t0\nROW\t41\nROW\t77\nEND_QUERY\t2\n'),[41,77])
  with self.assertRaises(ValueError):m.parse_fts('IMMUTABLE_SNAPSHOT\tshards=4\tdocuments=2\tbytes=4\tcommit=REFUSED\nLIVE_ID\t1\nLIVE_ID\t1\n')
if __name__=='__main__':unittest.main(failfast=True)
