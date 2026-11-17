"""Faux préparés avant runner : AUCUNE exécution autorisée par ce fichier."""
import copy
import unittest
import hashlib
import json
import tempfile
from pathlib import Path
from unittest.mock import patch
from contextlib import ExitStack
import tool_phase
import tool_qualification as tool


class FakeBackend:
    def __init__(self, bad=None):
        self.calls=[];self.bad=bad
    def run(self, request):
        self.calls.append(copy.deepcopy(request))
        result=dict(name=request['name'],returncode=request['expected_code'],timed_out=False,
                    quiet=True,raw_published=True,within_whole=True,stdin_started_inside_whole=True,
                    native_invoked=True,stdout=(b'{"request_valid":true,"pg_contact":false}\n' if request['expected_code']==0 else b''),stderr=request['expected_stderr'].encode())
        if request['name']==self.bad:
            result['returncode']=127
        return result


class QualificationFakes(unittest.TestCase):
    def setUp(self):
        self.cases=[dict(name='case-'+str(i),argv=['--validate-request'],stdin='{}',returncode=1,stderr_contains='refused') for i in range(8)]
        self.config=dict(compile_argv=['PINNED_RUSTC'],client_path='/fixture/new-output/client',
                         compile_cap=60,worker_cap=5,sql_authorized=False,docker_authorized=False,
                         cases=self.cases)
    def test_single_compile_then_eight_cases(self):
        backend=FakeBackend();results=tool.orchestrate(self.config,backend)
        self.assertEqual(len(results),9)
        self.assertEqual([x['name'] for x in backend.calls].count('compile'),1)
        self.assertEqual(len(backend.calls),9)
    def test_link_failure_never_retries_or_runs_cases(self):
        backend=FakeBackend('compile')
        with self.assertRaises(ValueError):tool.orchestrate(self.config,backend)
        self.assertEqual([x['name'] for x in backend.calls],['compile'])
    def test_first_case_failure_stops_following_cases(self):
        backend=FakeBackend('case-0')
        with self.assertRaises(ValueError):tool.orchestrate(self.config,backend)
        self.assertEqual(len(backend.calls),2)
    def test_whole_deadline_covers_raw_publication_and_stdin(self):
        for key in ['quiet','raw_published','within_whole','stdin_started_inside_whole','native_invoked']:
            backend=FakeBackend();original=backend.run
            def run(request,key=key):
                result=original(request);result[key]=False;return result
            backend.run=run
            with self.assertRaises(ValueError):tool.orchestrate(self.config,backend)
    def test_capture_mode_or_contacts_are_refused(self):
        for key in ['sql_authorized','docker_authorized']:
            config=copy.deepcopy(self.config);config[key]=True
            with self.assertRaises(ValueError):tool.orchestrate(config,FakeBackend())
        config=copy.deepcopy(self.config);config['cases'][0]['argv']=['--capture-readonly']
        with self.assertRaises(ValueError):tool.orchestrate(config,FakeBackend())
    def test_worker5_is_not_inner4_and_cannot_be_extended(self):
        config=copy.deepcopy(self.config);config['worker_cap']=6
        with self.assertRaises(ValueError):tool.orchestrate(config,FakeBackend())
    def test_readiness_unknown_cannot_execute(self):
        with self.assertRaises(ValueError):tool.validate_go(dict(run_authorized=False),{},'0'*64,now=100.)
    def test_timeout_or_unknown_quiescence_is_not_success(self):
        for key in ['timed_out','quiet']:
            backend=FakeBackend();original=backend.run
            def run(request,key=key):
                result=original(request);result[key]=(key=='timed_out');return result
            backend.run=run
            with self.assertRaises(ValueError):tool.orchestrate(self.config,backend)


class NativeBoundaryFakes(unittest.TestCase):
    """Vrai NativeBackend avec frontières OS fausses : aucun proc/Popen réel."""
    def setUp(self):
        self.temp=tempfile.TemporaryDirectory();self.addCleanup(self.temp.cleanup)
        self.out=Path(self.temp.name);(self.out/'home').mkdir();(self.out/'tmp').mkdir()
        self.clock=[1000.];self.stops=[]
        self.packet={'inputs_manifest_sha256':'1'*64,'source_code_commit':'FAKE-NOT-RUNTIME','source_files':{'pg_ro_client.rs':'FAKE-SOURCE'}}
        self.go=dict(kind='pg-rust-compile-and-parse-only',run_authorized=True,packet_sha256='2'*64,
                     reference='FAKE-NOT-C7',nonce='a'*32,inputs_manifest_sha256='1'*64,
                     deadline_monotonic=2000.,sql_authorized=False,docker_authorized=False,output=str(self.out))
        self.request=dict(name='compile',kind='compile',argv=['PINNED_RUSTC'],stdin='',cap=60,expected_code=0,expected_stderr='')
        self.backend=tool.NativeBackend(self.out,self.packet,'2'*64,self.go,self.out,'0::/fake')
    def install(self,stack,*,timeout=False,late_fsync=False):
        outer=self
        stack.enter_context(patch.dict(tool.os.environ,{},clear=True))
        class Process:
            pid=4321
            returncode=0
            def __init__(self,*args,**kwargs):pass
            def communicate(self,timeout=None):
                if timeout is not None and timeout<0:raise AssertionError('negative remaining budget')
                if timeout_flag[0]:
                    timeout_flag[0]=False;outer.clock[0]=1059.7
                    raise tool.subprocess.TimeoutExpired('FAKE',timeout)
                phase=outer.out/'compile'
                (phase/'stdout.raw').write_bytes(b'');(phase/'stderr.raw').write_bytes(b'')
                tool.write_new(phase/'phase-result.json',dict(returncode=0,native_reaped=True,raw_published=True,
                              within_whole=True,stdin_started_inside_whole=True,native_invoked=True))
                (outer.out/'client').write_bytes(b'FAKE-ELF-NOT-NATIVE')
                return b'',b''
            def wait(self,timeout=None):return 0
        timeout_flag=[timeout]
        stack.enter_context(patch.object(tool.time,'monotonic',lambda:outer.clock[0]))
        stack.enter_context(patch.object(tool,'common_authority',lambda *a:dict(end=2000.)))
        stack.enter_context(patch.object(tool,'verify_sources',lambda *a:None))
        stack.enter_context(patch.object(tool,'verify_inputs',lambda *a:None))
        stack.enter_context(patch.object(tool,'compile_argv',lambda *a:['PINNED_RUSTC']))
        stack.enter_context(patch.object(tool,'own_cgroup',lambda:'0::/fake'))
        stack.enter_context(patch.object(tool_phase,'identity',lambda pid:dict(pid=pid,start=1,uid=1000,cgroup='0::/fake')))
        stack.enter_context(patch.object(tool,'group_members',lambda *a:([],[])))
        stack.enter_context(patch.object(tool,'stop_group',lambda group,cg:outer.stops.append((group,cg))))
        stack.enter_context(patch.object(tool.subprocess,'Popen',Process))
        if late_fsync:
            real=tool.sync_dir
            def sync(path):real(path);outer.clock[0]=1061.
            stack.enter_context(patch.object(tool,'sync_dir',sync))
    def test_final_publication_after_cap_cannot_be_success(self):
        with ExitStack() as stack:
            self.install(stack,late_fsync=True)
            result=self.backend._run(self.request)
        self.assertIs(result['within_whole'],False)
    def test_timeout_calls_owned_group_cleanup_not_a_retry(self):
        with ExitStack() as stack:
            self.install(stack,timeout=True)
            result=self.backend._run(self.request)
        self.assertIs(result['timed_out'],True)
        self.assertEqual(self.stops,[(4321,'0::/fake')])


class WholePhaseSupervisorFakes(unittest.TestCase):
    def test_preflight_block_refuses_without_parent_hash(self):
        backend=tool.NativeBackend(Path('/fake'),{},'fake',{'deadline_monotonic':2000.},Path('/fake'),'/fake')
        with patch.object(tool,'own_cgroup',return_value='/fake'), patch.object(tool,'supervise_phase',side_effect=TimeoutError('preflight blocked')) as supervisor, patch.object(tool,'verify_inputs') as hashes:
            with self.assertRaises(TimeoutError):backend.run({'cap':5})
            supervisor.assert_called_once();hashes.assert_not_called()
    def test_final_fsync_block_cannot_return_worker_success(self):
        backend=tool.NativeBackend(Path('/fake'),{},'fake',{'deadline_monotonic':2000.},Path('/fake'),'/fake')
        with patch.object(tool,'own_cgroup',return_value='/fake'), patch.object(tool,'supervise_phase',side_effect=TimeoutError('final fsync blocked')):
            with self.assertRaises(TimeoutError):backend.run({'cap':60})
            self.assertIsNone(backend.client_sha)


if __name__=='__main__':unittest.main(verbosity=2)
