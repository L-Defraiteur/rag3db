"""Témoins publiés avant CPU. Aucun PG, psql, Docker, réseau ou secret réel.

Le module runner.py est volontairement absent à la publication rouge initiale.
Les processus natifs de test sont uniquement des enfants Python privés.
"""
import copy
import importlib.util
import json
import os
from pathlib import Path
import sys
import tempfile
import time
import unittest

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


class RealCapture(unittest.TestCase):
    def call(self,code,*,stdin=b'',seconds=1,env=None):
        return runner.capture([sys.executable,'-I','-c',code],
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
            runner.capture([sys.executable,'-I','-c','import time;time.sleep(30)'],
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
        with self.assertRaises(ValueError):
            runner.capture([sys.executable,'-I','-c','raise Exception("must not run")'],
                env={},stdin=b'',deadline=time.monotonic()-1)


def fake_observe(out, *, expire=False):
    # Même raccord observe/publisher que le client réel ; seul le transport est faux.
    now=[100.0]; p=profile();ready=dict(active=True,first_red=False,common_end=120.0)
    def factory(deadline):
        def session(identity=None):
            ro,row=rows(p,check=identity is not None,identity=identity)
            now[0]+=.1
            return dict(stdout=encode(ro,row),stderr=b'',returncode=0,reaped=True,
                partial=False,truncated=False,timed_out=False,pid=900+(identity is not None),
                started_monotonic=now[0]-.1,ended_monotonic=now[0])
        return session
    calls=[0]
    def phase_check():
        calls[0]+=1
        if expire and calls[0]>=4:now[0]=106.0
        return ready.copy()
    return runner.observe(p,out,session_factory=factory,clock=lambda:now[0],phase_check=phase_check)


class WholeJob(unittest.TestCase):
    """Observe -> capture injectée -> validation -> vrai publisher fichiers."""
    def test_success_publishes_new_fixture_kind_not_v1_grant(self):
        with tempfile.TemporaryDirectory() as temp:
            r=fake_observe(Path(temp)/'proof')
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


if __name__=='__main__':unittest.main()
