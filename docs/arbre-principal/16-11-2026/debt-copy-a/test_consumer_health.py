"""Real q.health with fake HTTP/clock/identity, no network or process access."""
from pathlib import Path
from types import SimpleNamespace
from unittest.mock import patch
import json,tempfile,unittest
import qualify as q

IDENTITY=dict(service='rag3weaver-embeddings',modele='granite-278m',dim=768,factice=False,dual=False,sparse=False,precision='Flex32',executable='/fixture/embeddings',max_chars_per_text=12000)

class ConsumerHealth(unittest.TestCase):
 def invoke(self,raw,status=200,elapsed=0.):
  clock=[10.];root=self.root
  class Response:
   def __enter__(self):return self
   def __exit__(self,*args):pass
   def read(self,limit):clock[0]+=elapsed;return raw
  response=Response();response.status=status
  with patch.object(q,'P',root),patch.object(q,'provider_identity'),patch.object(q.time,'monotonic',side_effect=lambda:clock[0]),patch.object(q.urllib.request,'urlopen',return_value=response):
   return q.health(dict(identity=IDENTITY),'health',100.)
 def setUp(self):
  self.tmp=tempfile.TemporaryDirectory();self.addCleanup(self.tmp.cleanup);self.root=Path(self.tmp.name)
 def raw(self,**changes):return json.dumps(dict(IDENTITY,usage=dict(baux=0,en_cours=0),**changes)).encode()
 def test_usage_false_is_not_integer_zero(self):
  data=dict(IDENTITY,usage=dict(baux=False,en_cours=False))
  with self.assertRaises(ValueError):self.invoke(json.dumps(data).encode())
 def test_dimension_float_is_not_integer_identity(self):
  with self.assertRaises(ValueError):self.invoke(self.raw(dim=768.0))
 def test_whole_response_exceeding_five_seconds_is_refused(self):
  with self.assertRaises(ValueError):self.invoke(self.raw(),elapsed=6.)
 def test_invalid_json_preserves_original_raw_before_refusal(self):
  raw=b'{invalid-json'
  with self.assertRaises(ValueError):self.invoke(raw)
  self.assertTrue(any(raw in p.read_bytes() or raw.hex().encode() in p.read_bytes() for p in self.root.iterdir()),'original raw absent after JSON refusal')
 def test_non200_preserves_status_and_body_before_refusal(self):
  raw=b'health unavailable'
  with self.assertRaises(ValueError):self.invoke(raw,status=503)
  content=b'\n'.join(p.read_bytes() for p in self.root.iterdir())
  self.assertIn(b'503',content);self.assertTrue(raw in content or raw.hex().encode() in content)
 def test_oversized_body_preserves_bounded_raw_before_refusal(self):
  raw=b'x'*1048577
  with self.assertRaises(ValueError):self.invoke(raw)
  self.assertTrue(any(p.stat().st_size>0 for p in self.root.iterdir()),'no evidence for oversized response')

if __name__=='__main__':unittest.main()
