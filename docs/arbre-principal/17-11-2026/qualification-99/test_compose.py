"""Actual compose and atomic publication; fake Common/process/files only.

Does not qualify real poste, process cleanup, Cargo, readers or SDK runtime.
"""
from pathlib import Path as RealPath
from types import SimpleNamespace
from unittest.mock import patch
import hashlib,json,os,tempfile,unittest
import launch as l
import build_private_cli as driver

class Composition(unittest.TestCase):
 def execute(self,mode):
  with tempfile.TemporaryDirectory() as d:
   root=RealPath(d);q=root/'Q';q.mkdir();p=root/'P';s=root/'S';t=root/'target';app=root/'app';scripts=app/'extension/rag3weaver/scripts';scripts.mkdir(parents=True)
   (scripts/'confiner_validation.py').write_text('def poste_tenu(table): return True\ndef processus(): return {}\n')
   now=[99.];allocation='fixture';cfg=dict(source_root=str(app),allocation=allocation,timeout='/fixture/timeout',launcher_sha256=hashlib.sha256(RealPath(l.__file__).read_bytes()).hexdigest())
   (q/'prepared.json').write_text(json.dumps(cfg));config_sha=hashlib.sha256((q/'prepared.json').read_bytes()).hexdigest()
   proc=root/'proc/123';proc.mkdir(parents=True)
   argv=[cfg['timeout'],'--signal=TERM','--kill-after=2s','2398s','/usr/bin/python3.14','-B',str(q/'launch.py'),'--compose',config_sha,'--allocation',allocation]
   (proc/'cmdline').write_bytes(b'\0'.join(os.fsencode(x) for x in argv)+b'\0')
   def path(value):return root/'proc' if str(value)=='/proc' else RealPath(value)
   def run(*args,**kwargs):
    p.mkdir();scope=s/'validation-fixture';scope.mkdir(parents=True)
    authority=dict(etat='termine',pid=11,debut='7',cgroup='/fixture',echeance=100.)
    verdict=dict(verdict='VERT',code=0,restes=[],nettoye=True,prive=str(root/'bus'))
    if mode=='authority-active':authority['etat']='actif'
    if mode=='residue':verdict['restes']=[{'pid':999}]
    if mode=='bus-present':(root/'bus').mkdir()
    (scope/'autorite.json').write_text(json.dumps(authority));(scope/'verdict.json').write_text(json.dumps(verdict))
    (p/'authority-reference.json').write_text(json.dumps(dict(path=str(scope/'autorite.json'),pid=11,start='7',cgroup='/fixture',deadline=100.)))
    result=dict(status='BUILT_AND_STATIC_PAIR_CHECKED_ONLY',app=l.APP,common_end=100.,SDK_runtime_qualified=False,backend_invoked=False,DB_opened=False,provider_contact=False,reader_DB_qualified=False,live_authorized=False)
    if mode=='scope-widened':result['backend_invoked']=True
    (p/'result.json').write_text(json.dumps(result));artifacts={}
    for name in [*driver.NAMES,'typed-reader','ids','query']:
     origin=p/(name+'.origin');copy=p/(name+'.copy');origin.write_bytes(name.encode());copy.write_bytes(origin.read_bytes());artifacts[name]=dict(origin=str(origin),path=str(copy),sha256=hashlib.sha256(origin.read_bytes()).hexdigest())
    (p/'ledger-before-first-invocation.json').write_text(json.dumps(dict(app=l.APP,features=driver.FEATURES,config_sha256=config_sha,artifacts=artifacts)))
    if mode=='artifact-changed':RealPath(artifacts['typed-reader']['path']).write_bytes(b'changed')
    return SimpleNamespace(returncode=0)
   real_sync=os.fsync;fired=[]
   def fsync(fd):
    real_sync(fd)
    if list(q.glob('.composed.json-*')) and not fired:
     fired.append(True)
     if mode=='deadline-during-final-fsync':now[0]=100.1
     if mode=='first-red-during-final-fsync':(p/'first-red.json').write_bytes(b'{"reason":"original refusal"}')
   with patch.object(l,'P',p),patch.object(l,'Q',q),patch.object(l,'S',s),patch.object(l,'T',t),patch.object(l,'Path',side_effect=path),patch.object(l,'guard'),patch.object(l.os,'getppid',return_value=123),patch.object(l.os,'readlink',return_value=cfg['timeout']),patch.object(l.subprocess,'run',side_effect=run),patch.object(l.time,'monotonic',side_effect=lambda:now[0]),patch.object(os,'fsync',side_effect=fsync):
    rc=l.compose(config_sha,allocation)
   result=json.loads((q/'composed.json').read_text());red=(p/'first-red.json').read_bytes() if (p/'first-red.json').exists() else None
   return rc,result,red
 def test_static_only_compose_requires_clean_terminated_scope(self):
  rc,result,red=self.execute('valid');self.assertEqual(rc,0);self.assertEqual(result['status'],'BUILD_STATIC_PAIR_OBSERVED');self.assertIsNone(red)
 def test_active_authority_refuses(self):self.assertNotEqual(self.execute('authority-active')[0],0)
 def test_owned_residue_refuses(self):self.assertNotEqual(self.execute('residue')[0],0)
 def test_remaining_bus_refuses(self):self.assertNotEqual(self.execute('bus-present')[0],0)
 def test_changed_artifact_after_common_refuses(self):self.assertNotEqual(self.execute('artifact-changed')[0],0)
 def test_runtime_claim_outside_static_scope_refuses(self):self.assertNotEqual(self.execute('scope-widened')[0],0)
 def test_fsync_crossing_end_cannot_leave_positive_composed(self):
  rc,result,_=self.execute('deadline-during-final-fsync');self.assertNotEqual(rc,0);self.assertEqual(result['status'],'RED')
 def test_first_red_is_preserved_during_final_publication(self):
  rc,result,red=self.execute('first-red-during-final-fsync');self.assertNotEqual(rc,0);self.assertEqual(result['status'],'RED');self.assertEqual(red,b'{"reason":"original refusal"}')

if __name__=='__main__':unittest.main()
