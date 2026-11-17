"""Real READY/action guard with complete synthetic OS/loop artifacts; no process."""
from pathlib import Path
from unittest.mock import patch
import copy,json,os,unittest
import lifecycle as c
from test_grant_watch_controller import FakeWorld

class ProducerBeforeReady(unittest.TestCase):
 def setUp(self):
  self.w=FakeWorld('valid');self.addCleanup(self.w.tmp.cleanup);self.root=self.w.root;self.q=self.w.q
  self.w.cfg['paths']['ready']=str(self.root/'READY_FOR_PROVIDER_COORDINATION.json');self.w.cfg['paths']['grant']=str(self.q/'PROVIDER_GO.json')
  source=self.q/'lifecycle.py';source.write_bytes(Path(c.__file__).read_bytes())
  prepared=dict(pilot='f'*40,python='/fixture/python',python_sha256='c'*64,files={'grant_watch_actor.py':self.w.cfg['actor']['script_sha256'],'lifecycle.py':self.w.sha(source)})
  self.prepared=prepared;(self.q/'prepared.json').write_text(json.dumps(prepared));(self.q/'allocation.json').write_text(json.dumps(dict(reference=self.w.cfg['allocation'])))
  self.w.cfg['prepared_sha256']=self.w.sha(self.q/'prepared.json');self.w.cfgpath.write_text(json.dumps(self.w.cfg));self.w.cfgsha=self.w.sha(self.w.cfgpath);self.w.snapshot['argv']=self.w.cfg['actor']['argv_prefix']+[self.w.cfgsha]
  self.authority=dict(pid=123,debut='42',cgroup='/fixture',script_sha256='e'*64,echeance=800.)
  self.scenario='valid'
 def process(self,pid):
  if pid==os.getpid():
   return dict(self.w.snapshot,pid=pid,ppid=321,argv=['/fixture/python','-B',str(self.q/'lifecycle.py'),'--driver'],script_path=str(self.q/'lifecycle.py'),script_sha256=self.prepared['files']['lifecycle.py'],observed_monotonic=self.w.now)
  if pid==456:
   if self.scenario=='gone-after-armed':raise ProcessLookupError('fixture producer exited')
   value=self.w.process(pid)
   if self.scenario=='exited-announcement':value['state']='Z'
   if self.scenario=='wrong-argv':value['argv']=['/fixture/python','-c','pass']
   return value
  return self.w.process(pid)
 def admit(self):
  self.w.armed();receipt=self.w.path('armed')
  admission=dict(kind='INDEPENDENT_PRODUCER_ADMISSION',config_path=str(self.w.cfgpath),config_sha256=self.w.cfgsha,armed_path=str(receipt),armed_sha256=self.w.sha(receipt),expected_pid=456,process=copy.deepcopy(self.w.snapshot),actor_nonce=self.w.cfg['actor_nonce'],ready_nonce=self.w.cfg['ready_nonce'],pilot=self.w.cfg['pilot'],allocation=self.w.cfg['allocation'],prepared_sha256=self.w.cfg['prepared_sha256'])
  (self.root/'watcher-admission.json').write_text(json.dumps(admission))
  return c.install_action_guard(self.prepared,self.authority)
 def publish(self,scenario):
  with patch.object(c,'P',self.root),patch.object(c,'Q',self.q),patch.object(c.a,'P',self.root),patch.object(c.q,'P',self.root),patch.object(c.q,'ACTION_GUARD',None),patch.object(c.time,'monotonic',side_effect=lambda:self.w.now),patch.object(c.watch_actor,'process_snapshot',side_effect=self.process),patch.object(c.watch_actor,'python_role_snapshot',side_effect=lambda pid,*a:self.process(pid)),patch.dict(c.os.environ,{'RAG3WEAVER_VALIDATION_RECU':self.w.cfg['authority']['path']}):
   if scenario=='never-armed':
    c.publish_ready({})
    return
   state=self.admit();self.w.now=70.;hb=json.loads(self.w.path('heartbeat').read_text());hb.update(loop_counter=2,observed_monotonic=self.w.now);self.w.write('heartbeat',hb)
   ready=dict(c.protocol.ready_bounds(70.,800.),deadline=800.,nonce=self.w.cfg['ready_nonce'],phase=dict(job='lifecycle-provider',pilot=self.w.cfg['pilot'],allocation=self.w.cfg['allocation'],authority=self.w.cfg['authority'],first_red_path=str(self.w.path('first_red')),producer_watcher=c.producer_descriptor(state)),no_embedding_calls_authorized=True,expected_embedding_calculations=0,health_required=True,max_chars=0,manifest_sha256='a'*64,graph_sha256='b'*64,fixture_vector_digest_sha256='c'*64)
   self.scenario=scenario
   if scenario=='no-loop':hb['loop_counter']=0;self.w.write('heartbeat',hb)
   if scenario=='refused':self.w.path('first_red').write_bytes(b'first immutable refusal')
   c.publish_ready(ready)
 def refuses(self,scenario):
  with self.assertRaises((ValueError,ProcessLookupError,RuntimeError)):self.publish(scenario)
  self.assertFalse(self.w.path('ready').exists())
 def test_never_armed_cannot_publish_ready(self):self.refuses('never-armed')
 def test_announced_alive_but_exited_cannot_publish_ready(self):self.refuses('exited-announcement')
 def test_delayed_exec_with_wrong_argv_cannot_publish_ready(self):self.refuses('wrong-argv')
 def test_process_without_live_loop_cannot_publish_ready(self):self.refuses('no-loop')
 def test_exit_after_armed_cannot_publish_ready(self):self.refuses('gone-after-armed')
 def test_refusal_before_ready_is_immutable_and_blocks_publication(self):
  self.refuses('refused');self.assertEqual(self.w.path('first_red').read_bytes(),b'first immutable refusal')
 def test_actual_live_loop_admission_reaches_ready_without_provider_or_writer(self):
  self.publish('valid');self.assertTrue(self.w.path('ready').exists());self.assertEqual(self.w.actions,[]);self.assertFalse(self.w.path('first_red').exists())

if __name__=='__main__':unittest.main()
