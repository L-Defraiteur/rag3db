"""Matrice V2 préparée avant code ; aucun transport/processus/secret réel."""
import copy
import json
import tempfile
from pathlib import Path
import unittest
import pg_v2 as v2


CANARY='G_V2_FAKE_SECRET_CANARY'


def allocation():
    return dict(schema_version=2,kind='pg-probe-allocation-v2',host_id='FAKE-PC',clock_domain='FAKE-PC-MONOTONIC',
                nonce='a'*32,scope='FAKE-SCOPE',source_sha256='1'*64,client_sha256='2'*64,
                target=dict(host='127.0.0.1',port=15433,database='g_pg_probe_fake',user='g_pg_observer'),
                expected_identity=dict(database='g_pg_probe_fake',user='g_pg_observer',session_user='g_pg_observer',
                database_oid=101,user_oid=102,owner='g_pg_fixture_owner',owner_oid=103,server_address='127.0.0.1',
                server_port=5432,server_version_num=170000,postmaster_start='FAKE-START'),
                reservation_reference='FAKE-RESERVATION',role_readonly_attested=True,private_test=True,
                common_end_monotonic=1100.,lease_end_monotonic=1100.)


def frames():
    identity=dict(allocation()['expected_identity'],backend_pid=201,application_name='g-pg-'+'a'*32)
    state=dict(transaction_read_only=True,default_transaction_read_only=True,superuser=False,createdb=False,
               createrole=False,replication=False,bypassrls=False,database_owner=False,
               user='g_pg_observer',database='g_pg_probe_fake',backend_pid=201)
    return [dict(event='connected',session='identity',cancel_token_acquired=True),
            dict(event='readonly',session='identity',state=state),dict(event='identity',row=identity),
            dict(event='closed',session='identity',driver_joined=True),
            dict(event='connected',session='postcheck',cancel_token_acquired=True),
            dict(event='readonly',session='postcheck',state={**state,'backend_pid':202}),
            dict(event='postcheck',row=dict(backend_pid=201,application_name='g-pg-'+'a'*32,backend_absent=True,
                 observer_backend_pid=202,observer_application_name='g-pg-'+'a'*32+':check')),
            dict(event='closed',session='postcheck',driver_joined=True),dict(event='complete',transport_qualified=False)]


class FakeTransport:
    def __init__(self,events=None,error=None,quiet=True,reaped=True):
        self.events=frames() if events is None else events;self.error=error
        self.quiet=quiet;self.reaped=reaped;self.calls=[]
    def capture(self,payload,deadline):
        self.calls.append(('capture',deadline))
        self.payload=payload
        for event in self.events:yield json.dumps(event).encode()+b'\n'
        if self.error:raise self.error
    def finish(self,deadline,*,stop):
        self.calls.append(('finish',deadline,stop))
        return dict(reaped=self.reaped,quiet=self.quiet,returncode=1 if stop else 0)


class V2Fixtures(unittest.TestCase):
    def setUp(self):
        self.tmp=tempfile.TemporaryDirectory();self.addCleanup(self.tmp.cleanup);self.out=Path(self.tmp.name)
        self.secret=v2.SecretInput(dsn='FAKE-DSN-'+CANARY,secret_fragments=(CANARY,),provider_id='FAKE-PROVIDER')
    def run_probe(self,transport=None,alloc=None,clock=None):
        return v2.observe(allocation() if alloc is None else alloc,self.secret,transport or FakeTransport(),
                          out=self.out,clock=clock or (lambda:1000.),scope_check=lambda:True)
    def test_v2_probe_success_is_not_admission_or_v1(self):
        result=self.run_probe()
        self.assertEqual(result['kind'],'pg-tool-probe-v2')
        self.assertFalse(result['transport_qualified']);self.assertFalse(result['lease_granted'])
    def test_secret_stdin_is_never_archived(self):
        self.run_probe()
        self.assertNotIn(CANARY,repr(self.secret))
        for p in self.out.iterdir():self.assertNotIn(CANARY,p.read_text())
    def test_secret_in_wire_refuses_and_keeps_only_redacted_trace(self):
        transport=FakeTransport(events=[dict(event='failure',code=CANARY)])
        with self.assertRaises(v2.Refused):self.run_probe(transport)
        for p in self.out.iterdir():self.assertNotIn(CANARY,p.read_text())
    def test_connect_timeout_before_pid_remains_unknown(self):
        transport=FakeTransport(events=[],error=TimeoutError(CANARY))
        with self.assertRaises(v2.Refused):self.run_probe(transport)
        red=json.loads((self.out/'first-red.json').read_text())
        self.assertFalse(red['server_attributed']);self.assertIsNone(red['backend_absent'])
        self.assertNotIn(CANARY,json.dumps(red))
    def test_readonly_and_role_must_be_observed_not_assumed(self):
        for key in ['transaction_read_only','superuser','createdb','database_owner']:
            with self.subTest(key=key):
                self.out=Path(self.tmp.name)/key;self.out.mkdir();events=frames()
                events[1]['state'][key]=not events[1]['state'][key]
                with self.assertRaises(v2.Refused):self.run_probe(FakeTransport(events))
    def test_allocation_must_be_external_private_and_nonowner(self):
        for key in ['private_test','role_readonly_attested']:
            with self.subTest(key=key):
                self.out=Path(self.tmp.name)/key;self.out.mkdir();a=allocation();a[key]=False
                transport=FakeTransport()
                with self.assertRaises(v2.Refused):self.run_probe(transport,alloc=a)
                self.assertEqual(transport.calls,[])
    def test_arbitrary_sql_or_unknown_frames_are_refused(self):
        with self.assertRaises(v2.Refused):self.run_probe(FakeTransport([dict(event='execute',sql='FAKE-NOT-EXECUTED')]))
    def test_backend_still_present_cannot_be_success(self):
        events=frames();events[6]['row']['backend_absent']=False
        with self.assertRaises(v2.Refused):self.run_probe(FakeTransport(events))
    def test_first_exception_stays_first_cleanup_uses_same_end(self):
        transport=FakeTransport(events=[],error=ValueError(CANARY))
        with self.assertRaises(v2.Refused):self.run_probe(transport)
        self.assertEqual(transport.calls,[('capture',1010.),('finish',1010.,True)])
        self.assertNotIn(CANARY,(self.out/'first-red.json').read_text())
    def test_unreaped_or_nonquiet_refuses(self):
        for name,kwargs in [('reap',dict(reaped=False)),('quiet',dict(quiet=False))]:
            with self.subTest(name=name):
                self.out=Path(self.tmp.name)/name;self.out.mkdir()
                with self.assertRaises(v2.Refused):self.run_probe(FakeTransport(**kwargs))
    def test_shared_deadline_covers_late_final_publication(self):
        now=[1000.]
        transport=FakeTransport()
        old_finish=transport.finish
        def finish(*args,**kwargs):now[0]=1011.;return old_finish(*args,**kwargs)
        transport.finish=finish
        with self.assertRaises(v2.Refused):self.run_probe(transport,clock=lambda:now[0])
    def test_duplicates_and_overflow_refuse(self):
        for name,raw in [('dup',b'{"event":"connected","event":"complete"}\n'),('big',b'x'*65537)]:
            with self.subTest(name=name):
                self.out=Path(self.tmp.name)/name;self.out.mkdir()
                transport=FakeTransport();transport.capture=lambda *a:iter([raw])
                with self.assertRaises(v2.Refused):self.run_probe(transport)


if __name__=='__main__':unittest.main(verbosity=2)
