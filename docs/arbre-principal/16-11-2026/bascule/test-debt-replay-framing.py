from pathlib import Path
import importlib.util,unittest
spec=importlib.util.spec_from_file_location('replay',Path(__file__).with_name('diagnose-debt-replay.py'));m=importlib.util.module_from_spec(spec);spec.loader.exec_module(m)
class Framing(unittest.TestCase):
 def test_exact_body_including_multiline_trailing_newlines_and_protocol_text(self):
  for body in [b'',b'abc',b'a|b\n',b'\nEND_QUERY\t1\nROW\tx\n',b'\r\n\n', 'é\n'.encode()]:
   with self.subTest(length=len(body)):self.assertEqual(m.one_body(b'BEGIN_QUERY\t0\nROW\t'+body+b'\nEND_QUERY\t1\n'),body)
 def test_zero_or_multiple_rows_or_truncated_framing_refused(self):
  for raw in [b'BEGIN_QUERY\t0\nEND_QUERY\t0\n',b'BEGIN_QUERY\t0\nROW\ta\nROW\tb\nEND_QUERY\t2\n',b'BEGIN_QUERY\t0\nROW\ta\n',b'ROW\ta\nEND_QUERY\t1\n']:
   with self.subTest(length=len(raw)),self.assertRaises(ValueError):m.one_body(raw)
if __name__=='__main__':unittest.main(failfast=True)
