"""Real supervisor with fake Popen/process/clock; no exec, signal, proc or network."""
from pathlib import Path
from unittest.mock import patch
import copy,hashlib,json,os,tempfile,unittest
import grant_watch_controller as ctrl

class FakeWorld:
 def __init__(self,scenario):
  self.tmp=tempfile.TemporaryDirectory();self.root=Path(self.tmp.name);self.q=self.root/'Q';self.q.mkdir();self.now=50.;self.scenario=scenario;self.actions=[];self.children=[];self.driver=None
  self.cfgpath=self.root/'watcher-config.json';script=self.q/'grant_watch_actor.py';script.write_bytes(Path(ctrl.actor.__file__).read_bytes())
  policy=self.q/'policy.py';policy.write_text('def preflight(config): pass\ndef publish_ready(*args): return {}\n')
  prepared=self.q/'prepared.json';prepared.write_text(json.dumps(dict(pilot='f'*40,files={'grant_watch_actor.py':self.sha(script)})))
  authority=self.root/'authority.json';authority.write_text(json.dumps(dict(etat='actif',pid=123,debut='42',cgroup='/fixture',echeance=800.,script_sha256='e'*64)))
  self.cfg=dict(clock='CLOCK_MONOTONIC@luciepc',pilot='f'*40,allocation='C7-fixture',prepared_path=str(prepared),prepared_sha256=self.sha(prepared),actor_nonce='a'*32,ready_nonce='b'*32,issued_monotonic=50.,arm_deadline_monotonic=55.,common_end_monotonic=800.,controller=dict(pid=321,start='31'),authority=dict(path=str(authority),pid=123,start='42',uid=os.getuid(),cgroup='/fixture',script_sha256='e'*64),actor=dict(script_path=str(script),script_sha256=self.sha(script),exe='/fixture/python',exe_sha256='c'*64,argv_prefix=['/fixture/python','-B',str(script),'--watch',str(self.cfgpath)]),paths={k:str((self.q if k=='grant' else self.root)/n) for k,n in dict(ready='READY.json',grant='GRANT.json',armed='watcher-armed.json',heartbeat='watcher-heartbeat.json',grant_ack='watcher-grant.json',result='watcher-result.json',release='watcher-release.json',first_red='first-red.json',evidence='owner-evidence.json').items()},policy=dict(path=str(policy),sha256=self.sha(policy)),policy_context={})
  self.cfgpath.write_text(json.dumps(self.cfg,allow_nan=False));self.cfgsha=self.sha(self.cfgpath)
  self.snapshot=dict(pid=456,ppid=321,start='77',uid=os.getuid(),cgroup='/fixture',exe='/fixture/python',exe_sha256='c'*64,argv=self.cfg['actor']['argv_prefix']+[self.cfgsha],script_path=str(script),script_sha256=self.sha(script),state='S',observed_monotonic=50.)
  self.driver_argv=['/fixture/python','-B','/fixture/driver.py','--driver']
 @staticmethod
 def sha(path):return hashlib.sha256(path.read_bytes()).hexdigest()
 def path(self,key):return Path(self.cfg['paths'][key])
 def write(self,key,value):self.path(key).write_text(json.dumps(value))
 def armed(self):
  receipt=dict(kind='ARMED',config_sha256=self.cfgsha,actor_nonce=self.cfg['actor_nonce'],ready_nonce=self.cfg['ready_nonce'],pilot=self.cfg['pilot'],allocation=self.cfg['allocation'],common_end_monotonic=800.,process=copy.deepcopy(self.snapshot),loop_counter=1,loop_observed_monotonic=self.now)
  hb=dict(kind='HEARTBEAT',config_sha256=self.cfgsha,actor_nonce=self.cfg['actor_nonce'],pid=456,start='77',loop_counter=1,phase='WAIT_READY',observed_monotonic=self.now)
  if self.scenario=='no-loop':receipt['loop_counter']=0;hb['loop_counter']=0
  self.write('heartbeat',hb);self.write('armed',receipt)
 def process(self,pid):
  if pid==321 and hasattr(self,'controller_process'):return dict(self.controller_process,observed_monotonic=self.now)
  value=copy.deepcopy(self.snapshot);value['observed_monotonic']=self.now
  if pid==321:value.update(pid=321,start='31',ppid=123)
  elif pid==123:value.update(pid=123,start='42',ppid=1,script_sha256='e'*64)
  elif self.scenario=='wrong-argv':value['argv']=['/fixture/python','-c','pass']
  elif self.scenario=='gone-after-armed' and self.driver is not None:raise ProcessLookupError('fixture actor gone')
  return value
 def sleep(self,delay):self.now+=delay
 def spawn(self,argv,**kwargs):
  world=self
  class Child:
   def __init__(self,is_actor):self.args=argv;self.pid=456 if is_actor else 789;self.returncode=None;self.is_actor=is_actor
   def poll(self):
    if self.is_actor:
     if world.scenario in ['announced-exited','refused']:self.returncode=0 if world.scenario=='announced-exited' else 1
     elif world.scenario=='gone-after-armed' and world.driver is not None:self.returncode=0
     elif world.path('release').exists():
      world.write('result',dict(kind='RELEASED',config_sha256=world.cfgsha,actor_nonce=world.cfg['actor_nonce'],ready_nonce=world.cfg['ready_nonce'],pilot=world.cfg['pilot'],allocation=world.cfg['allocation'],pid=456,start='77',observed_monotonic=world.now));self.returncode=0
    else:
     world.actions.extend(['READY','HEALTH','WRITER']);self.returncode=0
    return self.returncode
   def wait(self,timeout=None):return self.poll()
   def terminate(self):self.returncode=-15
   def kill(self):self.returncode=-9
  is_actor='--watch' in argv;child=Child(is_actor);self.children.append(child)
  if is_actor:
   self.cfg=json.loads(self.cfgpath.read_text());self.cfgsha=self.sha(self.cfgpath);self.snapshot.update(ppid=self.cfg['controller']['pid'],argv=self.cfg['actor']['argv_prefix']+[self.cfgsha],script_path=self.cfg['actor']['script_path'],script_sha256=self.cfg['actor']['script_sha256'])
   if self.scenario!='never-armed':self.armed()
   if self.scenario=='refused':self.write('first_red',dict(reason='actor fixture refusal',observed=self.now));self.original_red=self.path('first_red').read_bytes()
  else:self.driver=child;self.actions.append('DB')
  return child
 def run(self,entry=None):
  with patch.object(ctrl.subprocess,'Popen',side_effect=self.spawn),patch.object(ctrl.actor,'process_snapshot',side_effect=self.process),patch.object(ctrl.actor,'basic_process',side_effect=self.process),patch.object(ctrl.time,'monotonic',side_effect=lambda:self.now),patch.object(ctrl.time,'sleep',side_effect=self.sleep),patch.object(ctrl.os,'getpid',return_value=321):
   return entry() if entry is not None else ctrl.supervise(self.cfgpath,self.cfgsha,self.driver_argv)

class GrantWatchController(unittest.TestCase):
 def world(self,scenario):
  world=FakeWorld(scenario);self.addCleanup(world.tmp.cleanup);return world
 def test_never_armed_has_no_db_or_ready(self):
  w=self.world('never-armed');self.assertNotEqual(w.run(),0);self.assertEqual(w.actions,[]);self.assertTrue(w.path('first_red').exists())
 def test_announcement_with_exit_even_zero_does_not_start_db(self):
  w=self.world('announced-exited');self.assertNotEqual(w.run(),0);self.assertEqual(w.actions,[])
 def test_wrong_exec_argv_does_not_start_db(self):
  w=self.world('wrong-argv');self.assertNotEqual(w.run(),0);self.assertEqual(w.actions,[])
 def test_descriptor_without_live_loop_does_not_start_db(self):
  w=self.world('no-loop');self.assertNotEqual(w.run(),0);self.assertEqual(w.actions,[])
 def test_actor_disappears_after_armed_before_ready(self):
  w=self.world('gone-after-armed');self.assertNotEqual(w.run(),0);self.assertNotIn('READY',w.actions);self.assertNotIn('HEALTH',w.actions);self.assertNotIn('WRITER',w.actions)
 def test_actor_refusal_keeps_first_red_immutable(self):
  w=self.world('refused');self.assertNotEqual(w.run(),0);self.assertEqual(w.actions,[]);self.assertEqual(w.path('first_red').read_bytes(),w.original_red)
 def test_valid_loop_is_armed_before_db_then_released_after_driver(self):
  w=self.world('valid');self.assertEqual(w.run(),0);self.assertEqual(w.actions,['DB','READY','HEALTH','WRITER']);self.assertTrue(w.path('release').exists());self.assertFalse(w.path('first_red').exists());self.assertTrue(all(c.returncode==0 for c in w.children))

if __name__=='__main__':unittest.main()
