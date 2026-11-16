"""Shared bounded phase contract and real RO loader with a synthetic proc tree."""
from pathlib import Path as RealPath
import copy,json,os,tempfile,types,unittest
from unittest.mock import patch
import lifecycle_protocol as p
from test_ready_consumption import fixture

class ReadyContract(unittest.TestCase):
 def test_bounds_are_calculated_from_prepublication_observation_once(self):
  self.assertEqual(p.ready_bounds(40.,600.)['grant_deadline_monotonic'],100.)
  self.assertEqual(p.ready_bounds(40.,1000.)['grant_deadline_monotonic'],160.)
  for observed,end in [(100.,600.),(True,1000.),(1.,float('inf')),(float('nan'),1000.),('40',1000.)]:
   with self.subTest(observed=observed,end=end),self.assertRaises(ValueError):p.ready_bounds(observed,end)
 def test_authority_first_red_identity_and_unknown_are_strict(self):
  ready,phase=fixture();p.validate_ready_phase(ready,99.,phase)
  variants=[]
  for key,value in [('first_red_exists',True),('snapshot_stable',False),('snapshot_observed_monotonic',100.)]:
   item=copy.deepcopy(phase);item[key]=value;variants.append(item)
  for section,key,value in [('authority','etat','termine'),('authority','pid',123.),('process','start','reused'),('process','cgroup','/foreign'),('process','state','?'),('process','uid',False)]:
   item=copy.deepcopy(phase);item[section][key]=value;variants.append(item)
  for i,item in enumerate(variants):
   with self.subTest(case=i),self.assertRaises(ValueError):p.validate_ready_phase(ready,99.,item)
 def test_grant_window_nonce_and_lease_are_distinct(self):
  ready,phase=fixture();grant=dict(nonce=ready['nonce'],ready_sha256='a'*64,no_embedding_calls_authorized=True,observed_monotonic=98.,valid_until_monotonic=590.)
  p.validate_grant_phase(ready,grant,99.,phase,'a'*64)
  for key,value in [('nonce','foreign'),('ready_sha256','b'*64),('no_embedding_calls_authorized',False),('observed_monotonic',100.),('observed_monotonic',39.),('valid_until_monotonic',99.),('valid_until_monotonic',float('inf'))]:
   with self.subTest(key=key),self.assertRaises(ValueError):p.validate_grant_phase(ready,dict(grant,**{key:value}),99.,phase,'a'*64)
  with self.assertRaises(ValueError):p.validate_grant_phase(ready,grant,100.,phase,'a'*64)
  # A timely admitted lease can still have an active scope after admission closes.
  later=copy.deepcopy(phase);later['snapshot_observed_monotonic']=150.;p.validate_phase_authority(ready,150.,later)
 def test_real_loader_filters_private_token_and_checks_first_refusal(self):
  ready,_=fixture();uid=os.getuid()
  class Proc:
   def __init__(self,key):self.key=key
   def __truediv__(self,key):return Proc(self.key+'/'+str(key))
   def stat(self):return types.SimpleNamespace(st_uid=uid)
   def read_text(self):
    if self.key.endswith('/stat'):return '123 (fake) '+' '.join(['S']+['0']*18+['42'])
    if self.key.endswith('/cgroup'):return '0::/fixture\n'
    raise AssertionError(self.key)
  with tempfile.TemporaryDirectory() as d:
   root=RealPath(d);authority=root/'authority.json';red=root/'first-red.json';ready['phase']['authority']['path']=str(authority);ready['phase']['first_red_path']=str(red)
   authority.write_text(json.dumps(dict(etat='actif',pid=123,debut='42',cgroup='/fixture',echeance=600.,script_sha256='a'*64,jeton='PRIVATE-FIXTURE-TOKEN')))
   with patch.object(p,'Path',lambda value:Proc('/proc') if str(value)=='/proc' else RealPath(value)),patch.object(p.os,'uname',return_value=types.SimpleNamespace(nodename='luciepc')),patch.object(p.time,'monotonic',return_value=99.):
    observed=p.read_phase_state(ready);self.assertNotIn('PRIVATE-FIXTURE-TOKEN',json.dumps(observed));p.validate_ready_phase(ready,99.,observed)
    red.write_text('immutable refusal');refused=p.read_phase_state(ready)
    with self.assertRaises(ValueError):p.validate_ready_phase(ready,99.,refused)
   self.assertEqual(red.read_text(),'immutable refusal')

if __name__=='__main__':unittest.main(failfast=True)
