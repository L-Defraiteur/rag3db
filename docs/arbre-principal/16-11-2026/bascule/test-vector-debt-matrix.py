from pathlib import Path
import importlib.util,unittest
spec=importlib.util.spec_from_file_location('matrix',Path(__file__).with_name('vector-debt-matrix.py'));m=importlib.util.module_from_spec(spec);spec.loader.exec_module(m)
def levels(rows):return {name:list(rows) for name in m.LEVELS}
class CompleteDebtMatrix(unittest.TestCase):
 def test_three_categories_do_not_collapse_to_counts(self):
  universe=[('n1','v1','a'),('n1','v1','b'),('n2','v2','a'),('n2','v2','b'),('n3','v3','c')]
  c=levels([('n1','a',1),('n2','b',1)])
  result=m.build_matrix(['n1','n2','n3'],universe,c,3)
  self.assertEqual(result['audit']['categories'],dict(COMPATIBLE_AT_SELECTED_REPRESENTATIVE=1,COMPATIBLE_ONLY_AT_OTHER_OCCURRENCE=1,NO_COMPATIBLE_CANDIDATE=1));self.assertTrue(result['audit']['complete'])
 def test_missing_parent_is_explicit_unknown(self):
  result=m.build_matrix(['n'],[],levels([]),1);self.assertFalse(result['audit']['complete']);self.assertEqual(result['audit']['unknown'],1)
 def test_duplicate_debt_or_occurrence_refused(self):
  with self.assertRaises(ValueError):m.build_matrix(['n','n'],[],levels([]),2)
  with self.assertRaises(ValueError):m.build_matrix(['n'],[('n','v','s'),('n','v','s')],levels([]),1)
 def test_foreign_occurrence_candidate_refused(self):
  with self.assertRaises(ValueError):m.build_matrix(['n'],[('n','v','s')],levels([('n','foreign',1)]),1)
 def test_nonmonotone_levels_refused(self):
  c=levels([]);c['valid']=[('n','s',1)]
  with self.assertRaises(ValueError):m.build_matrix(['n'],[('n','v','s')],c,1)
 def test_text_hash_and_literal_text_remain_distinct(self):
  c=levels([('n','s',1)]);c['text']=[];result=m.build_matrix(['n'],[('n','v','s')],c,1);self.assertEqual(result['audit']['literal_text_mismatch'],1)
if __name__=='__main__':unittest.main(failfast=True)
