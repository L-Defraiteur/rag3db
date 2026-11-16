"""Real guard parsing against hermetic management stdout and reply fixtures."""
from pathlib import Path
import copy,importlib.util,unittest
spec=importlib.util.spec_from_file_location('cold_guard',Path(__file__).with_name('cold-service-guard.py'));g=importlib.util.module_from_spec(spec);spec.loader.exec_module(g)
RAW_SERVICE='LoadState=loaded\nActiveState=inactive\nSubState=dead\nMainPID=0\n';RAW_SOCKET='LoadState=loaded\nActiveState=inactive\nSubState=dead\n'
def fixture():
 req=dict(stage='service-guard-corrective-02',nonce='a'*32,phase='versions',entered_monotonic=10.,requested_monotonic=11.,runner=dict(pid=1,start='2',cgroup='/validation'),common_receipt=dict(sha256='b'*64),expected_host=dict(pid=3,exe_path='/fixed/backend',exe_sha256=g.CLI_SHA,source_sha=g.SOURCE_SHA))
 reply=dict(schema_version=1,verdict='VERT',stage=req['stage'],nonce=req['nonce'],phase=req['phase'],entered_monotonic=req['entered_monotonic'],runner=req['runner'],request_sha256='c'*64,common_receipt_sha256='b'*64,observed_monotonic=12.,units={g.SERVICE:g.parse_unit_stdout(RAW_SERVICE,g.SERVICE),g.SOCKET:g.parse_unit_stdout(RAW_SOCKET,g.SOCKET)},previous_host=dict(pid=3,absent=True),executable=dict(path='/fixed/backend',sha256=g.CLI_SHA,source_sha=g.SOURCE_SHA),port_7970=dict(rc=0,listeners=[]),base_refs=dict(fd=[],maps=[]),protected_processes=[],raw_files=[dict(kind='units_service',path='/fixture/service',sha256='d'*64)])
 return req,reply,dict(units_service=RAW_SERVICE,units_socket=RAW_SOCKET)
class ColdServiceGuard(unittest.TestCase):
 def test_bound_reply_and_real_stdout_parse(self):
  req,reply,raw=fixture();self.assertIs(g.validate_reply(req,reply,'c'*64,13.,raw),reply)
 def test_empty_or_error_stdout_never_green(self):
  for text in ['', 'Failed to get properties: no service files\n']:
   with self.assertRaises(ValueError):g.parse_unit_stdout(text,g.SERVICE)
 def test_duplicate_stdout_fields_refused(self):
  with self.assertRaises(ValueError):g.parse_unit_stdout(RAW_SERVICE+'MainPID=0\n',g.SERVICE)
 def test_active_unit_or_nonzero_pid_refused(self):
  for text in [RAW_SERVICE.replace('inactive','active'),RAW_SERVICE.replace('MainPID=0','MainPID=9')]:
   with self.assertRaises(ValueError):g.parse_unit_stdout(text,g.SERVICE)
 def test_nonce_or_receipt_mismatch_refused(self):
  for key in ['nonce','common_receipt_sha256','request_sha256']:
   req,reply,raw=fixture();reply[key]='wrong'
   with self.assertRaises(ValueError):g.validate_reply(req,reply,'c'*64,13.,raw)
 def test_stale_future_or_nonfinite_clock_refused(self):
  for observed,now in [(12.,43.),(14.,13.),(float('nan'),13.)]:
   req,reply,raw=fixture();reply['observed_monotonic']=observed
   with self.assertRaises(ValueError):g.validate_reply(req,reply,'c'*64,now,raw)
 def test_changed_runner_or_source_refused(self):
  req,reply,raw=fixture();reply['runner']=dict(req['runner'],start='changed')
  with self.assertRaises(ValueError):g.validate_reply(req,reply,'c'*64,13.,raw)
  req,reply,raw=fixture();reply['executable']['source_sha']='wrong'
  with self.assertRaises(ValueError):g.validate_reply(req,reply,'c'*64,13.,raw)
 def test_json_green_cannot_override_raw_stdout(self):
  req,reply,raw=fixture();raw['units_service']=RAW_SERVICE.replace('MainPID=0','MainPID=1')
  with self.assertRaises(ValueError):g.validate_reply(req,reply,'c'*64,13.,raw)
 def test_base_reference_or_listener_refused(self):
  for section in ['base_refs','port_7970']:
   req,reply,raw=fixture()
   if section=='base_refs':reply[section]['fd']=[dict(pid=9)]
   else:reply[section]['listeners']=['127.0.0.1:7970']
   with self.assertRaises(ValueError):g.validate_reply(req,reply,'c'*64,13.,raw)
 def test_matching_host_and_unrelated_test_host(self):
  self.assertTrue(g.main_host('/fixed/rag3weaver-backend',['backend','daemon']))
  self.assertTrue(g.main_host('/fixed/rag3weaver-backend',['backend','daemon','--address','127.0.0.1:7970']))
  self.assertFalse(g.main_host('/fixed/rag3weaver-backend',['backend','daemon','--address','127.0.0.1:7991']))
 def test_fd_and_deleted_fd_preserve_base_scope(self):
  base=Path('/data/code.rag3db')
  self.assertTrue(g.refers_to_base('/data/code.rag3db.wal',[base]));self.assertTrue(g.refers_to_base('/data/code.rag3db (deleted)',[base]));self.assertFalse(g.refers_to_base('/private/copy/code.rag3db',[base]))
 def test_tcp_listener_is_observed_without_bus(self):
  header='sl local_address rem_address st\n'
  self.assertEqual(g.tcp_listeners(header+'0: 0100007F:1F22 00000000:0000 0A\n'),['0100007F:1F22'])
  self.assertEqual(g.tcp_listeners(header+'0: 0100007F:1F22 00000000:0000 01\n'),[])
 def test_raw_cgroup_normalizes_without_changing_scope(self):
  self.assertEqual(g.normalized_cgroup('0::/user.slice/stage'),'/user.slice/stage')
  for value in ['relative','0::/a\n0::/b','/a/../b']:
   with self.assertRaises(ValueError):g.normalized_cgroup(value)
 def test_local_unknown_or_new_host_cannot_green(self):
  obs=dict(previous_pid_exists=False,listeners=[],hosts=[],fd=[],maps=[],errors=[])
  for key in ['hosts','errors','fd','maps','listeners']:
   bad=dict(obs,**{key:['witness']})
   with self.assertRaises(ValueError):g.validate_local(bad)
if __name__=='__main__':unittest.main(failfast=True)
