"""Faux CPU purs : aucune connexion, aucun Docker, aucun processus/proc réel."""
import copy
import hashlib
import json
from pathlib import Path
import unittest

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


if __name__=='__main__':unittest.main(verbosity=2)
