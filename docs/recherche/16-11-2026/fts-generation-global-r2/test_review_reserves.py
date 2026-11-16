"""Actual consumer expressions, fake providers/clocks only; no network or process."""
import ast,contextlib,json,tempfile,unittest
from pathlib import Path
from unittest.mock import patch
import qualify as q

class ReviewReserves(unittest.TestCase):
 def body(self):
  return next(n for n in ast.parse(Path(q.__file__).read_text()).body if isinstance(n,ast.FunctionDef) and n.name=='driver').body
 def test_actual_check_features_suffix_is_accepted(self):
  assertion=next(n for n in self.body() if isinstance(n,ast.Assert) and 'check_features.log' in ast.unparse(n))
  with tempfile.TemporaryDirectory() as d:
   official=Path(d);(official/'check_features.log').write_text('CHECK_FEATURES VERT 8/8\n')
   exec(compile(ast.Module(body=[assertion],type_ignores=[]),'<actual-features-assert>','exec'),dict(vars(q),official=official))
 def test_first_refusal_after_first_get_forbids_second_contact(self):
  call=next(n for n in self.body() if isinstance(n,ast.Expr) and isinstance(n.value,ast.Call) and ast.unparse(n.value.func)=='health' and 'services-before' in ast.unparse(n))
  contacts=[];uid=q.os.getuid()
  ready=dict(clock='CLOCK_MONOTONIC@luciepc',ready_observed_monotonic=40.,grant_deadline_monotonic=100.,deadline=600.,nonce='fake',phase=dict(job='lifecycle-provider',pilot='a'*40,allocation='fake',authority=dict(path='/fake',pid=123,start='42',cgroup='/fake',uid=uid,script_sha256='a'*64),first_red_path='/fake/red'))
  def phase(_):
   return dict(clock='CLOCK_MONOTONIC@luciepc',snapshot_stable=True,first_red_exists=bool(contacts),snapshot_observed_monotonic=70.,authority=dict(etat='actif',pid=123,debut='42',cgroup='/fake',script_sha256='a'*64,echeance=600.),authority_uid=uid,process=dict(start='42',cgroup='/fake',uid=uid,state='S'))
  grant=dict(valid_until_monotonic=600.,providers=[dict(address='127.0.0.1:7878',identity=dict(model='first')),dict(address='127.0.0.1:7879',identity=dict(model='second'))])
  def observe(ready,pins,provider,evidence,**kwargs):
   contacts.append(provider['address']);evidence['transport']=dict(returncode=0,stdout_hex='7b7d')
   return dict(provider['identity'],usage=dict(baux=0,en_cours=0))
  pins=dict(providers=grant['providers'])
  with tempfile.TemporaryDirectory() as d,patch.object(q,'P',Path(d)),patch.object(q.time,'monotonic',return_value=70.),patch.object(q,'owner_pins',return_value=pins),patch.object(q.ready_protocol,'read_phase_state',side_effect=phase),patch.object(q.global_owner,'health_observation',side_effect=observe):
   with self.assertRaises(ValueError):exec(compile(ast.Module(body=[call],type_ignores=[]),'<actual-services-before>','exec'),dict(vars(q),grant=grant,end=600.,ready=ready))
   self.assertEqual(json.loads((Path(d)/'services-before-7878.json').read_text())['transport']['returncode'],0)
  self.assertEqual(len(contacts),1)
 def test_feature_suffix_mutants_are_not_success(self):
  assertion=next(n for n in self.body() if isinstance(n,ast.Assert) and 'check_features.log' in ast.unparse(n))
  for tail in ['CHECK_FEATURES VERT','CHECK_FEATURES VERT 7/8','CHECK_FEATURES VERT 9/9','CHECK_FEATURES VERT 0/0','CHECK_FEATURES VERT 8/8\nROUGE']:
   with self.subTest(tail=tail),tempfile.TemporaryDirectory() as d:
    official=Path(d);(official/'check_features.log').write_text(tail+'\n')
    with self.assertRaises(AssertionError):exec(compile(ast.Module(body=[assertion],type_ignores=[]),'<actual-features-assert>','exec'),dict(vars(q),official=official))
 def test_consumer_preserves_transport_timeout_and_never_contacts_next(self):
  import subprocess
  pins=dict(providers=[dict(address='127.0.0.1:7878'),dict(address='127.0.0.1:7879')]);grant=dict(valid_until_monotonic=500.);error=subprocess.TimeoutExpired('fake',5)
  def timeout(ready,pins,provider,evidence,**kwargs):
   evidence.update(transport=dict(timed_out=True,returncode=-9,stdout_hex='7061727469616c',stderr_hex='657272'))
   raise error
  with tempfile.TemporaryDirectory() as d,patch.object(q,'P',Path(d)),patch.object(q,'owner_pins',return_value=pins),patch.object(q.ready_protocol,'read_phase_state',return_value={}),patch.object(q.ready_protocol,'validate_ready_phase'),patch.object(q.time,'monotonic',return_value=70.),patch.object(q.global_owner,'health_observation',side_effect=timeout) as contact:
   with self.assertRaises(subprocess.TimeoutExpired) as caught:q.health(grant,'fake',600.,ready=dict(deadline=600.),admission=True)
   self.assertIs(caught.exception,error);self.assertEqual(contact.call_count,1)
   evidence=json.loads((Path(d)/'fake-7878.json').read_text());self.assertTrue(evidence['transport']['timed_out']);self.assertEqual(evidence['transport']['returncode'],-9)
   self.assertFalse((Path(d)/'fake-7879.json').exists())
 def test_after_canonical_uses_active_lease_without_new_ready_admission(self):
  pins=dict(providers=[dict(address='127.0.0.1:7878'),dict(address='127.0.0.1:7879')]);grant=dict(valid_until_monotonic=500.);seen=[]
  def observe(ready,pins,provider,evidence,**kwargs):seen.append(kwargs);return {}
  with tempfile.TemporaryDirectory() as d,patch.object(q,'P',Path(d)),patch.object(q,'owner_pins',return_value=pins),patch.object(q.ready_protocol,'read_phase_state',return_value={}),patch.object(q.ready_protocol,'validate_ready_phase',side_effect=AssertionError('must not re-admit')),patch.object(q.ready_protocol,'validate_phase_authority') as core,patch.object(q.time,'monotonic',return_value=150.),patch.object(q.global_owner,'health_observation',side_effect=observe):
   self.assertEqual(q.health(grant,'after',600.,ready=dict(deadline=600.,grant_deadline_monotonic=100.),admission=False),[{},{}])
   self.assertEqual(core.call_count,4)
  self.assertEqual(seen,[dict(admission=False,lease_end=500.)]*2)
 def test_owner_pins_bind_admission_clock_and_every_source(self):
  import copy,hashlib
  with tempfile.TemporaryDirectory() as d:
   root=Path(d);P=root/'P';Q=root/'Q';P.mkdir();Q.mkdir()
   ready=dict(app=q.APP,pilot='a'*40,scope='/scope',nonce='fake',c7_reference='go')
   path=P/'READY_FOR_SERVICES.json';path.write_text(json.dumps(ready))
   sources={}
   for key,name in [('protocol_sha256','lifecycle_protocol.py'),('process_module_sha256','lifecycle_provider_observation.py'),('owner_sha256','global_services_owner.py'),('pg_receipt_sha256','pg_receipt.py'),('pg_identity_sql_sha256','identity.sql'),('pg_postcheck_sql_sha256','backend-postcheck.sql')]:
    f=Q/name;f.write_bytes(('fake source '+name).encode());sources[key]=q.sha(f)
   providers=[]
   for address in ['127.0.0.1:7878','127.0.0.1:7879']:
    providers.append(dict(address=address,identity=dict(fake=True),process=dict(pid=123,uid=q.os.getuid(),start='42',exe='/fake',exe_sha256='b'*64)))
   pins=dict(ready,ready_path=str(path),ready_sha256=q.sha(path),providers=providers,admitted_monotonic=80.,**sources)
   grant=dict(providers=[dict(address=p['address'],identity=p['identity'],**p['process']) for p in providers],observed_monotonic=80.)
   def bind(value):return dict(grant,owner_pins=value,owner_pins_sha256=hashlib.sha256(json.dumps(value,sort_keys=True,separators=(',',':'),allow_nan=False).encode()).hexdigest())
   with patch.object(q,'P',P),patch.object(q,'Q',Q):
    self.assertEqual(q.owner_pins(bind(pins),ready),pins)
    variants=[]
    for key,value in [('admitted_monotonic',79.),('admitted_monotonic',True),('pg_receipt_sha256','c'*64),('pg_identity_sql_sha256','c'*64),('pg_postcheck_sql_sha256','c'*64),('owner_sha256','c'*64),('nonce','foreign'),('pilot','c'*40),('scope','/foreign'),('c7_reference','other')]:
     item=copy.deepcopy(pins);item[key]=value;variants.append(item)
    for item in variants:
     with self.subTest(item=item),self.assertRaises(AssertionError):q.owner_pins(bind(item),ready)
    tampered=bind(pins);tampered['owner_pins_sha256']='0'*64
    with self.assertRaises(AssertionError):q.owner_pins(tampered,ready)

if __name__=='__main__':unittest.main()
