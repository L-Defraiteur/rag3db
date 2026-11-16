"""Actual READY publication/consumer AST, temp files and fake clock/phase; no RPC."""
from pathlib import Path
import ast,json,os,tempfile,unittest
from unittest.mock import patch
import lifecycle as c

class WouldContactProvider(RuntimeError):pass
def fixture():
 uid=os.getuid();authority=dict(path='/fixture/authority',pid=123,start='42',cgroup='/fixture',uid=uid,script_sha256='a'*64)
 identity=dict(job='lifecycle-provider',pilot='f'*40,allocation='fixture',authority=authority,first_red_path='/fixture/first-red.json')
 ready=dict(clock='CLOCK_MONOTONIC@luciepc',nonce='fixture-nonce',deadline=600.,ready_observed_monotonic=40.,grant_deadline_monotonic=100.,phase=identity)
 phase=dict(clock='CLOCK_MONOTONIC@luciepc',authority=dict(etat='actif',pid=123,debut='42',cgroup='/fixture',echeance=600.,script_sha256='a'*64),authority_uid=uid,process=dict(start='42',cgroup='/fixture',uid=uid,state='S'),first_red_exists=False,snapshot_stable=True,snapshot_observed_monotonic=99.)
 return ready,phase
def driver_body():
 return next(n for n in ast.parse(Path(c.__file__).read_text()).body if isinstance(n,ast.FunctionDef) and n.name=='driver').body

class ReadyConsumption(unittest.TestCase):
 def test_actual_ready_publication_contains_observation_and_absolute_bound(self):
  body=driver_body();assignment=next(n for n in body if isinstance(n,ast.Assign) and any(isinstance(t,ast.Name) and t.id=='ready' for t in n.targets));publication=next(n for n in body if isinstance(n,ast.Expr) and isinstance(n.value,ast.Call) and ast.unparse(n.value.func)=='publish_ready')
  ready,_=fixture();published=[]
  with tempfile.TemporaryDirectory() as d:
   root=Path(d);p=root/'P';p.mkdir();q=root/'Q';q.mkdir()
   for name in ['lifecycle.json','lifecycle.mmd']:(q/name).write_text('{}')
   (p/'vectors-bits-before.json').write_text('{}')
   namespace=dict(vars(c),P=p,Q=q,nonce=ready['nonce'],end=600.,phase_identity=ready['phase'])
   with patch.object(c,'P',p),patch.object(c.time,'monotonic',return_value=40.):exec(compile(ast.Module(body=[assignment,publication],type_ignores=[]),'<actual-ready-publication>','exec'),namespace)
   published.append(json.loads((p/'READY_FOR_PROVIDER_COORDINATION.json').read_text()))
  self.assertEqual(published[0]['ready_observed_monotonic'],40.);self.assertEqual(published[0]['grant_deadline_monotonic'],100.)
 def consume(self,now,terminated=False,refused=False):
  body=driver_body();start=next(i for i,n in enumerate(body) if isinstance(n,ast.While) and 'PROVIDER_GO.json' in ast.unparse(n.test));stop=next(i for i in range(start,len(body)) if isinstance(body[i],ast.Expr) and isinstance(body[i].value,ast.Call) and ast.unparse(body[i].value.func)=='q.health')
  ready,phase=fixture()
  if terminated:phase['authority']['etat']='termine'
  if refused:phase['first_red_exists']=True
  with tempfile.TemporaryDirectory() as d:
   root=Path(d);p=root/'P';p.mkdir();q=root/'Q';q.mkdir();(p/'READY_FOR_PROVIDER_COORDINATION.json').write_text(json.dumps(ready))
   grant=dict(nonce=ready['nonce'],ready_sha256=c.q.sha(p/'READY_FOR_PROVIDER_COORDINATION.json'),no_embedding_calls_authorized=True,observed_monotonic=now,valid_until_monotonic=590.)
   (q/'PROVIDER_GO.json').write_text(json.dumps(grant))
   namespace=dict(vars(c),P=p,Q=q,ready=ready,nonce=ready['nonce'],pause=100.,end=600.,save=lambda *args:None)
   with patch.object(c.time,'monotonic',return_value=now),patch.object(c.q,'validate_grant'),patch.object(c.protocol,'read_phase_state',return_value=phase,create=True),patch.object(c.q,'health',side_effect=WouldContactProvider('provider action reached')):
    exec(compile(ast.Module(body=body[start:stop+1],type_ignores=[]),'<actual-ready-consumer>','exec'),namespace)
 def test_late_grant_already_present_cannot_skip_deadline_and_contact_provider(self):
  with self.assertRaisesRegex(ValueError,'deadline'):self.consume(100.1)
 def test_timely_present_grant_can_reach_first_action(self):
  with self.assertRaises(WouldContactProvider):self.consume(99.)
 def test_terminated_authority_or_first_red_forbids_first_action(self):
  for options in [dict(terminated=True),dict(refused=True)]:
   with self.subTest(options=options),self.assertRaises(ValueError):self.consume(99.,**options)
 def test_atomic_ready_never_replaces_or_publishes_after_delayed_write(self):
  ready,_=fixture()
  with tempfile.TemporaryDirectory() as d:
   root=Path(d)
   with patch.object(c,'P',root),patch.object(c.time,'monotonic',return_value=40.):c.publish_ready(ready)
   public=root/'READY_FOR_PROVIDER_COORDINATION.json';original=public.read_bytes()
   self.assertEqual(public.stat().st_ino,(root/'READY_FOR_PROVIDER_COORDINATION.pending.json').stat().st_ino)
   with patch.object(c,'P',root),self.assertRaises(FileExistsError):c.publish_ready(dict(ready,nonce='foreign'))
   self.assertEqual(public.read_bytes(),original)
  with tempfile.TemporaryDirectory() as d:
   root=Path(d)
   with patch.object(c,'P',root),patch.object(c.time,'monotonic',return_value=100.1),self.assertRaises(ValueError):c.publish_ready(ready)
   self.assertFalse((root/'READY_FOR_PROVIDER_COORDINATION.json').exists());self.assertTrue((root/'READY_FOR_PROVIDER_COORDINATION.pending.json').exists())

if __name__=='__main__':unittest.main(failfast=True)
