"""Hermetic controller witnesses: fake pidfds/process states, no real signals."""
from pathlib import Path
import ast,json,runpy,signal,tempfile,unittest
HERE=Path(__file__).parent
probe=runpy.run_path(str(HERE/'cargo-stop.py'))
PROFILE={k:'expected-'+k for k in ['RAG3DB_LIB','RAG3DB_SHARED','RAG3DB_LIBRARY_DIR','RAG3DB_INCLUDE_DIR','LD_LIBRARY_PATH','RAG3DB_BUFFER_POOL_SIZE','RAG3WEAVER_MOTEUR_ANCIEN']}
EXPECTED=dict(pid=99,starttime='123',uid=42,cgroup='/owned',exe='/cargo',exe_sha256='abc')
class FakeOps:
 def __init__(self,case='happy'):
  self.case=case;self.signals=[];self.t=0.;self.state='T' if case=='already_stopped' else 'R';self.ids=0;self.thread_reads=0;self.snapshots=0;self.closed=False;self.profile=PROFILE
 def open(self,pid):assert pid==99;return 701
 def close(self,fd):assert fd==701;self.closed=True
 def now(self):return self.t
 def sleep(self):self.t+=.1
 def signal(self,fd,sig):
  assert fd==701;self.signals.append((fd,sig))
  if sig==signal.SIGSTOP and self.case!='stop_timeout':self.state='T'
  if sig==signal.SIGCONT:
   if self.case=='cont_failure':raise OSError('CONT failed')
   self.state='R'
 def identity(self,pid):
  self.ids+=1
  if self.case=='cg_unknown':raise PermissionError('cgroup unavailable')
  row=dict(EXPECTED,state=self.state,tracer_pid=17 if self.case=='traced' else 0)
  if self.case=='pid_reuse':row['starttime']='456'
  if self.case=='cg_changed':row['cgroup']='/foreign'
  if self.case=='exec_during_capture' and self.snapshots:row['exe']='/other'
  if self.case=='noncargo_at_stop' and self.signals:row['exe']='/other'
  return row
 def threads(self,pid):
  self.thread_reads+=1;result={'99':dict(starttime='123',state=self.state),'100':dict(starttime='124',state=self.state)}
  if self.case=='thread_unstopped':result['100']['state']='R'
  if self.case=='thread_changed' and self.snapshots:result['101']=dict(starttime='125',state='T')
  return result
 def snapshot(self,pid,keys):
  self.snapshots+=1
  assert self.state=='T','no profile read before effective STOP'
  actual=dict(self.profile);present={k:self.profile[k] is not None for k in keys};length=321
  if self.case in ['raw_empty','noncargo_at_stop']:
   actual={k:None for k in keys};present={k:False for k in keys};length=0
  if self.case=='profile_diff':actual[keys[0]]='wrong'
  return dict(rawlen=length,argv_rawlen=35,argv=['cargo','check','--features',''],actual=actual,present=present)
class CargoStop(unittest.TestCase):
 def run_case(self,case,expect_error=False):
  ops=FakeOps(case)
  with tempfile.TemporaryDirectory(prefix='cx-cargo-stop-fixture-') as tmp:
   p=Path(tmp)/'attempt.json'
   if expect_error:
    with self.assertRaises((AssertionError,PermissionError,OSError)):probe['observe'](99,EXPECTED,PROFILE,p,100.,ops)
   else:probe['observe'](99,EXPECTED,PROFILE,p,100.,ops)
   self.assertTrue(p.exists());row=json.loads(p.read_text());resume=json.loads(p.with_suffix('.resume.json').read_text())
   self.assertTrue(ops.closed)
   return ops,row,resume
 def test_same_pidfd_stop_and_continue_and_coherent_threads(self):
  ops,row,resume=self.run_case('happy');self.assertEqual(ops.signals,[(701,signal.SIGSTOP),(701,signal.SIGCONT)]);self.assertEqual(row['threads_before'],row['threads_after']);self.assertEqual(row['argv'][-1],'');self.assertEqual(row['diff'],{});self.assertTrue(resume['continued'])
 def test_pid_reuse_after_open_is_refused_before_signal(self):
  ops,_,resume=self.run_case('pid_reuse',True);self.assertEqual(ops.signals,[]);self.assertFalse(resume['initiated'])
 def test_unknown_cgroup_is_refused_without_signal(self):
  ops,_,_=self.run_case('cg_unknown',True);self.assertEqual(ops.signals,[])
 def test_foreign_cgroup_is_refused_without_signal(self):
  ops,_,_=self.run_case('cg_changed',True);self.assertEqual(ops.signals,[])
 def test_already_stopped_is_never_resumed(self):
  ops,_,_=self.run_case('already_stopped',True);self.assertEqual(ops.signals,[])
 def test_traced_process_is_never_stopped_or_resumed(self):
  ops,_,_=self.run_case('traced',True);self.assertEqual(ops.signals,[])
 def test_stop_timeout_has_no_profile_and_continues_only_owned_stop(self):
  ops,_,resume=self.run_case('stop_timeout',True);self.assertEqual(ops.snapshots,0);self.assertTrue(resume['continued']);self.assertLess(ops.t,1.2)
 def test_one_running_thread_prevents_profile(self):
  ops,_,resume=self.run_case('thread_unstopped',True);self.assertEqual(ops.snapshots,0);self.assertTrue(resume['continued'])
 def test_thread_set_change_after_capture_is_refused_and_continued(self):
  _,row,resume=self.run_case('thread_changed',True);self.assertNotEqual(row['threads_before'],row['threads_after']);self.assertTrue(resume['continued'])
 def test_exec_change_during_capture_is_refused_and_archived(self):
  _,row,resume=self.run_case('exec_during_capture',True);self.assertNotEqual(row['stopped']['exe'],row['after']['exe']);self.assertTrue(resume['continued'])
 def test_non_cargo_at_effective_stop_is_not_a_valid_cargo_profile(self):
  _,row,resume=self.run_case('noncargo_at_stop');self.assertEqual(row['status'],'NON_CARGO');self.assertEqual(row['rawlen'],0);self.assertTrue(resume['continued'])
 def test_empty_environment_is_archived_before_refusal(self):
  _,row,resume=self.run_case('raw_empty',True);self.assertEqual(row['rawlen'],0);self.assertFalse(any(row['present'].values()));self.assertEqual(len(row['diff']),7);self.assertTrue(resume['continued'])
 def test_profile_difference_is_archived_before_refusal(self):
  _,row,resume=self.run_case('profile_diff',True);self.assertEqual(len(row['diff']),1);self.assertTrue(resume['continued'])
 def test_resume_failure_cannot_return_a_green_capture(self):
  _,_,resume=self.run_case('cont_failure',True);self.assertIn('error',resume)
 def test_real_cpu_profile_requires_six_absent_fields_and_rustup_ld(self):
  tree=ast.parse((HERE/'driver.py').read_text())
  sdk=next(n for n in tree.body if isinstance(n,ast.Assign) and any(isinstance(t,ast.Name) and t.id=='SDK_PROFILE' for t in n.targets))
  rust=next(n for n in tree.body if isinstance(n,ast.Assign) and any(isinstance(t,ast.Name) and t.id=='RUST_LIB' for t in n.targets))
  statements=[n for n in tree.body if isinstance(n,ast.Assign) and any((isinstance(t,ast.Name) and t.id=='METADATA_BEFORE_EXPORT') or (isinstance(t,ast.Subscript) and isinstance(t.value,ast.Name) and t.value.id=='METADATA_BEFORE_EXPORT') for t in n.targets)]
  env=dict(SDK_PROFILE={k.arg:'irrelevant' for k in sdk.value.keywords},RUST_LIB=ast.literal_eval(rust.value))
  exec(compile(ast.Module(body=statements,type_ignores=[]),'real-cpu-profile','exec'),env)
  profile=env['METADATA_BEFORE_EXPORT'];self.assertEqual(sum(v is None for v in profile.values()),6)
  ops=FakeOps();ops.profile=profile
  with tempfile.TemporaryDirectory(prefix='cx-cargo-cpu-fixture-') as tmp:
   row=probe['observe'](99,EXPECTED,profile,Path(tmp)/'attempt.json',100.,ops)
  self.assertEqual(row['status'],'OBSERVED_CARGO');self.assertEqual(row['present'],{k:v is not None for k,v in profile.items()});self.assertEqual(row['diff'],{})
 def test_real_snapshot_preserves_empty_argv_and_publishes_only_selected_keys(self):
  raw=b'RAG3DB_LIB=sdk29\0SECRET_KEY=do-not-publish\0';argv=b'cargo\0check\0--features\0\0'
  class FixturePath:
   def __init__(self,value):self.value=str(value)
   def __truediv__(self,part):return FixturePath(self.value+'/'+str(part))
   def read_bytes(self):return raw if self.value.endswith('/environ') else argv
  ops=probe['ProcOps']();globals_=ops.snapshot.__globals__;old=globals_['Path'];globals_['Path']=FixturePath
  try:row=ops.snapshot(99,['RAG3DB_LIB','RAG3DB_SHARED'])
  finally:globals_['Path']=old
  self.assertEqual(row['argv'],['cargo','check','--features','']);self.assertEqual(row['rawlen'],len(raw));self.assertNotIn('SECRET',json.dumps(row));self.assertEqual(row['actual']['RAG3DB_LIB'],'sdk29');self.assertIsNone(row['actual']['RAG3DB_SHARED'])
if __name__=='__main__':unittest.main(failfast=True)
