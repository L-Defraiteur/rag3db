"""Prepublished pure/temporary-file witnesses; no native execution or real scope."""
from pathlib import Path
import json, tempfile, unittest
from unittest.mock import patch
import contracts as proof
import qualify_green as q

def output(name,interleaved=False):
    root='/private/tmp/fixture'
    row=dict(name=name,manifest=root+'/setup/backend.json',head='a'*40,commands='approval',
             extra_read=[root+'/private-controller'],target_token='{repo_target}',tmp_token='{task_dir}/tmp',
             cargo_jobs='1',offline='true',targets_root=root+'/private-session-targets',
             manifest_equals_declaration=True,head_equals_declaration=True)
    summary='test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1 filtered out;\n'
    marker='DOCUMENTATION_PRIVATE_SANDBOX_COMMITTED '+json.dumps(row)+'\n'
    return ('test '+name+' ... DOCUMENTATION_SETUP_CHILD_OK '+name+'\n'+
            'test '+name+' ... '+marker+'ok\n'+summary+'ok\n'+summary)

class Contracts(unittest.TestCase):
    def test_two_positive_judgments_require_committed_head_and_private_target(self):
        for name in proof.NAMES.values():
            row=proof.expected_green(name,0,output(name))
            self.assertEqual((row['distinct_judges'],row['raw_test_invocations']),(1,2))

    def test_compiler_failure_zero_ignored_wrong_name_and_false_attestation_are_red(self):
        refusal=json.dumps(dict(reason='compiler-message',message=dict(level='error',message='parse refused')))
        self.assertIsNotNone(q.compile_refusal(refusal))
        self.assertIsNone(q.compile_refusal(refusal.replace('"error"','"warning"')))
        name=proof.NAMES['e2e'];good=output(name)
        cases=[(1,good),(0,'error: compiler refused'),(0,good.replace(name,'wrong',1)),
               (0,good.replace('1 passed','0 passed',1)),(0,good.replace('0 ignored','1 ignored',1)),
               (0,good+'thread extra panicked at elsewhere\n'),
               (0,good.replace('"head_equals_declaration": true','"head_equals_declaration": false')),
               (0,good.replace('"commands": "approval"','"commands": "auto"')),
               (0,good.replace('"cargo_jobs": "1"','"cargo_jobs": "8"')),
               (0,good.replace('{repo_target}','/shared/target')),
               (0,good.replace('DOCUMENTATION_SETUP_CHILD_OK','missing-marker'))]
        for rc,text in cases:
            with self.subTest(rc=rc,text=text),self.assertRaises(AssertionError):proof.expected_green(name,rc,text)

    def test_private_profile_respects_poste_and_drops_credentials_and_old_ld(self):
        root=Path('/private')
        env=proof.private_environment(dict(HOME='/home/test',PATH='/usr/bin',GOOGLE_CLOUD_PROJECT='forbidden',
            GOOGLE_APPLICATION_CREDENTIALS='/secret',SOME_API_KEY='secret',LD_LIBRARY_PATH='/old',
            RAG3DB_SHARED='1',POSTE_FILS='16'),root,root/'tmp',2)
        self.assertEqual([env[k] for k in ['POSTE_FILS','CARGO_BUILD_JOBS','RAYON_NUM_THREADS']],['2']*3)
        self.assertEqual(env['RUST_TEST_THREADS'],'1')
        self.assertFalse(any(k.startswith('GOOGLE_') or k.endswith('_API_KEY') for k in env))
        self.assertNotIn('LD_LIBRARY_PATH',env);self.assertNotIn('RAG3DB_SHARED',env)
        self.assertEqual(Path(env['XDG_STATE_HOME']).parent,Path(env['RAG3WEAVER_STATE_DIR']).parent)
        self.assertNotEqual(env['XDG_STATE_HOME'],env['RAG3WEAVER_STATE_DIR'])

    def test_invalid_poste_allocation_cannot_launch_a_profile(self):
        with self.assertRaises(AssertionError):proof.private_environment({},Path('/p'),Path('/p/tmp'),0)

    def test_real_final_open_changes_the_returned_and_written_verdict(self):
        with tempfile.TemporaryDirectory() as tmp:
            root=Path(tmp);clock=[99.];real_open=Path.open
            def open_late(path,*args,**kwargs):
                stream=real_open(path,*args,**kwargs)
                if path==root/'result.json':clock[0]=101.
                return stream
            with patch.object(q,'P',root),patch.object(q.time,'monotonic',side_effect=lambda:clock[0]),patch.object(Path,'open',open_late):
                value=q.result_file(dict(status='GREEN_SETUP_TARGETED_ONLY',deadline=100.))
            self.assertEqual(value['status'],'UNEXPECTED_FAILURE')
            self.assertEqual(json.loads((root/'result.json').read_text()),value)
            with patch.object(q,'P',root),self.assertRaises(FileExistsError):q.result_file(value)

    def test_real_fsync_crossing_final_deadline_rewrites_file_and_return_to_failure(self):
        with tempfile.TemporaryDirectory() as tmp:
            root=Path(tmp);clock=[99.];real_sync=q.os.fsync;calls=[]
            def late_sync(fd):
                real_sync(fd);calls.append(fd);clock[0]=101.
            with patch.object(q,'P',root),patch.object(q.time,'monotonic',side_effect=lambda:clock[0]),patch.object(q.os,'fsync',side_effect=late_sync):
                value=q.result_file(dict(status='GREEN_SETUP_TARGETED_ONLY',deadline=100.))
            self.assertTrue(calls,'the real write/flush/fsync boundary must be exercised')
            self.assertEqual(value['status'],'UNEXPECTED_FAILURE')
            self.assertEqual(json.loads((root/'result.json').read_text()),value)
            self.assertEqual(0 if value['status']=='GREEN_SETUP_TARGETED_ONLY' else 1,1)

    def test_real_constructor_and_policy_agree_with_outer_timeout_and_exact_pilot(self):
        q.configure_controller();pilot='a'*40;control=Path('/private/checkout')
        actual=q.controller.commands(control,pilot)
        self.assertEqual(actual['outer'],['timeout','--signal=TERM','--kill-after=15s','780s',
            q.sys.executable,str(q.Q/'qualify_green.py'),'--arm',pilot])
        with patch.object(q.controller,'sha',return_value='script-hash'):
            policy=q.controller.policy(control,pilot,dict(pid=7,start='123'))
        self.assertEqual(policy['outer_command'],actual['outer'])
        self.assertEqual(policy['roles']['watchdog']['argv'],actual['outer'])
        self.assertEqual(policy['roles']['poste']['argv'][1:],actual['poste'])
        self.assertEqual(policy['roles']['launch']['argv'],['python3',str(control/q.REL),'--launch',pilot])
        self.assertEqual(policy['launch_identity'],dict(pid=7,start='123'))
        launch=dict(whole_start=100.,soft_end=880.,hard_end=895.)
        self.assertEqual(q.phase_end(2500.,launch),880.)
        self.assertEqual(q.phase_end(800.,launch),800.)
        self.assertEqual(q.final_end(800.,launch,799.),880.)
        with self.assertRaises(AssertionError):q.final_end(800.,launch,801.)
        with self.assertRaises(AssertionError):q.final_end(2500.,launch,881.)
        with patch.object(q.controller,'OUTER_SECONDS',123):
            limited=q.controller.commands(control,pilot)
            self.assertEqual(limited['outer'][3],'123s')
            with patch.object(q.controller,'sha',return_value='script-hash'):
                self.assertEqual(q.controller.policy(control,pilot)['roles']['watchdog']['argv'],limited['outer'])

if __name__=='__main__':unittest.main(failfast=True)
