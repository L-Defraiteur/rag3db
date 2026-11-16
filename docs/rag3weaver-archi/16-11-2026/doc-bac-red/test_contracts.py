"""Prepublished pure/temporary-file witnesses; no native execution or real scope."""
from pathlib import Path
import json, tempfile, unittest
from unittest.mock import patch
import contracts as proof
import qualify_red as q

def output(name,interleaved=False):
    prefix=('test '+name+' ... DOCUMENTATION_FAILED_FIXTURE /private/tmp/fixture\n'
            if interleaved else 'DOCUMENTATION_FAILED_FIXTURE /private/tmp/fixture\n')
    child=('test '+name+' ... FAILED\n'
           'test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1 filtered out;\n')
    if interleaved:
        return prefix+"thread parent panicked at /fixture:1\n"+proof.PARENT_MARKER+'\n'+child+\
            "thread child panicked at /fixture:2\n"+proof.CHILD_MARKER+'\nFAILED\n'+\
            'test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1 filtered out;\n'
    return prefix+'test '+name+' ... FAILED\nthread parent panicked at /fixture:1\n'+proof.PARENT_MARKER+'\n'+\
        child+'thread child panicked at /fixture:2\n'+proof.CHILD_MARKER+'\n'+\
        'test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1 filtered out;\n'

class Contracts(unittest.TestCase):
    def test_both_actual_harness_output_shapes_are_one_distinct_two_raw(self):
        name=proof.NAMES['lib']
        for interleaved in [False,True]:
            row=proof.expected_red(name,101,output(name,interleaved))
            self.assertEqual((row['distinct_judges'],row['raw_test_invocations']),(1,2))

    def test_compiler_failure_zero_test_wrong_name_or_extra_panic_is_not_expected(self):
        refusal=json.dumps(dict(reason='compiler-message',message=dict(level='error',message='parse refused')))
        self.assertIsNotNone(q.compile_refusal(refusal))
        self.assertIsNone(q.compile_refusal(refusal.replace('"error"','"warning"')))
        name=proof.NAMES['e2e'];good=output(name)
        for rc,text in [(1,good),(101,'error: compiler refused'),(101,good.replace(name,'wrong',1)),
                        (101,good.replace('1 failed','0 failed',1)),
                        (101,good+'thread extra panicked at elsewhere\n'),
                        (101,good.replace(proof.CHILD_MARKER,'other failure'))]:
            with self.subTest(rc=rc,text=text),self.assertRaises(AssertionError):proof.expected_red(name,rc,text)

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
                value=q.result_file(dict(status='EXPECTED_SETUP_REDS_OBSERVED',deadline=100.))
            self.assertEqual(value['status'],'UNEXPECTED_FAILURE')
            self.assertEqual(json.loads((root/'result.json').read_text()),value)
            with patch.object(q,'P',root),self.assertRaises(FileExistsError):q.result_file(value)

    def test_real_constructor_and_policy_agree_with_outer_timeout_and_exact_pilot(self):
        q.configure_controller();pilot='a'*40;control=Path('/private/checkout')
        actual=q.controller.commands(control,pilot)
        self.assertEqual(actual['outer'],['timeout','--signal=TERM','--kill-after=15s','1920s',
            str(q.C/'scripts/poste'),'lourd',q.sys.executable,str(q.Q/'qualify_red.py'),'--admit',pilot])
        with patch.object(q.controller,'sha',return_value='script-hash'):
            policy=q.controller.policy(control,pilot,dict(pid=7,start='123'))
        self.assertEqual(policy['outer_command'],actual['outer'])
        self.assertEqual(policy['roles']['watchdog']['argv'],actual['outer'])
        self.assertEqual(policy['roles']['poste']['argv'][1:],actual['poste'])
        self.assertEqual(policy['roles']['launch']['argv'],['python3',str(control/q.REL),'--launch',pilot])
        self.assertEqual(policy['launch_identity'],dict(pid=7,start='123'))
        launch=dict(whole_start=100.,soft_end=2020.,hard_end=2035.)
        self.assertEqual(q.phase_end(2500.,launch),2020.)
        self.assertEqual(q.phase_end(1900.,launch),1900.)
        self.assertEqual(q.final_end(1900.,launch,1899.),2019.)
        with self.assertRaises(AssertionError):q.final_end(1900.,launch,1901.)
        with self.assertRaises(AssertionError):q.final_end(2500.,launch,2021.)
        with patch.object(q.controller,'OUTER_SECONDS',123):
            limited=q.controller.commands(control,pilot)
            self.assertEqual(limited['outer'][3],'123s')
            with patch.object(q.controller,'sha',return_value='script-hash'):
                self.assertEqual(q.controller.policy(control,pilot)['roles']['watchdog']['argv'],limited['outer'])

if __name__=='__main__':unittest.main(failfast=True)
