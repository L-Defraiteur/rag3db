"""Bounded global ownership with real temp files/link, no contacts or proc reads."""
from pathlib import Path
import copy
import hashlib
import importlib.util
import json
import os
import tempfile
import unittest
from unittest.mock import patch
import global_services_owner as owner


class GlobalOwner(unittest.TestCase):
    def setUp(self):
        self.tmp=tempfile.TemporaryDirectory();self.addCleanup(self.tmp.cleanup)
        self.root=Path(self.tmp.name);self.now=80.;uid=os.getuid()
        source_root=Path(os.environ.get('GLOBAL_PG_SOURCE_ROOT',
            str(Path(__file__).resolve().parents[3]/'optimiseur/16-11-2026/pg-ro-attestation')))
        for name in ['pg_receipt.py','identity.sql','backend-postcheck.sql']:
            (self.root/name).write_bytes((source_root/name).read_bytes())
        shared_source=Path(os.environ.get('GLOBAL_HEALTH_SOURCE',
            str(Path(__file__).resolve().parents[3]/'orchestration/16-11-2026/coordination/provider_health_transport.py')))
        (self.root/'provider_health_transport.py').write_bytes(shared_source.read_bytes())
        (self.root/'global_services_owner.py').write_bytes(Path(owner.__file__).read_bytes())
        location=patch.object(owner,'__file__',str(self.root/'global_services_owner.py'))
        location.start();self.addCleanup(location.stop)
        spec=importlib.util.spec_from_file_location('test_admitted_g_receipt',self.root/'pg_receipt.py')
        self.pg=importlib.util.module_from_spec(spec);spec.loader.exec_module(self.pg)
        self.pins=dict(app='a'*40,pilot='b'*40,scope='/fixture/scope',c7_reference='C7-fixture',
            embar_reference='Embar-fixture',g_reference='G-fixture',reservation_reference='PG-slot-fixture',
            protocol_sha256=owner.sha(owner.phase_protocol.__file__),
            process_module_sha256=owner.sha(owner.process_observation.__file__),
            owner_sha256=owner.sha(owner.__file__),providers=[],
            transport_sha256=owner.sha(self.root/'provider_health_transport.py'),
            pg_receipt_sha256=owner.sha(self.root/'pg_receipt.py'),
            pg_identity_sql_sha256=owner.sha(self.root/'identity.sql'),
            pg_postcheck_sql_sha256=owner.sha(self.root/'backend-postcheck.sql'),
            admitted_monotonic=80.)
        for i,(address,(model,dimension,dual)) in enumerate(owner.PROVIDERS.items()):
            self.pins['providers'].append(dict(address=address,
                process=dict(pid=321+i,uid=uid,start=str(77+i),exe='/fixture/provider',exe_sha256='c'*64),
                identity=dict(service='rag3weaver-embeddings',modele=model,dim=dimension,
                    dual=dual,sparse=dual,factice=False,precision='Flex32',
                    executable='/fixture/provider@suffix',max_chars_per_text=12000)))
        self.pg_identity=dict(database='rag3weaver_test',user='rag3weaver',session_user='rag3weaver',
            database_oid=123,user_oid=42,owner='rag3weaver',owner_oid=42,
            server_port=None,server_address=None,server_version_num=170006,
            postmaster_start='fixture-start',backend_pid=700,application_name='g-pg-'+'a'*32)
        self.topology=dict(container_id='4'*64,image_id='sha256:'+'5'*64,
            binding=dict(host='127.0.0.1',host_port=5433,container_port=5432,protocol='tcp'))
        self.ready=dict(owner.phase_protocol.ready_bounds(40.,1000.),deadline=1000.,nonce='a'*32,
            app=self.pins['app'],pilot=self.pins['pilot'],scope=self.pins['scope'],
            c7_reference=self.pins['c7_reference'],service_operations_authorized=False,
            embedding_calls_authorized_only_by_canonical=True,
            requested=[*owner.PROVIDERS,'127.0.0.1:5433/rag3weaver_test'],
            phase=dict(job='lifecycle-provider',purpose='fts-global-models-and-pg',
                pilot=self.pins['pilot'],allocation='C7-fixture',first_red_path='/fixture/first-red',
                authority=dict(path='/fixture/authority',pid=123,start='42',cgroup='/fixture',
                               uid=uid,script_sha256='e'*64)))
        self.phase=dict(clock=owner.phase_protocol.READY_CLOCK,
            authority=dict(etat='actif',pid=123,debut='42',cgroup='/fixture',echeance=1000.,script_sha256='e'*64),
            authority_uid=uid,process=dict(uid=uid,start='42',cgroup='/fixture',state='S'),
            first_red_exists=False,snapshot_stable=True)
        self.path=self.root/'READY.json';self.path.write_text(json.dumps(self.ready));self.sha=owner.sha(self.path)
        def raw(name,value):
            body=value if isinstance(value,bytes) else json.dumps(value).encode()
            (self.root/name).write_bytes(body)
            return dict(path=str(self.root/name),sha256=hashlib.sha256(body).hexdigest())
        check=dict(backend_pid=700,application_name=self.pg_identity['application_name'],backend_absent=True,
                   observer_backend_pid=701,observer_application_name=self.pg_identity['application_name']+':check')
        self.reservation=dict(schema_version=1,kind='postgres-read-only-admission',
            app=self.pins['app'],pilot=self.pins['pilot'],scope=self.pins['scope'],
            c7_reference='C7-fixture',g_reference='G-fixture',reservation_reference='PG-slot-fixture',
            nonce='a'*32,ready_sha256=self.sha,exclusive=True,private_test=True,active=True,
            observed_monotonic=65.,valid_until_monotonic=1000.,topology=self.topology,
            sql=self.pg_identity,children_policy='per-suite-process-databases',
            evidence=dict(query_sha256=self.pg.QUERY_SHA256,observer_source_sha256='d'*64,
                transport='private-host-psql',client_bundle_sha256='e'*64,
                started_monotonic=60.,deadline_monotonic=65.,ended_monotonic=64.,returncode=0,
                partial=False,truncated=False,client_reaped=True,quiet=True,backend_absent=True,read_only=True,
                stdout=raw('sql.stdout',self.pg_identity),stderr=raw('sql.stderr',b''),
                postcheck=dict(query_sha256=self.pg.POSTCHECK_SHA256,started_monotonic=63.,
                    ended_monotonic=63.8,returncode=0,partial=False,truncated=False,
                    stdout=raw('check.stdout',check),stderr=raw('check.stderr',b''))))
        allocation={k:copy.deepcopy(self.reservation[k]) for k in ['app','pilot','scope','c7_reference',
            'g_reference','reservation_reference','nonce','ready_sha256','topology','children_policy']}
        allocation.update(tool_qualified=True,allocation_active=True,
            sql_identity={k:v for k,v in self.pg_identity.items() if k not in ['backend_pid','application_name']},
            observer_source_sha256='d'*64,client_bundle_sha256='e'*64)
        self.pins['postgres']=dict(host='127.0.0.1',port=5433,database='rag3weaver_test',
                                  user='rag3weaver',allocation=allocation)
        self.respath=self.root/'PG.json';self.respath.write_text(json.dumps(self.reservation));self.ressha=owner.sha(self.respath)
        self.dest=self.root/'SERVICES_GO.json';self.evidence=self.root/'EVIDENCE.json'
        self.contacts=[];self.after_contact=lambda context:None
        self.processes={p['process']['pid']:dict(p['process'],state='S') for p in self.pins['providers']}
        self.pins.update(ready_path=str(self.path),ready_sha256=self.sha,nonce='a'*32,
                         reservation_path=str(self.respath),reservation_sha256=self.ressha)

    def phase_reader(self,ready):
        return dict(copy.deepcopy(self.phase),snapshot_observed_monotonic=self.now)
    def process_reader(self,pid):return copy.deepcopy(self.processes[pid])
    def contact(self,context,trace):
        self.contacts.append(context['address'])
        trace.update(stdout_hex=b'fake worker stdout'.hex(),stderr_hex='',returncode=0)
        provider=next(p for p in self.pins['providers'] if p['address']==context['address'])
        raw=json.dumps(dict(provider['identity'],usage=dict(baux=0,en_cours=0))).encode()
        self.after_contact(context)
        return dict(status=200,body=raw)
    def publish(self):
        return owner.publish_services(self.path,self.dest,expected_ready_sha256=self.sha,
            expected_nonce='a'*32,pins=self.pins,reservation_path=self.respath,
            reservation_sha256=self.ressha,evidence_path=self.evidence,
            clock=lambda:self.now,phase_reader=self.phase_reader,
            process_reader=self.process_reader,contact=self.contact)
    def refused(self):
        with self.assertRaises(ValueError):self.publish()
        self.assertFalse(self.dest.exists())
        self.assertEqual(list(self.root.glob('.services-*')),[])

    def test_joint_receipt_real_link_no_lifecycle_flag_and_end_lease(self):
        grant=self.publish()
        self.assertEqual(self.contacts,list(owner.PROVIDERS))
        self.assertEqual(grant['valid_until_monotonic'],1000.)
        self.assertNotIn('no_embedding_calls_authorized',grant)
        self.assertEqual(grant['owner_pins_sha256'],owner.pins_digest(grant['owner_pins']))
        self.assertEqual(owner.owned_json(self.dest),grant)
        self.assertEqual(grant['postgres']['identity'],self.pg_identity)
        evidence=owner.owned_json(self.evidence)
        self.assertEqual(len(evidence['contacts']),2)
        self.assertTrue(evidence['pg_attestation']['evidence']['read_only'])

    def test_phase_red_between_providers_prevents_second_contact(self):
        self.after_contact=lambda context:self.phase.update(first_red_exists=True)
        self.refused();self.assertEqual(len(self.contacts),1)

    def test_second_provider_late_contact_has_no_publication(self):
        def delay(context):
            if context['address'].endswith('7879'):self.now+=6
        self.after_contact=delay;self.refused()

    def test_process_identity_changed_during_health_is_refused(self):
        self.after_contact=lambda context:self.processes[321].update(start='reused')
        self.refused()

    def test_delayed_fsync_does_not_publish_after_cutoff(self):
        original=owner.stage
        def delayed(*args):
            path=original(*args);self.now=160.;return path
        with patch.object(owner,'stage',side_effect=delayed):self.refused()

    def test_first_red_during_fsync_is_immutable_and_refuses_link(self):
        original=owner.stage;red=self.root/'first-red'
        def delayed(*args):
            path=original(*args);red.write_bytes(b'first immutable red')
            self.phase['first_red_exists']=True;return path
        with patch.object(owner,'stage',side_effect=delayed):self.refused()
        self.assertEqual(red.read_bytes(),b'first immutable red')

    def test_ready_changed_during_fsync_refuses_link(self):
        original=owner.stage
        def changed(*args):
            path=original(*args);self.path.write_text(json.dumps(dict(self.ready,nonce='foreign')));return path
        with patch.object(owner,'stage',side_effect=changed):self.refused()

    def test_pg_reservation_changed_during_fsync_refuses_link(self):
        original=owner.stage
        def changed(*args):
            path=original(*args);self.respath.write_text(json.dumps(dict(self.reservation,active=False)));return path
        with patch.object(owner,'stage',side_effect=changed):self.refused()

    def test_absent_or_stale_pg_reservation_prevents_any_health(self):
        self.reservation['observed_monotonic']=100.
        self.respath.write_text(json.dumps(self.reservation));self.ressha=owner.sha(self.respath)
        self.refused();self.assertEqual(self.contacts,[])

    def test_pg_tcp_ready_without_sql_identity_is_not_attestation(self):
        self.reservation['evidence'].pop('stdout')
        self.respath.write_text(json.dumps(self.reservation));self.ressha=owner.sha(self.respath)
        with self.assertRaises((ValueError,KeyError)):self.publish()
        self.assertFalse(self.dest.exists());self.assertEqual(self.contacts,[])

    def test_pg_wrong_database_oid_or_owner_is_refused(self):
        wrong=dict(self.pg_identity,database_oid=999)
        self.reservation['sql']=wrong
        (self.root/'sql.stdout').write_text(json.dumps(wrong))
        self.reservation['evidence']['stdout']['sha256']=owner.sha(self.root/'sql.stdout')
        self.respath.write_text(json.dumps(self.reservation));self.ressha=owner.sha(self.respath)
        self.refused();self.assertEqual(self.contacts,[])

    def test_pg_binding_is_not_inferred_from_sql_port(self):
        self.reservation['topology']=copy.deepcopy(self.topology)
        self.reservation['topology']['binding']['host_port']=5432
        self.respath.write_text(json.dumps(self.reservation));self.ressha=owner.sha(self.respath)
        self.refused();self.assertEqual(self.contacts,[])

    def test_lease_does_not_cover_end_refuses_before_contacts(self):
        self.reservation['valid_until_monotonic']=999.
        self.respath.write_text(json.dumps(self.reservation));self.ressha=owner.sha(self.respath)
        self.refused();self.assertEqual(self.contacts,[])

    def test_existing_destination_is_never_overwritten_or_reobserved(self):
        self.dest.write_bytes(b'first receipt')
        with self.assertRaises(FileExistsError):self.publish()
        self.assertEqual(self.dest.read_bytes(),b'first receipt');self.assertEqual(self.contacts,[])

    def test_postcanonical_health_uses_active_lease_without_readmission(self):
        self.now=200.
        evidence={}
        observed=owner.health_observation(self.ready,self.pins,self.pins['providers'][0],evidence,
            admission=False,lease_end=1000.,clock=lambda:self.now,phase_reader=self.phase_reader,
            process_reader=self.process_reader,contact=self.contact)
        self.assertEqual(observed['modele'],'granite-278m')
        with self.assertRaises(ValueError):
            owner.health_observation(self.ready,self.pins,self.pins['providers'][1],{},
                admission=True,lease_end=1000.,clock=lambda:self.now,phase_reader=self.phase_reader,
                process_reader=self.process_reader,contact=self.contact)

    def test_postcanonical_expired_lease_or_red_never_contacts(self):
        self.now=1000.
        with self.assertRaises(ValueError):owner.health_observation(self.ready,self.pins,self.pins['providers'][0],{},
            admission=False,lease_end=1000.,clock=lambda:self.now,phase_reader=self.phase_reader,
            process_reader=self.process_reader,contact=self.contact)
        self.assertEqual(self.contacts,[])

    def test_raw_evidence_survives_contact_failure_without_grant(self):
        def refuse(context,trace):
            trace.update(stdout_hex=b'partial health'.hex(),stderr_hex=b'timeout'.hex(),returncode=-9)
            raise ValueError('fake timeout')
        self.contact=refuse;self.refused()
        evidence=owner.owned_json(self.evidence)
        self.assertEqual(bytes.fromhex(evidence['contacts'][0]['trace']['stdout_hex']),b'partial health')


class SharedTransport(unittest.TestCase):
    def fixture(self):
        f=GlobalOwner('test_joint_receipt_real_link_no_lifecycle_flag_and_end_lease')
        f.setUp();self.addCleanup(f.doCleanups)
        context=dict(pins=f.pins,ready_path=str(f.path),ready_sha256=f.sha,nonce='a'*32,
            kind='provider',address='127.0.0.1:7879',admission=True,lease_end=1000.,contact_deadline=85.)
        return f,context

    def test_real_worker_is_bge_health_only_not_7878_lifecycle_or_embedding(self):
        from unittest.mock import Mock
        f,context=self.fixture();connection=Mock()
        connection.getresponse.return_value.status=200
        connection.getresponse.return_value.read1.side_effect=[b'{"modele":"bge-m3"}',b'']
        with patch.object(owner.http.client,'HTTPConnection',return_value=connection) as http, \
             patch.object(owner.phase_protocol,'read_phase_state',side_effect=f.phase_reader), \
             patch.object(owner.process_observation,'process_snapshot',side_effect=f.process_reader), \
             patch.object(owner.time,'monotonic',return_value=80.),patch('builtins.print'):
            owner.worker(context)
        http.assert_called_once_with('127.0.0.1',7879,timeout=4.5)
        connection.request.assert_called_once_with('GET','/sante')
        connection.close.assert_called_once()

    def test_worker_first_red_is_checked_before_any_http(self):
        f,context=self.fixture();f.phase['first_red_exists']=True
        with patch.object(owner.http.client,'HTTPConnection') as http, \
             patch.object(owner.process_observation,'process_snapshot',side_effect=f.process_reader), \
             patch.object(owner.phase_protocol,'read_phase_state',side_effect=f.phase_reader), \
             patch.object(owner.time,'monotonic',return_value=80.):
            with self.assertRaises(ValueError):owner.worker(context)
        http.assert_not_called()

    def test_worker_unknown_provider_or_pg_probe_cannot_open_http(self):
        f,context=self.fixture();context['kind']='postgres'
        with patch.object(owner.http.client,'HTTPConnection') as http:
            with self.assertRaises(ValueError):owner.worker(context)
        http.assert_not_called()

    def test_transport_timeout_preserves_partial_output_and_reaps(self):
        from unittest.mock import Mock
        f,context=self.fixture();trace={}
        error=owner.subprocess.TimeoutExpired('fake',5,output=b'partial',stderr=b'early')
        child=Mock(pid=999,returncode=-9)
        child.communicate.side_effect=[error,(b'partial and tail',b'early and tail')]
        with patch.object(owner.subprocess,'Popen',return_value=child), \
             patch.object(owner.time,'monotonic',return_value=80.):
            with self.assertRaises(owner.subprocess.TimeoutExpired):owner.transport(context,trace)
        child.kill.assert_called_once();self.assertEqual(child.communicate.call_count,2)
        self.assertEqual(trace['returncode'],-9)
        self.assertEqual(bytes.fromhex(trace['stdout_hex']),b'partial and tail')
        self.assertEqual(bytes.fromhex(trace['timeout_stdout_hex']),b'partial')
        self.assertTrue(trace['timed_out'])

    def test_nonzero_worker_retains_all_raw_without_grant(self):
        from unittest.mock import Mock
        f,context=self.fixture();trace={};child=Mock(pid=999,returncode=1)
        child.communicate.return_value=(b'health prefix',b'worker error')
        with patch.object(owner.subprocess,'Popen',return_value=child), \
             patch.object(owner.time,'monotonic',return_value=80.):
            with self.assertRaises(ValueError):owner.transport(context,trace)
        self.assertEqual(bytes.fromhex(trace['stdout_hex']),b'health prefix')
        self.assertEqual(bytes.fromhex(trace['stderr_hex']),b'worker error')
        self.assertEqual(trace['returncode'],1)


if __name__=='__main__':unittest.main()
