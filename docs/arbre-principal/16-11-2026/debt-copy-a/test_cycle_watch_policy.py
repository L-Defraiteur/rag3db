"""Cycle policy's real static preflight/delegation; fake Git, no service action."""
from pathlib import Path
from unittest.mock import patch
import json,tempfile,unittest
import lifecycle_health_policy as p
from test_ready_consumption import fixture

class CycleWatchPolicy(unittest.TestCase):
 def setUp(self):
  self.tmp=tempfile.TemporaryDirectory();self.addCleanup(self.tmp.cleanup);self.root=Path(self.tmp.name);self.q=self.root/'Q';self.q.mkdir()
  prefix=Path(p.__file__).parent;profile=json.loads((prefix/'lifecycle-provider-expectations.json').read_text());origin=self.root/'expected-origin.json';origin.write_text('{"fixture":"expected only"}');profile['provenance']['path']=str(origin);profile['provenance']['sha256']=p.owner.digest(origin)
  for name,source in [('lifecycle_protocol.py',Path(p.protocol.__file__)),('lifecycle_grant_producer.py',Path(p.producer.__file__)),('lifecycle_provider_observation.py',Path(p.owner.__file__)),('provider_health_transport.py',prefix.parents[2]/'orchestration/16-11-2026/coordination/provider_health_transport.py')]:
   (self.q/name).write_bytes(source.read_bytes())
  (self.q/'lifecycle-provider-expectations.json').write_text(json.dumps(profile));names=[x.name for x in self.q.iterdir()]
  self.prepared=dict(pilot='f'*40,source_root='/fixture/source',files={n:p.owner.digest(self.q/n) for n in names});(self.q/'prepared.json').write_text(json.dumps(self.prepared));(self.q/'allocation.json').write_text(json.dumps(dict(reference='fixture')))
  self.ready,_=fixture();self.ready.update(no_embedding_calls_authorized=True,expected_embedding_calculations=0);self.ready_path=self.root/'READY.json';self.ready_path.write_text(json.dumps(self.ready))
  self.config=dict(prepared_path=str(self.q/'prepared.json'),prepared_sha256=p.owner.digest(self.q/'prepared.json'),pilot='f'*40,allocation='fixture',actor_nonce='a'*32,ready_nonce=self.ready['nonce'],paths=dict(ready=str(self.ready_path),grant=str(self.q/'PROVIDER_GO.json'),evidence=str(self.root/'evidence.json')),policy_context=dict(expected_provider_path=str(self.q/'lifecycle-provider-expectations.json'),expected_provider_sha256=self.prepared['files']['lifecycle-provider-expectations.json']))
  location=patch.object(p.owner,'__file__',str(self.q/'lifecycle_provider_observation.py'));location.start();self.addCleanup(location.stop)
 def git(self,args,**kwargs):return 'f'*40+'\n' if 'rev-parse' in args else ''
 def test_static_preflight_has_no_process_probe_health_or_helper(self):
  with patch.object(p.subprocess,'check_output',side_effect=self.git),patch.object(p.subprocess,'Popen') as spawn,patch.object(p.owner,'process_snapshot') as process,patch.object(p.owner,'publish_observed_grant') as helper:
   self.assertIsNone(p.preflight(self.config))
  spawn.assert_not_called();process.assert_not_called();helper.assert_not_called()
 def test_changed_head_or_profile_refuses_before_any_helper(self):
  with patch.object(p.subprocess,'check_output',return_value='other'),patch.object(p.owner,'publish_observed_grant') as helper,self.assertRaises(ValueError):p.preflight(self.config)
  helper.assert_not_called()
  self.config['policy_context']['expected_provider_sha256']='0'*64
  with patch.object(p.subprocess,'check_output',side_effect=self.git),patch.object(p.owner,'publish_observed_grant') as helper,self.assertRaises(ValueError):p.preflight(self.config)
  helper.assert_not_called()
 def test_real_publish_hook_uses_exact_owner_with_preapproved_pins(self):
  with patch.object(p.subprocess,'check_output',side_effect=self.git),patch.object(p.owner,'process_snapshot') as process,patch.object(p.owner,'publish_observed_grant',return_value={'fixture':True}) as helper:
   self.assertEqual(p.publish_ready(self.config,self.ready_path,p.owner.digest(self.ready_path),self.ready['nonce']),{'fixture':True})
  process.assert_not_called();self.assertEqual(helper.call_args.kwargs['pins']['protocol_sha256'],self.prepared['files']['lifecycle_protocol.py']);self.assertEqual(helper.call_args.kwargs['pins']['allocation'],'fixture');self.assertIs(helper.call_args.kwargs['pins']['identity']['factice'],False)
 def test_wrong_ready_nonce_never_calls_owner(self):
  with patch.object(p.owner,'publish_observed_grant') as helper,self.assertRaises(ValueError):p.publish_ready(self.config,self.ready_path,p.owner.digest(self.ready_path),'foreign')
  helper.assert_not_called()

if __name__=='__main__':unittest.main()
