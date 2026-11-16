"""Pure boundary checks: actual summaries/save/watchers, no runtime/Cargo/proc."""
from pathlib import Path
import tempfile,unittest
from unittest.mock import patch
import qualify as q

class Contracts(unittest.TestCase):
 def test_single_summary_rejects_empty_multiple_zero_and_failed(self):
  good='test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 200 filtered out; finished in 0s\n'
  self.assertEqual(q.library_summary(good)['passed'],7)
  for bad in ['',good+good,good.replace('7 passed','0 passed'),good.replace('0 failed','1 failed')]:
   with self.subTest(bad=bad),self.assertRaises(AssertionError):q.library_summary(bad)
 def test_save_refuses_expiry_after_file_open_before_green_publication(self):
  with tempfile.TemporaryDirectory(dir=Path.home()/'.cache/rag3weaver-build') as tmp:
   root=Path(tmp)
   with patch.object(q.time,'monotonic',return_value=101),self.assertRaises(RuntimeError):q.save(root,'result.json',dict(status='GREEN'),end=100)
   self.assertEqual((root/'result.json').read_bytes(),b'')
 def test_historical_cached_failure_is_not_an_active_log(self):
  with tempfile.TemporaryDirectory(dir=Path.home()/'.cache/rag3weaver-build') as tmp:
   root=Path(tmp);(root/'cached-template').mkdir();(root/'cached-template/lib.log').write_text('test old ... FAILED\n')
   with patch.object(q,'P',root):q.watch_logs('official-pass')
   self.assertFalse((root/'first-red.json').exists())
 def test_active_failure_creates_immutable_first_red_and_snapshot(self):
  with tempfile.TemporaryDirectory(dir=Path.home()/'.cache/rag3weaver-build') as tmp:
   root=Path(tmp);(root/'official').mkdir();(root/'official/lib.log').write_text('test current ... FAILED\n')
   with patch.object(q,'P',root),self.assertRaises(AssertionError):q.watch_logs('official-pass')
   original=(root/'first-red.json').read_bytes()
   with patch.object(q,'P',root):q.first_red('later reason')
   self.assertEqual((root/'first-red.json').read_bytes(),original);self.assertTrue((root/'watcher-first-match.json').is_file())

if __name__=='__main__':unittest.main(failfast=True)
