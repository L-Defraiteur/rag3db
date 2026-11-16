"""Synthetic native/snapshot output validation, no native execution."""
import unittest
from observations import native_queries,snapshot_ids,compare_ids
class Observations(unittest.TestCase):
 def test_complete_native_rows_are_required(self):
  self.assertEqual(native_queries('BEGIN_QUERY\t0\nROW\t1\nROW\t2\nEND_QUERY\t2\n',1),[['1','2']])
  for text in ['BEGIN_QUERY\t0\nROW\t1\nEND_QUERY\t2\n','BEGIN_QUERY\t1\nEND_QUERY\t0\n','BEGIN_QUERY\t0\nROW\t1\n']:
   with self.subTest(text=text),self.assertRaises(ValueError):native_queries(text,1)
 def test_live_ids_need_refused_commit_and_exact_unique_count(self):
  valid='IMMUTABLE_SNAPSHOT\tshards=1\tdocuments=2\tbytes=99\tcommit=REFUSED\nLIVE_ID\t1\nLIVE_ID\t9000000\n'
  self.assertEqual(snapshot_ids(valid),[1,9000000])
  for bad in [valid.replace('REFUSED','ACCEPTED'),valid+'LIVE_ID\t1\n',valid.replace('documents=2','documents=3'),valid.replace('9000000','-1')]:
   with self.subTest(bad=bad),self.assertRaises(ValueError):snapshot_ids(bad)
 def test_id_comparison_retains_both_differences(self):
  self.assertEqual(compare_ids(['1','2'],[1,9000000]),dict(graph_count=2,fts_count=2,fts_without_row=[9000000],rows_without_fts=[2]))
  for graph in [['1','1'],['-1'],['null']]:
   with self.subTest(graph=graph),self.assertRaises(ValueError):compare_ids(graph,[])
if __name__=='__main__':unittest.main()
