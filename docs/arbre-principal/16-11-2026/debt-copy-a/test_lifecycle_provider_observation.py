"""Owner caller through unchanged producer/link; no network, proc or native DB."""
from pathlib import Path
import copy
import hashlib
import json
import os
import tempfile
import unittest
from unittest.mock import patch
import lifecycle_provider_observation as owner
import lifecycle_protocol as protocol
import lifecycle_grant_producer as producer


class Observation(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)
        self.now = 80.
        uid = os.getuid()
        self.ready = dict(protocol.ready_bounds(40., 1000.), deadline=1000., nonce='nonce',
            no_embedding_calls_authorized=True, phase=dict(job='lifecycle-provider',
            pilot='f'*40, allocation='C7-fixture', first_red_path='/fixture/first-red.json',
            authority=dict(path='/fixture/auth', pid=123, start='42', cgroup='/fixture',
                           uid=uid, script_sha256='a'*64)))
        self.phase = dict(clock=protocol.READY_CLOCK,
            authority=dict(etat='actif', pid=123, debut='42', cgroup='/fixture',
                           echeance=1000., script_sha256='a'*64), authority_uid=uid,
            process=dict(start='42', cgroup='/fixture', uid=uid, state='S'),
            first_red_exists=False, snapshot_stable=True)
        self.process = dict(pid=321, uid=uid, start='77', exe='/fixture/provider',
                            exe_sha256='b'*64, state='S')
        identity = dict(service='rag3weaver-embeddings', modele='granite-278m', dim=768,
            factice=False, dual=False, sparse=False, precision='Flex32',
            executable='/fixture/provider@suffix', max_chars_per_text=12000)
        self.health = dict(identity, usage=dict(baux=0, en_cours=0))
        self.pins = dict(observation_sha256=owner.digest(owner.__file__),
            protocol_sha256=owner.digest(protocol.__file__),
            producer_sha256=owner.digest(producer.__file__), pilot='f'*40,
            allocation='C7-fixture', c7_reference='C7-fixture', embar_reference='Embar-fixture',
            identity=identity, expected_process={k:v for k,v in self.process.items() if k!='state'})
        self.path = self.root/'READY.json'
        self.path.write_text(json.dumps(self.ready))
        self.sha = owner.digest(self.path)
        self.dest = self.root/'PROVIDER_GO.json'
        self.evidence = self.root/'observation.json'
        self.gets = 0
        self.proc_reads = 0
        self.after_get = lambda: None
        self.before_get = lambda: None
        self.status = 200

    def phase_reader(self, ready):
        return dict(copy.deepcopy(self.phase), snapshot_observed_monotonic=self.now)

    def process_reader(self, pid):
        self.proc_reads += 1
        self.before_get()
        return copy.deepcopy(self.process)

    def get(self, deadline):
        self.gets += 1
        self.health_deadline = deadline
        raw = json.dumps(self.health).encode()
        self.after_get()
        return dict(status=self.status, body=raw)

    def publish(self):
        return owner.publish_observed_grant(self.path, self.dest,
            expected_ready_sha256=self.sha, expected_nonce='nonce', pins=self.pins,
            evidence_path=self.evidence, clock=lambda:self.now,
            phase_reader=self.phase_reader, process_reader=self.process_reader,
            get_health=self.get)

    def refused(self):
        with self.assertRaises(ValueError): self.publish()
        self.assertFalse(self.dest.exists())

    def test_real_publication_fixed_margin_and_lease_and_raw_evidence(self):
        with patch.object(producer, 'publish_grant', wraps=producer.publish_grant) as real:
            grant = self.publish()
        self.assertEqual(real.call_args.kwargs['health_margin'], 30.)
        self.assertEqual(self.health_deadline, 85.)
        self.assertEqual(grant['valid_until_monotonic'], 310.)
        self.assertEqual(grant['identity'], self.pins['identity'])
        self.assertTrue(grant['no_embedding_calls_authorized'])
        self.assertEqual(protocol.decode(self.dest.read_bytes()), grant)
        evidence = protocol.decode(self.evidence.read_bytes())
        self.assertEqual(protocol.decode(bytes.fromhex(evidence['health_raw_hex'])), self.health)
        self.assertGreaterEqual(self.proc_reads, 2)

    def test_http_total_budget_late_but_still_in_admission_is_refused(self):
        self.after_get = lambda:setattr(self, 'now', 85.001)
        self.refused()

    def test_delayed_process_snapshot_cannot_start_health_after_ready_margin(self):
        self.before_get = lambda:setattr(self, 'now', 140.)
        self.refused()
        self.assertEqual(self.gets, 0)

    def test_thirty_second_entry_margin_is_required_not_default_five(self):
        self.now = 131.
        self.refused()
        self.assertEqual(self.gets, 0)

    def test_wrong_model_is_refused_with_raw_health_preserved(self):
        self.health['modele'] = 'other'
        self.refused()
        self.assertIn('health_raw_hex', protocol.decode(self.evidence.read_bytes()))

    def test_wrong_dimension_or_bool_integer_or_missing_identity_is_refused(self):
        for field, value in [('dim', 384), ('factice', 0), ('dual', True),
                             ('max_chars_per_text', True), ('executable', 'foreign')]:
            with self.subTest(field=field):
                self.health = dict(self.pins['identity'], usage=dict(baux=0, en_cours=0))
                self.health[field] = value
                self.evidence = self.root/('refuse-'+field+'.json')
                self.refused()

    def test_usage_active_or_unknown_is_not_a_grant(self):
        for usage in [dict(baux=1,en_cours=0),dict(baux=0,en_cours=1),{},dict(baux=False,en_cours=0)]:
            with self.subTest(usage=usage):
                self.health['usage'] = usage
                self.evidence = self.root/('usage-'+str(len(list(self.root.iterdir())))+'.json')
                self.refused()

    def test_uid_start_and_executable_change_after_health_are_refused(self):
        for key,value in [('uid',os.getuid()+1),('start','88'),('exe_sha256','c'*64),('exe','/other')]:
            with self.subTest(field=key):
                self.process = dict(self.pins['expected_process'], state='S')
                self.after_get = lambda k=key,v=value:self.process.update({k:v})
                self.evidence = self.root/('process-'+key+'.json')
                self.dest = self.root/('process-'+key+'-grant.json')
                self.refused()

    def test_wrong_process_before_health_does_not_contact_provider(self):
        self.process['start'] = 'reused'
        self.refused()
        self.assertEqual(self.gets, 0)

    def test_terminated_authority_or_first_red_refuses_without_editing_red(self):
        for mode in ['terminated', 'red']:
            with self.subTest(mode=mode):
                self.phase['authority']['etat']='actif';self.phase['first_red_exists']=False
                red=self.root/'first-red.json'
                def stop():
                    if mode=='red':
                        red.write_bytes(b'first immutable refusal')
                        self.phase['first_red_exists']=True
                    else:self.phase['authority']['etat']='termine'
                self.after_get=stop;self.evidence=self.root/(mode+'.json')
                self.refused()
                if mode=='red':self.assertEqual(red.read_bytes(),b'first immutable refusal')

    def test_wrong_admission_reference_and_protocol_pin_prevent_contact(self):
        self.pins['allocation']='other';self.pins['c7_reference']='other'
        self.refused();self.assertEqual(self.gets,0)

    def test_source_pin_rejected_before_health(self):
        self.pins['producer_sha256']='0'*64
        self.refused();self.assertEqual(self.gets,0)

    def test_non_200_status_is_refused(self):
        self.status=503;self.refused()

    def test_evidence_is_persisted_before_link_not_after(self):
        original=os.link
        def check(src,dst):
            self.assertTrue(self.evidence.exists())
            return original(src,dst)
        with patch.object(producer.os,'link',side_effect=check):self.publish()


class Transport(unittest.TestCase):
    def context(self):
        fixture = Observation('test_non_200_status_is_refused')
        fixture.setUp()
        self.addCleanup(fixture.doCleanups)
        return fixture, dict(ready_path=str(fixture.path), ready_sha256=fixture.sha,
                             nonce='nonce', pins=fixture.pins)

    def test_worker_only_fixed_health_endpoint_no_redirect_or_embedding(self):
        from unittest.mock import Mock
        fixture, context = self.context()
        connection = Mock()
        connection.getresponse.return_value.status = 200
        connection.getresponse.return_value.read.return_value = json.dumps(fixture.health).encode()
        with patch.object(owner.http.client, 'HTTPConnection', return_value=connection) as http, \
             patch.object(protocol, 'read_phase_state', side_effect=fixture.phase_reader), \
             patch.object(owner.time, 'monotonic', return_value=80.), patch('builtins.print') as printed:
            owner._health_worker(85., context)
        http.assert_called_once_with('127.0.0.1',7878,timeout=5.)
        connection.request.assert_called_once_with('GET','/sante')
        connection.close.assert_called_once()
        observed = protocol.decode(printed.call_args.args[0])
        self.assertEqual(protocol.decode(bytes.fromhex(observed['raw_hex'])), fixture.health)

    def test_worker_checks_real_raccord_phase_before_http(self):
        fixture, context = self.context()
        fixture.phase['first_red_exists'] = True
        with patch.object(owner.http.client,'HTTPConnection') as http, \
             patch.object(protocol,'read_phase_state',side_effect=fixture.phase_reader), \
             patch.object(owner.time,'monotonic',return_value=80.):
            with self.assertRaises(ValueError): owner._health_worker(85.,context)
        http.assert_not_called()

    def test_parent_timeout_kills_and_reaps_worker_not_a_thread_left_running(self):
        from unittest.mock import Mock
        fixture, context = self.context()
        child = Mock()
        child.communicate.side_effect = [owner.subprocess.TimeoutExpired('fixture',5), (b'',b'')]
        with patch.object(owner.subprocess,'Popen',return_value=child), \
             patch.object(owner.time,'monotonic',return_value=80.):
            with self.assertRaises(owner.subprocess.TimeoutExpired):
                owner.health_get(85.,ready_path=fixture.path,ready_sha256=fixture.sha,
                                 nonce='nonce',pins=fixture.pins)
        self.assertEqual(child.communicate.call_args_list[0].kwargs['timeout'],5.)
        child.kill.assert_called_once()
        self.assertEqual(child.communicate.call_count,2)

    def test_nonzero_worker_retains_stdout_stderr_and_exit_before_refusal(self):
        from unittest.mock import Mock
        fixture, context = self.context()
        child = Mock(pid=987, returncode=3)
        child.communicate.return_value=(b'partial health raw', b'worker failure')
        trace={}
        with patch.object(owner.subprocess,'Popen',return_value=child), \
             patch.object(owner.time,'monotonic',return_value=80.):
            with self.assertRaises(ValueError):
                owner.health_get(85.,ready_path=fixture.path,ready_sha256=fixture.sha,
                                 nonce='nonce',pins=fixture.pins,trace=trace)
        self.assertEqual(bytes.fromhex(trace['stdout_hex']),b'partial health raw')
        self.assertEqual(bytes.fromhex(trace['stderr_hex']),b'worker failure')
        self.assertEqual(trace['returncode'],3)
        self.assertFalse(trace['timed_out'])

    def test_timeout_preserves_partial_and_reaped_output(self):
        from unittest.mock import Mock
        fixture, context = self.context()
        error=owner.subprocess.TimeoutExpired('fixture',5,output=b'partial',stderr=b'early error')
        child=Mock(pid=987,returncode=-9)
        child.communicate.side_effect=[error,(b'partial plus tail',b'early error plus tail')]
        trace={}
        with patch.object(owner.subprocess,'Popen',return_value=child), \
             patch.object(owner.time,'monotonic',return_value=80.):
            with self.assertRaises(owner.subprocess.TimeoutExpired):
                owner.health_get(85.,ready_path=fixture.path,ready_sha256=fixture.sha,
                                 nonce='nonce',pins=fixture.pins,trace=trace)
        self.assertEqual(bytes.fromhex(trace['stdout_hex']),b'partial plus tail')
        self.assertEqual(bytes.fromhex(trace['timeout_stdout_hex']),b'partial')
        self.assertEqual(bytes.fromhex(trace['stderr_hex']),b'early error plus tail')
        self.assertEqual(trace['returncode'],-9)
        self.assertTrue(trace['timed_out'])
        child.kill.assert_called_once()

    def test_oversize_worker_preserves_received_prefix_before_refusal(self):
        from unittest.mock import Mock
        fixture, context = self.context()
        connection=Mock();connection.getresponse.return_value.status=200
        connection.getresponse.return_value.read.return_value=b'x'*65537
        with patch.object(owner.http.client,'HTTPConnection',return_value=connection), \
             patch.object(protocol,'read_phase_state',side_effect=fixture.phase_reader), \
             patch.object(owner.time,'monotonic',return_value=80.),patch('builtins.print') as printed:
            with self.assertRaises(ValueError):owner._health_worker(85.,context)
        evidence=protocol.decode(printed.call_args.args[0])
        self.assertEqual(bytes.fromhex(evidence['raw_hex']),b'x'*65537)
        self.assertTrue(evidence['oversize'])
        self.assertTrue(evidence['prefix_only'])

    def test_worker_expired_phase_never_opens_connection(self):
        fixture, context = self.context()
        with patch.object(owner.http.client,'HTTPConnection') as http, \
             patch.object(protocol,'read_phase_state',side_effect=fixture.phase_reader), \
             patch.object(owner.time,'monotonic',return_value=160.):
            with self.assertRaises(ValueError): owner._health_worker(85.,context)
        http.assert_not_called()


if __name__=='__main__':unittest.main()
