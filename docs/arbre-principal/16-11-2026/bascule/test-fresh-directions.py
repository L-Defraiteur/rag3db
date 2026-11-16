"""The actual 18-table gate exercised without native queries or services."""
from pathlib import Path
import importlib.util,unittest
spec=importlib.util.spec_from_file_location('directions',Path(__file__).with_name('check-fresh-directions.py'));m=importlib.util.module_from_spec(spec);spec.loader.exec_module(m)
def observation(n=18):
 return ''.join(f'Table_{i} : 1 étiquettes sources, 1 destinations ; lectures avant 3, arrière 3 — égales\n    comptes : avant 3, arrière 3, union 3, perdues d’un seul sens 0\n' for i in range(n))+'Aucun écart entre les deux rangements.\n'
class CompleteDirections(unittest.TestCase):
 def test_complete_eighteen(self):self.assertEqual(len(m.parse_directions(observation())),18)
 def test_missing_table_refused(self):
  with self.assertRaises(ValueError):m.parse_directions(observation(17))
 def test_duplicate_table_refused(self):
  with self.assertRaises(ValueError):m.parse_directions(observation().replace('Table_17 :','Table_0 :'))
 def test_nonzero_lost_refused(self):
  with self.assertRaises(ValueError):m.parse_directions(observation().replace('seul sens 0','seul sens 1',1))
 def test_inconsistent_counts_refused(self):
  with self.assertRaises(ValueError):m.parse_directions(observation().replace('union 3','union 4',1))
 def test_raw_vs_distinct_mismatch_refused(self):
  with self.assertRaises(ValueError):m.parse_directions(observation().replace('lectures avant 3','lectures avant 4',1))
 def test_missing_final_verdict_refused(self):
  with self.assertRaises(ValueError):m.parse_directions(observation().split('Aucun écart')[0])
if __name__=='__main__':unittest.main(failfast=True)
