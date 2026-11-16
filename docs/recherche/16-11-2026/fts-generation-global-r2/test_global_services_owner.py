"""Bounded global ownership with real temp files/link, no contacts or proc reads."""
from pathlib import Path
import copy
import hashlib
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
        self.pins=dict(app='a'*40,pilot='b'*40,scope='/fixture/scope',c7_reference='C7-fixture',
            embar_reference='Embar-fixture',g_reference='G-fixture',reservation_reference='PG-slot-fixture',
            protocol_sha256=owner.sha(owner.phase_protocol.__file__),
            process_module_sha256=owner.sha(owner.process_observation.__file__),
            owner_sha256=owner.sha(owner.__file__),providers=[])
        for i,(address,(model,dimension,dual)) in enumerate(owner.PROVIDERS.items()):
            self.pins['providers'].append(dict(address=address,
                process=dict(pid=321+i,uid=uid,start=str(77+i),exe='/fixture/provider',exe_sha256='c'*64),
                identity=dict(service='rag3weaver-embeddings',modele=model,dim=dimension,
                    dual=dual,sparse=dual,factice=False,precision='Flex32',
                    executable='/fixture/provider@suffix',max_chars_per_text=12000)))
        self.pg_identity=dict(database='rag3weaver_test',user='rag3weaver',database_oid=123,
            user_oid=42,database_owner='rag3weaver',database_owner_oid=42,
            server_port=None,server_address=None)
        self.topology=dict(host='127.0.0.1',host_port=5433,container_port=5432,
                           container_id='fixture-container',image_id='sha256:fixture-image')
        self.pins['postgres']=dict(host='127.0.0.1',port=5433,database='rag3weaver_test',
            user='rag3weaver',identity=self.pg_identity,topology=self.topology,observer_source_sha256='d'*64)
        self.ready=dict(owner.phase_protocol.ready_bounds(40.,1000.),deadline=1000.,nonce='nonce',
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
        self.reservation=dict(kind='exclusive-private-pg-bootstrap',state='active',
            app=self.pins['app'],pilot=self.pins['pilot'],scope=self.pins['scope'],
            c7_reference='C7-fixture',g_reference='G-fixture',reference='PG-slot-fixture',
            nonce='nonce',ready_sha256=self.sha,exclusive_reservation=True,private_test_only=True,
            observed_monotonic=60.,valid_until_monotonic=1000.,topology=self.topology,
            observer_source_sha256='d'*64,sql_read_only=True,sql_returncode=0,observer_rc=0,
            observation_quiescent=True,sql_observed_monotonic=55.,
            sql_stdout_hex=json.dumps(self.pg_identity).encode().hex(),sql_stderr_hex='')
        self.respath=self.root/'PG.json';self.respath.write_text(json.dumps(self.reservation));self.ressha=owner.sha(self.respath)
        self.dest=self.root/'SERVICES_GO.json';self.evidence=self.root/'EVIDENCE.json'
        self.contacts=[];self.after_contact=lambda context:None
        self.processes={p['process']['pid']:dict(p['process'],state='S') for p in self.pins['providers']}
        self.pins.update(ready_path=str(self.path),ready_sha256=self.sha,nonce='nonce',
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
            expected_nonce='nonce',pins=self.pins,reservation_path=self.respath,
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
        self.assertTrue(evidence['pg_attestation']['sql_read_only'])

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
            path=original(*args);self.respath.write_text(json.dumps(dict(self.reservation,state='revoked')));return path
        with patch.object(owner,'stage',side_effect=changed):self.refused()

    def test_absent_or_stale_pg_reservation_prevents_any_health(self):
        self.reservation['observed_monotonic']=100.
        self.respath.write_text(json.dumps(self.reservation));self.ressha=owner.sha(self.respath)
        self.refused();self.assertEqual(self.contacts,[])

    def test_pg_tcp_ready_without_sql_identity_is_not_attestation(self):
        self.reservation.pop('sql_stdout_hex')
        self.respath.write_text(json.dumps(self.reservation));self.ressha=owner.sha(self.respath)
        with self.assertRaises((ValueError,KeyError)):self.publish()
        self.assertFalse(self.dest.exists());self.assertEqual(self.contacts,[])

    def test_pg_wrong_database_oid_or_owner_is_refused(self):
        wrong=dict(self.pg_identity,database_oid=999)
        self.reservation['sql_stdout_hex']=json.dumps(wrong).encode().hex()
        self.respath.write_text(json.dumps(self.reservation));self.ressha=owner.sha(self.respath)
        self.refused();self.assertEqual(self.contacts,[])

    def test_pg_binding_is_not_inferred_from_sql_port(self):
        self.reservation['topology']=dict(self.topology,host_port=5432)
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


if __name__=='__main__':unittest.main()
