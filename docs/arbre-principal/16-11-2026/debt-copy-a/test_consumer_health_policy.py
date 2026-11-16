"""Actual cycle worker gate with owned temp artifacts, fake phase and process."""
from pathlib import Path
from unittest.mock import patch
import copy,hashlib,json,os,tempfile,unittest
import lifecycle_health_policy as policy
from test_ready_consumption import fixture

class ConsumerHealthPolicy(unittest.TestCase):
 def setUp(self):
  self.tmp=tempfile.TemporaryDirectory();self.addCleanup(self.tmp.cleanup);root=Path(self.tmp.name)
  self.ready,self.phase=fixture();self.ready.update(no_embedding_calls_authorized=True,expected_embedding_calculations=0);self.phase['snapshot_observed_monotonic']=150.
  self.process=dict(pid=456,uid=os.getuid(),start='77',exe='/fixture/provider',exe_sha256='d'*64)
  hashes={}
  for name in ['lifecycle_protocol.py','lifecycle_grant_producer.py','lifecycle_provider_observation.py','provider_health_transport.py']:
   (root/name).write_bytes(b'fixture');hashes[name]=hashlib.sha256(b'fixture').hexdigest()
  (root/'prepared.json').write_text(json.dumps(dict(pilot='f'*40,files=hashes)))
  self.pins=dict(protocol_sha256=hashes['lifecycle_protocol.py'],producer_sha256=hashes['lifecycle_grant_producer.py'],observation_sha256=hashes['lifecycle_provider_observation.py'],transport_sha256=hashes['provider_health_transport.py'],pilot='f'*40,allocation='fixture',expected_process=self.process,identity={'fixture':'identity'})
  self.grant=dict(owner_pins=self.pins,owner_pins_sha256=policy.canonical_sha(self.pins),identity=self.pins['identity'],pid=456,start='77',exe_sha256='d'*64,nonce=self.ready['nonce'],ready_sha256='a'*64,no_embedding_calls_authorized=True,observed_monotonic=98.,valid_until_monotonic=590.)
  self.context=dict(address='127.0.0.1:7878',contact_deadline=154.,admission=False,expected_process=self.process,grant=self.grant,ready_path='/fixture/READY',ready_sha256='a'*64,nonce=self.ready['nonce'])
  self.stack=[]
  for target,name,value in [(policy,'__file__',str(root/'lifecycle_health_policy.py')),(policy.owner,'_pins',lambda pins:None),(policy.producer,'_ready',lambda *args:self.ready),(policy.protocol,'read_phase_state',lambda ready:self.phase),(policy.time,'monotonic',lambda:150.),(policy.owner,'process_snapshot',lambda pid:dict(self.process,state='S'))]:
   p=patch.object(target,name,value);p.start();self.addCleanup(p.stop)
 def test_timely_admitted_lease_survives_closed_admission_without_reopening(self):
  policy.worker_gate(self.context)
  with self.assertRaises(ValueError):policy.worker_gate(dict(self.context,admission=True))
 def test_binding_lease_and_no_embedding_remain_strict_after_admission(self):
  for key,value in [('nonce','foreign'),('ready_sha256','b'*64),('no_embedding_calls_authorized',False),('valid_until_monotonic',149.),('observed_monotonic',101.)]:
   with self.subTest(key=key),self.assertRaises(ValueError):policy.worker_gate(dict(self.context,grant=dict(self.grant,**{key:value})))
 def test_dead_phase_first_red_and_pid_reuse_refuse(self):
  self.phase['authority']['etat']='termine'
  with self.assertRaises(ValueError):policy.worker_gate(self.context)
  self.phase['authority']['etat']='actif';self.phase['first_red_exists']=True
  with self.assertRaises(ValueError):policy.worker_gate(self.context)
  self.phase['first_red_exists']=False
  with patch.object(policy.owner,'process_snapshot',return_value=dict(self.process,start='reused',state='S')),self.assertRaises(ValueError):policy.worker_gate(self.context)
 def test_extended_contact_deadline_and_foreign_owner_pins_refuse(self):
  with self.assertRaises(ValueError):policy.worker_gate(dict(self.context,contact_deadline=591.))
  broken=copy.deepcopy(self.grant);broken['owner_pins']['allocation']='foreign'
  with self.assertRaises(ValueError):policy.worker_gate(dict(self.context,grant=broken))

if __name__=='__main__':unittest.main()
