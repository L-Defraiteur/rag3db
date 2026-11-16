"""Actual archived logs, pure classifier only; no subprocess/proc/signal/database."""
from pathlib import Path
import copy,json,unittest
from classifier import classify_classic
F=Path(__file__).parent/'fixtures'
class ClassifierContract(unittest.TestCase):
 def args(self):
  return [(F/'exact-0.stdout').read_text(),(F/'exact-0.stderr').read_text(),json.loads((F/'exact-0.command.json').read_text()),json.loads((F/'exact-0.result.json').read_text()),json.loads((F/'ledger.json').read_text()),(F/'inventory.stdout').read_text()]
 def test_actual_interleaved_classic_log_is_accepted(self):
  self.assertEqual(classify_classic(*self.args())['status'],'CLASSIC_RAW_PASSED_ONLY')
 def test_any_failed_or_panic_even_after_summary_is_refused(self):
  for suffix in ['\nFAILED\n','\ntest x ... FAILED\n','\nthread x panicked at failure\n']:
   for stream in [0,1]:
    args=self.args();args[stream]+=suffix
    with self.subTest(stream=stream,suffix=suffix),self.assertRaises(AssertionError):classify_classic(*args)
 def test_wrong_exact_name_rc_inventory_or_summary_refused(self):
  variants=[]
  a=self.args();a[2]['argv'][-3]='other';variants.append(a)
  a=self.args();a[3]['rc']=1;variants.append(a)
  a=self.args();a[5]+='other: test\n';variants.append(a)
  a=self.args();a[0]=a[0].replace('1 passed; 0 failed; 0 ignored','0 passed; 0 failed; 1 ignored');variants.append(a)
  a=self.args();a[1]=a[1].replace('dispatch:classic','dispatch:unknown');variants.append(a)
  a=self.args();a[0]+='\ntest other ... ok\n';variants.append(a)
  a=self.args();a[0]+='\ntest result: ok. 1 passed; 0 failed; 0 ignored;\n';variants.append(a)
  for a in variants:
   with self.assertRaises(AssertionError):classify_classic(*a)
if __name__=='__main__':unittest.main(failfast=True)
