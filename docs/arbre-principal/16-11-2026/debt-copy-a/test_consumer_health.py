"""Real q.health with fake HTTP/clock/identity, no network or process access."""
from pathlib import Path
from types import SimpleNamespace
from unittest.mock import patch
import ast,importlib.util,json,tempfile,unittest
import lifecycle as c
from test_ready_consumption import fixture,driver_body
from test_producer_fixture import producer_state
import qualify as q

IDENTITY=dict(service='rag3weaver-embeddings',modele='granite-278m',dim=768,factice=False,dual=False,sparse=False,precision='Flex32',executable='/fixture/embeddings',max_chars_per_text=12000)

class ConsumerHealth(unittest.TestCase):
 def invoke(self,raw,status=200,elapsed=0.,publication_elapsed=0.):
  clock=[10.];root=self.root
  transport=self.transport()
  def receive(context,trace):
   clock[0]+=elapsed
   trace.update(returncode=0,stdout_hex=raw.hex())
   return dict(status=status,body=raw)
  policy=SimpleNamespace(worker_gate=lambda context:None)
  original_save=q.save
  original_validate=transport.validate_health
  def validate(*args,**kwargs):
   self.assertTrue((root/'health.transport.json').exists(),'transport evidence must precede validation')
   return original_validate(*args,**kwargs)
  def persist(path,value):
   original_save(path,value)
   if path.name=='health.json':clock[0]+=publication_elapsed
  with patch.object(transport,'validate_health',side_effect=validate),patch.object(q,'save',side_effect=persist),patch.object(q,'P',root),patch.object(q.time,'monotonic',side_effect=lambda:clock[0]),patch.object(q,'loadmod',side_effect=lambda name:policy if name=='lifecycle_health_policy' else transport),patch.object(transport,'run_health_worker',side_effect=receive):
   return q.health(dict(identity=IDENTITY,valid_until_monotonic=100.),'health',100.,context={})
 def transport(self):
  path=Path(__file__).parents[3]/'orchestration/16-11-2026/coordination/provider_health_transport.py'
  spec=importlib.util.spec_from_file_location('consumer_test_shared_transport',path);module=importlib.util.module_from_spec(spec);spec.loader.exec_module(module);return module
 def setUp(self):
  self.tmp=tempfile.TemporaryDirectory();self.addCleanup(self.tmp.cleanup);self.root=Path(self.tmp.name)
 def raw(self,**changes):return json.dumps(dict(IDENTITY,usage=dict(baux=0,en_cours=0),**changes)).encode()
 def test_valid_typed_observation_publishes_transport_before_result(self):
  self.assertEqual(self.invoke(self.raw())['dim'],768)
  self.assertTrue((self.root/'health.transport.json').exists())
  self.assertTrue((self.root/'health.json').exists())
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

 def test_result_publication_cannot_extend_effective_deadline(self):
  with self.assertRaises(ValueError):self.invoke(self.raw(),publication_elapsed=6.)
  self.assertTrue((self.root/'health.transport.json').exists())
 def test_actual_post_writer_raccord_rechecks_phase_before_get(self):
  body=driver_body()
  start=next(i for i,n in enumerate(body) if isinstance(n,ast.Expr) and isinstance(n.value,ast.Call) and ast.unparse(n.value.func)=='writer')
  stop=next(i for i in range(start,len(body)) if isinstance(body[i],ast.Expr) and isinstance(body[i].value,ast.Call) and ast.unparse(body[i].value.func)=='q.health')
  ready,phase=fixture();phase['authority']['etat']='termine';phase['snapshot_observed_monotonic']=150.;phase['producer_watcher']=producer_state(ready,150.)
  grant=dict(identity=IDENTITY,valid_until_monotonic=590.,nonce=ready['nonce'],ready_sha256='a'*64,no_embedding_calls_authorized=True,observed_monotonic=98.)
  def gate(context):
   self.assertIs(context['admission'],False)
   c.protocol.validate_admitted_grant_phase(ready,grant,150.,phase,'a'*64)
  transport=self.transport();policy=SimpleNamespace(worker_gate=gate)
  namespace=dict(vars(c),ready=ready,grant=grant,ready_sha256='a'*64,end=600.,cfg={},env={},profile={},nonce='fixture',writer=lambda *a:None,health_context=lambda ready,grant,sha,admission:dict(admission=admission))
  with patch.object(q,'P',self.root),patch.object(q.time,'monotonic',return_value=150.),patch.object(q,'loadmod',side_effect=lambda name:policy if name=='lifecycle_health_policy' else transport),patch.object(transport,'run_health_worker') as contact:
   with self.assertRaises(ValueError):exec(compile(ast.Module(body=body[start:stop+1],type_ignores=[]),'<actual-post-writer-health>','exec'),namespace)
   contact.assert_not_called()

if __name__=='__main__':unittest.main()
