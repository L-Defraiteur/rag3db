"""Prepublished hermetic witnesses. No real process, SDK, signal, network or Cargo."""
from pathlib import Path
import ast,json,os,tempfile,unittest
from types import SimpleNamespace
from unittest.mock import patch
import controllers,evidence,policy,qualify as q,runner
HERE=Path(__file__).parent
ROOT=HERE.parents[3];C=ROOT/'extension/rag3weaver'

def summary(passed=1,failed=0,ignored=0,filtered=0):return f'test result: {"FAILED" if failed else "ok"}. {passed} passed; {failed} failed; {ignored} ignored; 0 measured; {filtered} filtered out; finished in 0.01s\n'

def output(name='module::case',total=1):return 'running 1 test\ntest '+name+' ... ok\n'+summary(filtered=total-1)

class Pure(unittest.TestCase):
 def test_jobs_and_caps_have_one_global_envelope(self):
  self.assertEqual(policy.jobs(10),8)
  for n in [0,5,7]:
   with self.assertRaises(RuntimeError):policy.jobs(n)
  self.assertEqual(policy.deadline(100,200,500),185)
  self.assertEqual(policy.TERM+15,policy.HARD);self.assertLessEqual(sum(policy.CAPS.values()),policy.BUDGET)
  with self.assertRaises(RuntimeError):policy.deadline(190,200,20)
 def test_profiles_separate_feature_flags_and_inherited_sdk(self):
  expected=policy.profile(Path('/private/store'),Path('/private/cpu'),'rustlib:sdk')
  raw=b''.join(k.encode()+b'='+v.encode()+b'\0' for k,v in expected.items());value=policy.selected(raw,expected);policy.validate_profile(value)
  for bad in [b'',raw.replace(b'RAG3DB_SHARED=1\0',b''),raw+b'RAG3DB_SHARED=1\0',raw+b'RAG3WEAVER_MOTEUR_ANCIEN=1\0']:
   with self.subTest(raw=bad),self.assertRaises(RuntimeError):policy.validate_profile(policy.selected(bad,expected))
 def test_private_environment_drops_unrelated_credentials_and_has_sibling_states(self):
  env=policy.private_environment(dict(HOME='/home/test',PATH='/bin',GOOGLE_APPLICATION_CREDENTIALS='/secret',SOME_API_KEY='x',LD_LIBRARY_PATH='/old'),Path('/p'),Path('/f/tmp'),Path('/t/task-targets'),10)
  self.assertNotIn('GOOGLE_APPLICATION_CREDENTIALS',env);self.assertNotIn('SOME_API_KEY',env);self.assertNotIn('LD_LIBRARY_PATH',env)
  self.assertEqual(Path(env['XDG_STATE_HOME']).parent,Path(env['RAG3WEAVER_STATE_DIR']).parent);self.assertNotEqual(env['XDG_STATE_HOME'],env['RAG3WEAVER_STATE_DIR'])
 def test_target_names_follow_ascii_rust_contract(self):
  self.assertEqual(policy.source_target('/session/task-targets','É mon dépôt'),Path('/session/task-targets/taches-__mon_d_p_t'))
  text=(C/'src/commande.rs').read_text();self.assertIn('is_ascii_alphanumeric()',text);self.assertIn('taches-{propre}',text)
 def test_sandbox_capabilities_are_copied_before_task_without_mutating_template(self):
  with tempfile.TemporaryDirectory() as tmp:
   root=Path(tmp).resolve();manifest=dict(workspace=dict(commands='approval',sandbox=dict(extra_read=['rust'],env={})))
   value=policy.sandbox(manifest,root,root,root,10)
   self.assertEqual(manifest['workspace']['sandbox']['extra_read'],['rust']);self.assertIn(str(root),value['workspace']['sandbox']['extra_read'])
   self.assertEqual(value['workspace']['sandbox']['env']['CARGO_TARGET_DIR'],'{repo_target}');self.assertEqual(value['workspace']['sandbox']['env']['TMPDIR'],'{task_dir}/tmp')
   for key,bad in [('network',True),('mode','off'),('extra_write',['/shared'])]:
    broken=json.loads(json.dumps(manifest));broken['workspace']['sandbox'][key]=bad
    with self.assertRaises(RuntimeError):policy.sandbox(broken,root,root,root,10)
 def test_real_shim_guard_is_readable_by_the_actual_sandbox_declaration(self):
  import broker
  from settings import C as actual_c
  with tempfile.TemporaryDirectory() as tmp:
   root=Path(tmp).resolve();manifest=dict(workspace=dict(commands='approval',sandbox=dict(extra_read=[],env={})))
   value=policy.sandbox(manifest,root,root,root,8)
   tree=ast.parse(Path(broker.__file__).read_text())
   guards=[n.value for n in ast.walk(tree) if isinstance(n,ast.keyword) and n.arg=='guard']
   self.assertEqual(len(guards),1)
   actual=Path(eval(compile(ast.Expression(guards[0]),'actual-broker-guard','eval'),dict(C=actual_c,str=str)))
   self.assertTrue(any(actual==Path(r) or actual.is_relative_to(Path(r)) for r in value['workspace']['sandbox']['extra_read']))
 def test_named_red_rejects_an_unrelated_real_panic(self):
  text='running 1 test\ntest '+policy.D6_NAME+' ... FAILED\nthread x panicked at src/backend.rs:2\nunrelated storage panic\n'+summary(passed=0,failed=1)
  with self.assertRaises(RuntimeError):policy.judged(text,policy.D6_NAME,True)
 def test_published_primitive_hashes_match_the_actual_local_files(self):
  import hashlib
  rows=json.loads((HERE/'provenance.json').read_text())
  for row in rows:
   with self.subTest(destination=row['destination']):
    self.assertEqual(hashlib.sha256((HERE/row['destination']).read_bytes()).hexdigest(),row['destination_sha256'])
 def test_real_host_and_tty_entries_match_actual_cargo_and_cli(self):
  import tomllib
  cargo=tomllib.loads((C/'Cargo.toml').read_text());bins={b['name']:b for b in cargo['bin']}
  self.assertEqual(bins['rag3weaver-backend']['path'],'src/bin/rag3weaver-backend.rs')
  text=(C/bins['rag3weaver-backend']['path']).read_text();self.assertIn('Some("daemon")',text);self.assertIn('"--address"',text)
  tty=tomllib.loads((C/'tty/Cargo.toml').read_text());self.assertEqual(tty['bin'][0]['name'],'rag3weaver-tty');self.assertEqual(tty['bin'][0]['required-features'],['vue'])
  tree=ast.parse((HERE/'d6.py').read_text());calls=[n for n in ast.walk(tree) if isinstance(n,ast.Call) and isinstance(n.func,ast.Attribute) and n.func.attr=='Popen']
  actual=next(n.args[0] for n in calls if isinstance(n.args[0],ast.List) and any(isinstance(e,ast.Constant) and e.value=='daemon' for e in n.args[0].elts))
  self.assertEqual([ast.literal_eval(e) for e in actual.elts[1:]],['daemon','--address','127.0.0.1:0','--threads','8'])
 def test_cargo_adapter_splits_only_the_declared_real_test(self):
  for name in [policy.D6_NAME,policy.REVIEW_NAME]:
   build,test=policy.split_test(policy.d6_command(name));self.assertIn('--no-run',build);self.assertIn('--offline',build);self.assertEqual(test,[name,'--exact','--nocapture','--test-threads=1'])
  with self.assertRaises(RuntimeError):policy.split_test(['test','another'])
 def test_real_rust_red_is_exact_not_a_compile_failure_or_zero_test(self):
  name=policy.D6_NAME;text='running 1 test\ntest '+name+' ... FAILED\nthread x panicked at actual.rs:2\n'+summary(passed=0,failed=1)
  self.assertEqual(policy.judged(text,name,True)['failed'],1)
  for bad in [text.replace(name,'another',1),text.replace('1 failed','0 failed'),text+'error: could not compile anything']:
   with self.assertRaises(RuntimeError):policy.judged(bad,name,True)
 def test_independent_inventory_and_strict_phase_refuse_zero_duplicate_or_foreign(self):
  inv=evidence.inventory('module::case: test\nmodule::skip: test\n','module::skip: test\n');self.assertEqual(len(inv['names']),2)
  self.assertEqual(evidence.phase(output(total=2),['module::case'],[],2)['summary'],[1,0,0,0,1])
  for bad in ['running 0 tests\n'+output(total=2),output(total=2).replace('module::case','alien'),output(total=2)+summary(filtered=1),output(total=2).replace('1 filtered','0 filtered')]:
   with self.assertRaises(RuntimeError):evidence.phase(bad,['module::case'],[],2)
 def test_nocapture_interleaving_is_completed_by_the_actual_harness_result(self):
  text=output().replace('test module::case ... ok','test module::case ... a legitimate diagnostic\nok')
  self.assertEqual(evidence.phase(text,['module::case'],[],1)['summary'],[1,0,0,0,0])
  with self.assertRaises(RuntimeError):evidence.phase(text.replace('\nok\n','\nFAILED\n'),['module::case'],[],1)
 def test_nested_setup_attestation_is_typed_not_a_second_top_level_judge(self):
  name=evidence.NESTED_NAMES[0];marker=dict(name=name,commands='approval',head='a'*40,manifest_equals_declaration=True,head_equals_declaration=True,cargo_jobs='1',offline='true',target_token='{repo_target}',tmp_token='{task_dir}/tmp',targets_root='/f/private-session-targets',manifest='/f/setup/backend.json')
  child='running 1 test\ntest '+name+' ... DOCUMENTATION_PRIVATE_SANDBOX_COMMITTED '+json.dumps(marker)+'\nok\n'+summary(filtered=2)
  text='running 1 test\ntest '+name+' ... DOCUMENTATION_SETUP_CHILD_OK '+name+'\n'+child+'\nok\n'+summary(filtered=2)
  row=evidence.phase(text,[name],[],3);self.assertEqual(row['summary'],[1,0,0,0,2]);self.assertEqual(row['nested_raw_invocations'],1)
  for bad in [text.replace('"head_equals_declaration": true','"head_equals_declaration": false'),text.replace('1 passed','0 passed',1),text.replace('{repo_target}','/shared')]:
   with self.assertRaises(RuntimeError):evidence.phase(bad,[name],[],3)
 def test_result_reclassification_after_real_fsync_agrees_with_exit(self):
  with tempfile.TemporaryDirectory() as tmp:
   path=Path(tmp)/'result.json';clock=[99.];sync=os.fsync
   def late(fd):sync(fd);clock[0]=101.
   with patch.object(evidence.os,'fsync',side_effect=late):value=evidence.finish(path,dict(status='GREEN_GLOBAL_D6_AND_CANONICAL',deadline=100),clock=lambda:clock[0])
   self.assertEqual(value['status'],'RED');self.assertEqual(value,json.loads(path.read_text()));self.assertEqual(0 if value['status']=='GREEN_GLOBAL_D6_AND_CANONICAL' else 1,1)
 def test_first_red_is_immutable_and_active_logs_do_not_scan_historical_evidence(self):
  with tempfile.TemporaryDirectory() as tmp:
   root=Path(tmp);(root/'old.log').write_text('test obsolete ... FAILED\n');(root/'active.log').write_text('still running\n')
   with patch.object(runner,'P',root):
    context=runner.Runner({},1000,'/unused');context.active_failure([root/'active.log']);context.fail('first');context.fail('later')
   self.assertEqual(json.loads((root/'first-red.json').read_text())['reason'],'first')
 def test_cli_dispatch_is_pure_before_the_real_watchdog_spawn(self):
  pilot='a'*40;control=Path(q.__file__).resolve().parents[4];events=[];exists=Path.exists
  def access(path):
   if str(q.Q) in str(path):events.append('Q-access');return False
   return exists(path)
  def process(argv,**kwargs):events.append(('watchdog',argv));return SimpleNamespace(returncode=0)
  entry=ast.parse(Path(q.__file__).read_text()).body[-1]
  with patch.object(Path,'exists',access),patch.object(q.subprocess,'run',side_effect=process),patch.object(q.sys,'argv',[q.__file__,'--launch',pilot]),patch.object(q.sys,'orig_argv',['python3',str(control/q.REL),'--launch',pilot]):
   with self.assertRaises(SystemExit) as stop:exec(compile(ast.Module(body=[entry],type_ignores=[]),'actual-main','exec'),dict(q.__dict__,__name__='__main__'))
  self.assertEqual(stop.exception.code,0);self.assertEqual(len(events),1);self.assertEqual(events[0][0],'watchdog');self.assertEqual(events[0][1][:4],['timeout','--signal=TERM','--kill-after=15s','7440s'])
 def test_constructor_policy_uses_the_same_exclusive_poste_and_three_roles(self):
  q.configure();pilot='b'*40;control=Path('/private/control');actual=controllers.commands(control,pilot)
  self.assertEqual(actual['poste'][1],'mesure');self.assertIn('--arm',actual['outer']);self.assertEqual(actual['outer'][3],'7440s')
  with patch.object(controllers,'sha',return_value='hash'):expected=controllers.policy(control,pilot,dict(pid=8,start='99'))
  self.assertEqual(set(expected['roles']),{'launch','poste','watchdog'});self.assertEqual(expected['roles']['watchdog']['argv'],actual['outer']);self.assertEqual(expected['roles']['poste']['argv'][1:],actual['poste'])

if __name__=='__main__':unittest.main(failfast=True)
