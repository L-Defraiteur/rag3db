"""Real READY publication with synthetic producer states; no process or network."""
from pathlib import Path
from types import SimpleNamespace
from unittest.mock import patch
import copy,importlib,json,tempfile,unittest
import lifecycle as c
from test_ready_consumption import fixture

class ProducerBeforeReady(unittest.TestCase):
 def setUp(self):
  self.tmp=tempfile.TemporaryDirectory();self.addCleanup(self.tmp.cleanup);self.root=Path(self.tmp.name)
  self.ready,self.phase=fixture();self.phase['snapshot_observed_monotonic']=50.
  self.ready['phase']['first_red_path']=str(self.root/'first-red.json')
  self.snapshot=dict(pid=456,ppid=321,start='77',uid=1000,cgroup='/fixture',exe='/fixture/python',exe_sha256='a'*64,argv=['/fixture/python','-B','/fixture/actor.py','--watch','/fixture/config','b'*64],script_path='/fixture/actor.py',script_sha256='c'*64,state='S',observed_monotonic=50.)
 def publish(self,scenario):
  state=copy.deepcopy(self.snapshot)
  if scenario!='never-armed':
   (self.root/'watcher-armed.json').write_text(json.dumps(dict(kind='GRANT_WATCH_ARMED',process=self.snapshot,actor_nonce='fixture',loop_counter=1,loop_observed_monotonic=49.)))
  if scenario=='exited-announcement':state['state']='Z'
  if scenario=='wrong-argv':state['argv']=['/fixture/python','-c','pass']
  if scenario=='no-loop':
   value=json.loads((self.root/'watcher-armed.json').read_text());value['loop_counter']=0;(self.root/'watcher-armed.json').write_text(json.dumps(value))
  if scenario=='refused':(self.root/'first-red.json').write_bytes(b'first immutable refusal')
  try:actor=importlib.import_module('grant_watch_actor')
  except ModuleNotFoundError:actor=SimpleNamespace(process_snapshot=lambda pid:state)
  def observe(pid):
   if scenario=='gone-after-armed':raise ProcessLookupError('fixture producer exited')
   return copy.deepcopy(state)
  with patch.object(c,'P',self.root),patch.object(c.a,'P',self.root),patch.object(c.q,'P',self.root),patch.object(c.time,'monotonic',return_value=50.),patch.object(actor,'process_snapshot',side_effect=observe),patch.object(c.q,'process_identity',side_effect=observe):
   c.publish_ready(self.ready)
 def refuses(self,scenario):
  with self.assertRaises((ValueError,ProcessLookupError)):self.publish(scenario)
  self.assertFalse((self.root/'READY_FOR_PROVIDER_COORDINATION.json').exists())
 def test_never_armed_cannot_publish_ready(self):self.refuses('never-armed')
 def test_announced_alive_but_exited_cannot_publish_ready(self):self.refuses('exited-announcement')
 def test_delayed_exec_with_wrong_argv_cannot_publish_ready(self):self.refuses('wrong-argv')
 def test_process_without_live_loop_cannot_publish_ready(self):self.refuses('no-loop')
 def test_exit_after_armed_cannot_publish_ready(self):self.refuses('gone-after-armed')
 def test_refusal_before_ready_is_immutable_and_blocks_publication(self):
  self.refuses('refused');self.assertEqual((self.root/'first-red.json').read_bytes(),b'first immutable refusal')

if __name__=='__main__':unittest.main()
