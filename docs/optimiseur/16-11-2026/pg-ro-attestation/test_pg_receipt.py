"""Faux CPU purs : aucune connexion, aucun Docker, aucun processus/proc réel."""
import copy
import hashlib
import json
from pathlib import Path
import unittest
import tempfile
from unittest.mock import patch

import pg_receipt as pg


def digest(value):
    return hashlib.sha256(value).hexdigest()


class ReceiptTests(unittest.TestCase):
    def setUp(self):
        nonce = 'a'*32
        sql = dict(database='rag3weaver_test', user='rag3weaver', session_user='rag3weaver',
                   database_oid=42, user_oid=10, owner='rag3weaver', owner_oid=10,
                   server_address=None, server_port=None, server_version_num=170006,
                   postmaster_start='fixture-start', backend_pid=700, application_name='g-pg-'+nonce)
        check = dict(backend_pid=700,application_name='g-pg-'+nonce,backend_absent=True,
                     observer_backend_pid=701,observer_application_name='g-pg-'+nonce+':check')
        self.raw = {'sql-out':json.dumps(sql).encode(), 'sql-err':b'',
                    'check-out':json.dumps(check).encode(), 'check-err':b''}
        def ref(name):return dict(path=name,sha256=digest(self.raw[name]))
        self.body = dict(schema_version=1,kind='postgres-read-only-admission',app='1'*40,
                         pilot='2'*40,scope='/fixture/scope',c7_reference='fixture-C7',
                         g_reference='fixture-G',reservation_reference='fixture-reservation',
                         nonce=nonce,ready_sha256='3'*64,observed_monotonic=999.5,
                         valid_until_monotonic=4000.,exclusive=True,private_test=True,active=True,
                         topology=dict(container_id='4'*64,image_id='sha256:'+'5'*64,
                                       binding=dict(host='127.0.0.1',host_port=5433,container_port=5432,protocol='tcp')),
                         sql=sql,children_policy='per-suite-process-databases',
                         evidence=dict(query_sha256=pg.QUERY_SHA256,observer_source_sha256='6'*64,
                                       transport='private-host-psql',client_bundle_sha256='7'*64,
                                       started_monotonic=995.,deadline_monotonic=1000.,ended_monotonic=999.,
                                       returncode=0,partial=False,truncated=False,client_reaped=True,
                                       quiet=True,backend_absent=True,read_only=True,
                                       stdout=ref('sql-out'),stderr=ref('sql-err'),
                                       postcheck=dict(query_sha256=pg.POSTCHECK_SHA256,
                                                      started_monotonic=998.,ended_monotonic=998.8,
                                                      returncode=0,partial=False,truncated=False,
                                                      stdout=ref('check-out'),stderr=ref('check-err'))))
        self.expected = {k:copy.deepcopy(self.body[k]) for k in ['app','pilot','scope','c7_reference','g_reference','reservation_reference','nonce','ready_sha256','topology','children_policy']}
        self.expected.update(tool_qualified=True,allocation_active=True,
                             sql_identity={k:v for k,v in sql.items() if k not in ['backend_pid','application_name']},
                             observer_source_sha256='6'*64,client_bundle_sha256='7'*64)
        self.ready=dict(issued_monotonic=900.,admission_deadline_monotonic=1020.,common_end_monotonic=4000.,active=True,first_red=False)

    def validate(self):
        return pg.validate_receipt(self.body,expected=self.expected,ready=self.ready,
                                   now=1000.,read_raw=lambda path:self.raw[path])

    def reject(self):
        with self.assertRaises(ValueError):self.validate()

    def test_valid_fake_is_accepted(self):
        self.assertEqual(self.validate(),self.body)

    def test_unknown_or_unqualified_tool_refuses(self):
        for key in ['tool_qualified','allocation_active']:
            with self.subTest(key=key):
                self.setUp();self.expected[key]=False;self.reject()

    def test_all_external_bindings_are_exact(self):
        for key in ['app','pilot','scope','c7_reference','g_reference','reservation_reference','nonce','ready_sha256']:
            with self.subTest(key=key):
                self.setUp();self.body[key]='wrong';self.reject()

    def test_names_oids_and_owner_are_exact_not_boolean(self):
        for key,value in [('database','wrong'),('user','wrong'),('database_oid',True),('user_oid',11),('owner','wrong'),('owner_oid',11)]:
            with self.subTest(key=key):
                self.setUp();self.body['sql'][key]=value;self.reject()

    def test_topology_is_not_a_tcp_ping(self):
        for field in ['container_id','image_id']:
            with self.subTest(field=field):
                self.setUp();self.body['topology'][field]='unknown';self.reject()
        self.setUp();self.body['topology']['binding']['container_port']=5433;self.reject()

    def test_flags_and_lease_are_strict(self):
        for key in ['active','private_test','exclusive']:
            for value in [False,1]:
                with self.subTest(key=key,value=value):
                    self.setUp();self.body[key]=value;self.reject()
        self.setUp();self.body['valid_until_monotonic']=3999.;self.reject()
        self.setUp();self.body['observed_monotonic']=800.;self.reject()
        self.setUp();self.body['observed_monotonic']=1001.;self.reject()

    def test_whole_budget_quiet_and_partial_are_strict(self):
        for key,value in [('deadline_monotonic',1001.),('client_reaped',False),('quiet',False),('backend_absent',False),('partial',True),('truncated',True),('read_only',False),('returncode',True),('transport','docker-exec')]:
            with self.subTest(key=key):
                self.setUp();self.body['evidence'][key]=value;self.reject()

    def test_raw_sha_and_typed_sql_must_match(self):
        self.body['evidence']['stdout']['sha256']='0'*64;self.reject()
        self.setUp();self.raw['sql-out']=self.raw['sql-out'].replace(b'42',b'43')
        self.body['evidence']['stdout']['sha256']=digest(self.raw['sql-out']);self.reject()
        self.setUp();self.raw['sql-err']=b'ERROR';self.body['evidence']['stderr']['sha256']=digest(self.raw['sql-err']);self.reject()

    def test_postcheck_backend_identity_and_budget_are_exact(self):
        for key,value in [('backend_absent',False),('backend_pid',702),('observer_backend_pid',700),('application_name','other')]:
            with self.subTest(key=key):
                self.setUp();raw=json.loads(self.raw['check-out']);raw[key]=value
                self.raw['check-out']=json.dumps(raw).encode()
                self.body['evidence']['postcheck']['stdout']['sha256']=digest(self.raw['check-out']);self.reject()
        self.setUp();self.body['evidence']['postcheck']['ended_monotonic']=1005.;self.reject()

    def test_unknown_schema_or_duplicate_raw_key_refuses(self):
        self.body['extra']='ignored';self.reject()
        self.setUp();self.body['evidence']['postcheck']['extra']=True;self.reject()
        self.setUp();self.raw['sql-out']=self.raw['sql-out'].replace(b'{',b'{"database_oid":42,',1)
        self.body['evidence']['stdout']['sha256']=digest(self.raw['sql-out']);self.reject()

    def test_phase_first_red_cannot_be_revived(self):
        self.ready['first_red']=True;self.reject()
        self.setUp();self.ready['active']=False;self.reject()


class ArtifactTests(unittest.TestCase):
    def setUp(self):
        ReceiptTests.setUp(self)
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name).resolve()

    def write_artifact(self):
        for role, ref in [('sql-out',self.body['evidence']['stdout']),('sql-err',self.body['evidence']['stderr']),
                          ('check-out',self.body['evidence']['postcheck']['stdout']),('check-err',self.body['evidence']['postcheck']['stderr'])]:
            path=self.root/role;path.write_bytes(self.raw[role]);ref['path']=str(path)
        path=self.root/'attestation.json';raw=json.dumps(self.body).encode();path.write_bytes(raw)
        return path,digest(raw)

    def test_loaded_artifact_is_exact(self):
        path,sha=self.write_artifact()
        self.assertEqual(pg.validate_attestation(path,sha,self.expected,self.ready,1000.),self.body)

    def test_loaded_sha_symlink_and_outside_raw_are_refused(self):
        path,sha=self.write_artifact()
        with self.assertRaises(ValueError):pg.validate_attestation(path,'0'*64,self.expected,self.ready,1000.)
        link=self.root/'linked.json';link.symlink_to(path)
        with self.assertRaises(ValueError):pg.validate_attestation(link,sha,self.expected,self.ready,1000.)
        self.body['evidence']['stdout']['path']=str(self.root.parent/'outside')
        path.write_text(json.dumps(self.body))
        with self.assertRaises(ValueError):pg.validate_attestation(path,digest(path.read_bytes()),self.expected,self.ready,1000.)

    def test_post_admission_reuses_trusted_original_time(self):
        path,sha=self.write_artifact()
        result=pg.validate_attestation(path,sha,self.expected,self.ready,2000.,admission=False,admitted_at=1000.)
        self.assertEqual(result,self.body)
        for admitted in [None,1100.,2001.]:
            with self.assertRaises(ValueError):pg.validate_attestation(path,sha,self.expected,self.ready,2000.,admission=False,admitted_at=admitted)
        with self.assertRaises(ValueError):pg.validate_attestation(path,sha,self.expected,self.ready,2000.)

    def test_post_admission_expired_lease_or_first_red_refuses(self):
        path,sha=self.write_artifact()
        with self.assertRaises(ValueError):pg.validate_attestation(path,sha,self.expected,self.ready,4001.,admission=False,admitted_at=1000.)
        self.ready['first_red']=True
        with self.assertRaises(ValueError):pg.validate_attestation(path,sha,self.expected,self.ready,2000.,admission=False,admitted_at=1000.)

    def test_prepare_is_explicit_hold_without_probe(self):
        plan=pg.prepare_observation()
        self.assertEqual(plan['status'],'HOLD')
        self.assertIs(plan['run_authorized'],False)
        self.assertIs(plan['transport_qualified'],False)

    def test_offline_publisher_preserves_immutable_raw_before_refusal(self):
        self.body['evidence']['partial']=True
        target=self.root/'refusal'
        with self.assertRaises(ValueError):
            pg.publish_observation(target,self.body,self.raw,expected=self.expected,ready=self.ready,
                                   clock=lambda:1000.,phase_check=lambda:dict(self.ready))
        self.assertTrue((target/'refusal.json').is_file())
        self.assertEqual((target/'identity.stdout').read_bytes(),self.raw['sql-out'])
        self.assertFalse((target/'attestation.json').exists())
        before={p.name:p.read_bytes() for p in target.iterdir()}
        with self.assertRaises(FileExistsError):
            pg.publish_observation(target,self.body,self.raw,expected=self.expected,ready=self.ready,
                                   clock=lambda:1000.,phase_check=lambda:dict(self.ready))
        self.assertEqual(before,{p.name:p.read_bytes() for p in target.iterdir()})

    def test_publisher_requires_phase_after_fsync_and_single_deadline(self):
        calls=[]
        def phase():
            calls.append(True)
            return dict(self.ready,first_red=len(calls)>1)
        target=self.root/'late-red'
        with self.assertRaises(ValueError):
            pg.publish_observation(target,self.body,self.raw,expected=self.expected,ready=self.ready,
                                   clock=lambda:1000.,phase_check=phase)
        self.assertGreaterEqual(len(calls),2)
        self.assertFalse((target/'attestation.json').exists())
        with self.assertRaises(ValueError):
            pg.publish_observation(self.root/'late-clock',self.body,self.raw,expected=self.expected,ready=self.ready,
                                   clock=lambda:1001.,phase_check=lambda:dict(self.ready))

    def test_publisher_keeps_raw_even_when_metadata_is_malformed(self):
        del self.body['evidence']['postcheck']
        target=self.root/'malformed'
        with self.assertRaises((ValueError,KeyError)):
            pg.publish_observation(target,self.body,self.raw,expected=self.expected,ready=self.ready,
                                   clock=lambda:1000.,phase_check=lambda:dict(self.ready))
        for name,key in [('identity.stdout','sql-out'),('identity.stderr','sql-err'),('postcheck.stdout','check-out'),('postcheck.stderr','check-err')]:
            self.assertTrue((target/name).is_file(), "raw lost before metadata refusal")
            self.assertEqual((target/name).read_bytes(),self.raw[key])
        self.assertFalse((target/'attestation.json').exists())

    def test_publisher_keeps_available_partial_raw(self):
        del self.raw['check-err']
        target=self.root/'missing-raw'
        with self.assertRaises((ValueError,KeyError)):
            pg.publish_observation(target,self.body,self.raw,expected=self.expected,ready=self.ready,
                                   clock=lambda:1000.,phase_check=lambda:dict(self.ready))
        self.assertTrue((target/'identity.stdout').is_file(), 'available raw lost')
        self.assertEqual((target/'identity.stdout').read_bytes(),self.raw['sql-out'])
        self.assertFalse((target/'attestation.json').exists())

    def test_last_fsync_crossing_deadline_is_red_with_published_artifact(self):
        current=[1000.];calls=[]
        original=pg._sync_directory
        def sync(path):
            original(path);calls.append(True)
            if len(calls)==2:current[0]=1001.
        target=self.root/'last-fsync-late'
        with patch.object(pg,'_sync_directory',sync):
            with self.assertRaises(ValueError):
                pg.publish_observation(target,self.body,self.raw,expected=self.expected,ready=self.ready,
                                       clock=lambda:current[0],phase_check=lambda:dict(self.ready))
        self.assertEqual((target/'identity.stdout').read_bytes(),self.raw['sql-out'])
        self.assertTrue((target/'attestation.json').is_file())
        refusal=json.loads((target/'refusal.json').read_text())
        self.assertIs(refusal['artifact_published'],True)
        self.assertIs(refusal['grant_authorized'],False)
        path=target/'attestation.json'
        with self.assertRaises(ValueError):
            pg.validate_attestation(path,digest(path.read_bytes()),self.expected,self.ready,1001.)

    def test_first_red_at_last_fsync_is_not_returned_as_success(self):
        calls=[];original=pg._sync_directory
        def sync(path):
            original(path);calls.append(True)
            if len(calls)==2:self.ready['first_red']=True
        target=self.root/'last-fsync-first-red'
        with patch.object(pg,'_sync_directory',sync):
            with self.assertRaises(ValueError):
                pg.publish_observation(target,self.body,self.raw,expected=self.expected,ready=self.ready,
                                       clock=lambda:1000.,phase_check=lambda:dict(self.ready))
        self.assertTrue((target/'refusal.json').is_file())
        self.assertTrue((target/'attestation.json').is_file())

    def test_loader_refuses_a_previously_published_then_rejected_json(self):
        path,sha=self.write_artifact()
        (self.root/'refusal.json').write_text('{"verdict":"RED","artifact_published":true}')
        with self.assertRaises(ValueError):
            pg.validate_attestation(path,sha,self.expected,self.ready,1000.)

    def test_publisher_valid_fake_is_loadable(self):
        path,sha=pg.publish_observation(self.root/'ok',self.body,self.raw,expected=self.expected,ready=self.ready,
                                       clock=lambda:1000.,phase_check=lambda:dict(self.ready))
        result=pg.validate_attestation(path,sha,self.expected,self.ready,1000.)
        self.assertEqual(result['nonce'],self.body['nonce'])


if __name__=='__main__':unittest.main(verbosity=2)
