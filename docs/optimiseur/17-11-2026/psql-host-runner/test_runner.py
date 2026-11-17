"""Témoins publiés avant CPU. Aucun PG, psql, Docker, réseau ou secret réel.

La baseline runner.py est importable et refuse la capacité non implémentée.
Les processus natifs de test sont uniquement des enfants Python privés.
"""
import copy
import importlib.util
import json
import os
import signal
from pathlib import Path
import sys
import tempfile
import time
import unittest
from unittest.mock import patch
from capture_evidence import archived_capture, proc_state, write_new, sync

HERE = Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location('psql_runner', HERE/'runner.py')
runner = importlib.util.module_from_spec(spec)
spec.loader.exec_module(runner)


def profile():
    nonce = 'a'*32
    return dict(kind='psql-private-fixture-profile-1', nonce=nonce,
        fixture_nonce='b'*32, host='127.0.0.1', port=15433,
        database='g_pg_probe_'+('b'*12), user='g_pg_observer',
        expected_identity=dict(database='g_pg_probe_'+('b'*12),user='g_pg_observer',
            session_user='g_pg_observer',database_oid=16384,user_oid=16385,
            owner='g_pg_fixture_admin',owner_oid=10,server_address='127.0.0.1',
            server_port=5432,server_version_num=170011,postmaster_start='fixed-fixture-start'))


def rows(p=None, *, check=False, identity=None):
    p = p or profile(); pid = 202 if check else 101
    ro = dict(transaction_read_only=True,default_transaction_read_only=True,
        superuser=False,createdb=False,createrole=False,replication=False,
        bypassrls=False,database_owner=False,user=p['user'],database=p['database'],backend_pid=pid)
    app = 'g-pg-'+p['nonce']
    row = (dict(backend_pid=identity['backend_pid'],application_name=app,backend_absent=True,
        observer_backend_pid=pid,observer_application_name=app+':check') if check else
        dict(p['expected_identity'],backend_pid=pid,application_name=app))
    return ro,row


def encode(*values):
    return b''.join((json.dumps(x)+'\n').encode() for x in values)


class Profiles(unittest.TestCase):
    def test_fixture_is_distinct_from_v1(self):
        p=profile(); runner.validate_profile(p)
        for patch in [dict(port=5433),dict(kind='postgres-read-only-admission'),
                      dict(user='rag3weaver'),dict(database='rag3weaver_test'),dict(port=True)]:
            with self.subTest(patch=patch), self.assertRaises(ValueError):
                runner.validate_profile(dict(p,**patch))

    def test_external_identity_cannot_be_relabelled(self):
        p=profile();p['expected_identity']['owner']=p['user']
        with self.assertRaises(ValueError):runner.validate_profile(p)

    def test_same_session_ro_and_identity_are_required(self):
        p=profile();ro,row=rows(p)
        self.assertEqual(runner.validate_session(encode(ro,row),p),row)
        ro['backend_pid']=999
        with self.assertRaises(ValueError):runner.validate_session(encode(ro,row),p)

    def test_ro_truthy_is_not_true(self):
        for value in [1,'on',False,None]:
            ro,row=rows();ro['transaction_read_only']=value
            with self.subTest(value=value),self.assertRaises(ValueError):
                runner.validate_session(encode(ro,row),profile())

    def test_role_flags_cannot_be_numeric_false(self):
        ro,row=rows();ro['superuser']=0
        with self.assertRaises(ValueError):runner.validate_session(encode(ro,row),profile())

    def test_extra_row_and_duplicate_keys_refused(self):
        ro,row=rows()
        for raw in [encode(ro,row,{ }),encode(ro)+b'{"backend_pid":101,"backend_pid":102}\n']:
            with self.assertRaises(ValueError):runner.validate_session(raw,profile())

    def test_postcheck_requires_absence_and_distinct_pid(self):
        identity=rows()[1];ro,row=rows(check=True,identity=identity)
        self.assertEqual(runner.validate_session(encode(ro,row),profile(),identity=identity),row)
        for bad in [dict(backend_absent=1),dict(backend_absent=False),dict(observer_backend_pid=101)]:
            with self.subTest(bad=bad),self.assertRaises(ValueError):
                runner.validate_session(encode(ro,dict(row,**bad)),profile(),identity=identity)

    def test_postcheck_session_pid_application_and_target_binding(self):
        p=profile();identity=rows(p)[1];ro,row=rows(p,check=True,identity=identity)
        variants=[(dict(ro,backend_pid=303),row),
            (ro,dict(row,backend_pid=999)),
            (ro,dict(row,application_name='g-pg-'+('c'*32))),
            (ro,dict(row,observer_application_name='g-pg-'+p['nonce'])),
            (dict(ro,database='another_database'),row),
            (dict(ro,user='another_user'),row)]
        for ro_bad,row_bad in variants:
            with self.subTest(ro=ro_bad,row=row_bad),self.assertRaises(ValueError):
                runner.validate_session(encode(ro_bad,row_bad),p,identity=identity)


class RealCapture(unittest.TestCase):
    def call(self,code,*,stdin=b'',seconds=1,env=None):
        return archived_capture(runner,self.id().split('.')[-1],[sys.executable,'-I','-c',code],
            env=env or dict(PATH='/usr/bin:/bin',LANG='C'),stdin=stdin,
            deadline=time.monotonic()+seconds)

    def test_real_pipe_stdin_stdout_stderr_and_reap(self):
        r=self.call('import sys;sys.stdout.buffer.write(sys.stdin.buffer.read());sys.stderr.write("owned")',stdin=b'input')
        self.assertEqual(r['stdout'],b'input');self.assertEqual(r['stderr'],b'owned')
        self.assertIs(r['reaped'],True);self.assertEqual(r['returncode'],0)
        with self.assertRaises(ProcessLookupError):os.kill(r['pid'],0)

    def test_hung_child_is_killed_and_reaped_under_original_end(self):
        end=time.monotonic()+.5
        with self.assertRaises(runner.CaptureRefused) as ctx:
            archived_capture(runner,self.id().split('.')[-1],[sys.executable,'-I','-c','import time;time.sleep(30)'],
                env=dict(PATH='/usr/bin:/bin'),stdin=b'',deadline=end)
        rec=ctx.exception.record
        self.assertIs(rec['timed_out'],True);self.assertIs(rec['reaped'],True)
        self.assertLessEqual(rec['ended_monotonic'],end)
        with self.assertRaises(ProcessLookupError):os.kill(rec['pid'],0)

    def test_flood_is_bounded_and_refused(self):
        with self.assertRaises(runner.CaptureRefused) as ctx:
            self.call('import os;os.write(1,b"x"*1000000)')
        rec=ctx.exception.record
        self.assertIs(rec['truncated'],True);self.assertIs(rec['reaped'],True)
        self.assertLessEqual(len(rec['stdout']),65536)

    def test_exit_code_is_not_success(self):
        r=self.call('raise SystemExit(7)')
        self.assertEqual(r['returncode'],7);self.assertIs(r['reaped'],True)

    def test_expired_end_refuses_before_child(self):
        with patch.object(runner.subprocess,'Popen',side_effect=AssertionError('unexpected child')) as spawn:
            with self.assertRaises(ValueError):
                runner.capture([sys.executable,'-I','-c','raise Exception("must not run")'],
                    env={},stdin=b'',deadline=time.monotonic()-1)
            spawn.assert_not_called()

    def test_stderr_flood_is_bounded_and_refused(self):
        with self.assertRaises(runner.CaptureRefused) as ctx:
            self.call('import os;os.write(2,b"x"*1000000)')
        rec=ctx.exception.record
        self.assertIs(rec['truncated'],True);self.assertIs(rec['reaped'],True)
        self.assertLessEqual(len(rec['stderr']),65536)

    def test_ignoring_term_does_not_escape_original_end(self):
        end=time.monotonic()+.8
        with self.assertRaises(runner.CaptureRefused) as ctx:
            archived_capture(runner,self.id().split('.')[-1],[sys.executable,'-I','-c',
                'import os,signal,time;signal.signal(signal.SIGTERM,signal.SIG_IGN);os.write(1,b"HANDLER_READY\\n");time.sleep(30)'],
                env=dict(PATH='/usr/bin:/bin'),stdin=b'',deadline=end)
        self.assertIs(ctx.exception.record['reaped'],True)
        self.assertEqual(ctx.exception.record['stdout'],b'HANDLER_READY\n')
        self.assertEqual(ctx.exception.record['returncode'],-signal.SIGKILL)
        self.assertLessEqual(ctx.exception.record['ended_monotonic'],end)
        with self.assertRaises(ProcessLookupError):os.kill(ctx.exception.record['pid'],0)

    def test_child_exit_with_pipe_retained_is_partial_not_success(self):
        # Futur CPU distinct : vrai petit-enfant privé, borné .35 s.
        # Le parent attend son identité/handshake AVANT de sortir0 ; EOF reste
        # détenu dans le même process group/session privées. Pas d'observateur.
        code="""import os,json,time
readfd,writefd=os.pipe()
pid=os.fork()
if pid==0:
 os.close(readfd)
 start=open('/proc/self/stat').read().rsplit(')',1)[1].split()[19]
 row=dict(pid=os.getpid(),start=start,pgrp=os.getpgrp(),session=os.getsid(0))
 os.write(1,(json.dumps(row)+'\\n').encode())
 os.write(writefd,b'1');os.close(writefd)
 time.sleep(.35);os._exit(0)
os.close(writefd);os.read(readfd,1);os.close(readfd);os._exit(0)
"""
        with self.assertRaises(runner.CaptureRefused) as ctx:
            self.call(code,seconds=1)
        rec=ctx.exception.record
        self.assertIs(rec['partial'],True)
        self.assertIs(rec['reaped'],True)
        holder=json.loads(rec['stdout'])
        self.assertEqual(holder['pgrp'],rec['pid'])
        self.assertEqual(holder['session'],rec['pid'])
        state=proc_state(holder['pid'],holder['start'])
        target=Path(os.environ['PSQL_CPU_CAPTURE_PROOFS'])/self.id().split('.')[-1]
        write_new(target/'holder-after.json',encode(dict(identity=holder,observed=state)))
        sync(target)
        # A zombie is retained as such in the proof, not declared nonexistent.
        self.assertTrue(state['original'].startswith('ABSENT') or state['state']=='Z',state)



def fake_observe(out, *, expire=False, capture_patch=None):
    # Même raccord observe/publisher que le client réel ; seul le transport est faux.
    now=[100.0]; p=profile();ready=dict(active=True,first_red=False,common_end=120.0)
    def factory(deadline):
        def session(identity=None):
            ro,row=rows(p,check=identity is not None,identity=identity)
            now[0]+=.1
            rec=dict(stdout=encode(ro,row),stderr=b'',returncode=0,reaped=True,
                partial=False,truncated=False,timed_out=False,pid=900+(identity is not None),
                started_monotonic=now[0]-.1,ended_monotonic=now[0])
            rec.update(capture_patch or {})
            return rec
        return session
    def phase_check():return ready.copy()
    if not expire:
        return runner.observe(p,out,session_factory=factory,clock=lambda:now[0],phase_check=phase_check)
    actual_sync=runner.sync_directory
    crossed=[False]
    def real_fsync_then_expire(path):
        actual_sync(path)
        crossed[0]=True
        now[0]=106.0
    try:
        with patch.object(runner,'sync_directory',side_effect=real_fsync_then_expire):
            return runner.observe(p,out,session_factory=factory,clock=lambda:now[0],phase_check=phase_check)
    finally:
        if not crossed[0]:raise AssertionError('expiration must cross real publisher fsync')



class WholeJob(unittest.TestCase):
    """Observe -> capture injectée -> validation -> vrai publisher fichiers."""
    def test_success_publishes_new_fixture_kind_not_v1_grant(self):
        with tempfile.TemporaryDirectory() as temp:
            try:
                r=fake_observe(Path(temp)/'proof')
            except runner.RunnerUnavailable as error:
                self.fail('capacité nominale indisponible : '+str(error))
            doc=json.loads((Path(temp)/'proof/observation.json').read_text())
            self.assertEqual(doc['kind'],'psql-private-fixture-observation-1')
            self.assertIs(doc['transport_qualified'],False)
            self.assertIs(doc['grant_authorized'],False)
            self.assertLessEqual(doc['ended_monotonic'],doc['deadline_monotonic'])
            self.assertEqual(r[0],Path(temp)/'proof/observation.json')

    def test_whole5_exhausted_during_publication_has_durable_refusal(self):
        with tempfile.TemporaryDirectory() as temp:
            out=Path(temp)/'proof'
            with self.assertRaises(ValueError):fake_observe(out,expire=True)
            refusal=json.loads((out/'refusal.json').read_text())
            self.assertIs(refusal['grant_authorized'],False)
            self.assertNotIn('artifact_sha256',refusal)

    def test_cleanup_or_late_capture_never_publishes_positive(self):
        for bad in [dict(reaped=False),dict(ended_monotonic=106.0),dict(partial=True),
                    dict(truncated=True),dict(timed_out=True),dict(returncode=True)]:
            with self.subTest(bad=bad),tempfile.TemporaryDirectory() as temp:
                out=Path(temp)/'proof'
                with self.assertRaises(ValueError):fake_observe(out,capture_patch=bad)
                refusal=json.loads((out/'refusal.json').read_text())
                self.assertIs(refusal['grant_authorized'],False)
                self.assertFalse((out/'observation.json').exists())


if __name__=='__main__':unittest.main()
