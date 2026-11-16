"""Small CPU tests of real entry/controller code, no native build or systemd unit."""
import ast
import json
import os
from pathlib import Path
import signal
import subprocess
import sys
import tempfile
import time
import unittest
from types import SimpleNamespace

from parse_receipt import inspect_raw_identities

HERE=Path(__file__).parent


def functions_from_controller(namespace):
    tree=ast.parse((HERE/'controller.py').read_text())
    names={'write','group_exists','finish_child','run'}
    selected=ast.Module(body=[x for x in tree.body if isinstance(x,ast.FunctionDef) and x.name in names],type_ignores=[])
    import selectors
    namespace.update({'os':os,'json':json,'time':time,'signal':signal,
                      'subprocess':subprocess,'selectors':selectors})
    exec(compile(selected,str(HERE/'controller.py'),'exec'),namespace)
    return namespace


class SupervisionTests(unittest.TestCase):
    def setUp(self):
        self.temp=tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.out=Path(self.temp.name)
        (self.out/'raw').mkdir()

    def context(self):
        return functions_from_controller({'process':None,'raw':self.out/'raw','out':self.out,
                                          'deadline':time.monotonic()+15,'cgroup_pids':lambda:set()})

    def test_real_controller_timeout_reaps_its_own_process(self):
        context=self.context()
        with self.assertRaises(TimeoutError):
            context['run']([sys.executable,'-c','import time; time.sleep(5)'],'timeout',.15)
        result=json.loads((self.out/'raw/timeout/exit.json').read_text())
        self.assertTrue(result['timeout'])
        self.assertTrue(result['waited'])
        self.assertIsNotNone(context['process'].poll())
        self.assertLess(result['elapsed_s'],2)

    def test_real_controller_native_stdout_tee_is_unchanged(self):
        context=self.context()
        context['run']([sys.executable,'-c','print("fixture stdout")'],'tee',2,attestation=True)
        self.assertEqual((self.out/'attestation.log').read_bytes(),(self.out/'raw/tee/stdout.log').read_bytes())

    def test_group_cleanup_runs_even_if_leader_already_exited(self):
        context=self.context()
        alive=True
        signals=[]
        def killpg(group,sig):
            nonlocal alive
            if not alive:
                raise ProcessLookupError()
            if sig:
                signals.append((group,sig));alive=False
        context['os']=SimpleNamespace(killpg=killpg)
        context['process']=SimpleNamespace(pid=12345,poll=lambda:0)
        self.assertTrue(context['finish_child']())
        self.assertEqual(signals,[(12345,signal.SIGTERM)])

    def test_real_entry_preflight_refusal_still_writes_final_red(self):
        (self.out/'recipe.json').write_text(json.dumps({'unit':'not-the-owned-unit'}))
        env=dict(os.environ,PYTHONDONTWRITEBYTECODE='1')
        result=subprocess.run([sys.executable,str(HERE/'entry.py'),str(self.out)],
                              stdout=subprocess.PIPE,stderr=subprocess.STDOUT,env=env,timeout=3)
        self.assertEqual(result.returncode,1,result.stdout.decode())
        self.assertTrue((self.out/'raw/driver/first-refusal.json').is_file())
        state=json.loads((self.out/'raw/driver/quiescence.json').read_text())
        self.assertFalse(state['invoked'])
        self.assertEqual((self.out/'resume.log').read_text().splitlines()[-1],'ROUGE')
        self.assertFalse((self.out/'raw/driver/before-unit.txt').exists())

    def test_external_timeout_kills_a_term_ignoring_private_fixture(self):
        start=time.monotonic()
        result=subprocess.run(['timeout','--signal=TERM','--kill-after=0.2s','0.1s',sys.executable,
                               '-c','import signal,time; signal.signal(signal.SIGTERM,signal.SIG_IGN); time.sleep(5)'],
                              stdout=subprocess.PIPE,stderr=subprocess.PIPE,timeout=2)
        self.assertIn(result.returncode,[-signal.SIGKILL,137])
        self.assertLess(time.monotonic()-start,2)

    def test_controller_transaction_identity_refusal_precedes_next_phase(self):
        text=(HERE/'controller.py').read_text()
        begin=text.index("    phase='transaction'\n")
        end=text.index("    phase='known-red'\n",begin)
        excerpt='\n'.join(x[4:] if x.startswith('    ') else x for x in text[begin:end].splitlines())
        directory=self.out/'raw/transaction';directory.mkdir()
        (directory/'stdout.log').write_text('rag3db-build-'+'0'*40+'-Release\n')
        launched=[]
        context={'raw':self.out/'raw','files':{'transaction':Path('/fixture/transaction')},
                 'test_env':{},'marker':'rag3db-build-'+'8'*40+'-Release',
                 'identities':{},'results':{},'confined':lambda argv,*args:argv,
                 'run':lambda *args,**kwargs:launched.append('transaction'),
                 'inspect_raw_identities':inspect_raw_identities}
        with self.assertRaises(ValueError):
            exec(compile(excerpt,str(HERE/'controller.py'),'exec'),context)
        self.assertEqual(launched,['transaction'])
        self.assertEqual(context['phase'],'transaction')
        self.assertEqual(context['results'],{})


if __name__=='__main__':
    unittest.main(verbosity=2)
