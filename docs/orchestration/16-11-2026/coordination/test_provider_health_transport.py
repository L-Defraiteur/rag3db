"""Pure fakes at real worker/caller boundaries; no network/process/proc action."""
from pathlib import Path
import copy
import hashlib
import json
import os
import subprocess
import tempfile
import unittest
from unittest.mock import Mock, patch
import provider_health_transport as t


class HealthTransport(unittest.TestCase):
    def setUp(self):
        self.tmp=tempfile.TemporaryDirectory();self.addCleanup(self.tmp.cleanup)
        self.root=Path(self.tmp.name);self.gate=self.root/'policy.py'
        self.gate.write_text("def worker_gate(context):\n if context.get('refuse'): raise ValueError('phase red')\n")
        self.context=dict(address='127.0.0.1:7879',admission=True,contact_deadline=85.,
            gate_module_path=str(self.gate),gate_module_sha256=hashlib.sha256(self.gate.read_bytes()).hexdigest(),
            expected_process=dict(pid=123,uid=os.getuid(),start='42',exe='/fixture/provider',exe_sha256='a'*64))
        self.identity=dict(service='rag3weaver-embeddings',modele='bge-m3',dim=1024,factice=False,
            dual=True,sparse=True,executable='/fixture/provider@suffix',precision='Flex32',max_chars_per_text=12000)
        self.health=dict(self.identity,usage=dict(baux=0,en_cours=0))

    def child(self,returncode=0):
        child=Mock(pid=999,returncode=returncode)
        child.communicate.return_value=(json.dumps(dict(status=200,raw_hex=json.dumps(self.health).encode().hex(),oversize=False,prefix_only=False)).encode(),b'')
        return child

    def run_fake(self,child,trace):
        with patch.object(t.time,'monotonic',return_value=80.),patch.object(t.subprocess,'Popen',return_value=child):
            return t.run_health_worker(self.context,trace)

    def test_success_fixed_child_and_cleanup_reserve_not_extra_ttl(self):
        child=self.child();trace={};response=self.run_fake(child,trace)
        self.assertEqual(t.validate_health(response['body'],self.identity,status=response['status']),self.health)
        self.assertEqual(child.communicate.call_args.kwargs['timeout'],4.5)
        self.assertEqual(trace['work_deadline_monotonic'],84.5)
        self.assertTrue(trace['client_reaped']);self.assertTrue(trace['cleanup_verified'])

    def test_worker_calls_only_pinned_policy_then_fixed_bge_health(self):
        context=dict(self.context,work_deadline_monotonic=84.5)
        connection=Mock();connection.getresponse.return_value.status=200
        connection.getresponse.return_value.read.return_value=json.dumps(self.health).encode()
        with patch.object(t.time,'monotonic',return_value=80.),patch.object(t.http.client,'HTTPConnection',return_value=connection) as http,patch('builtins.print'):
            t.worker(context)
        http.assert_called_once_with('127.0.0.1',7879,timeout=4.5)
        connection.request.assert_called_once_with('GET','/sante');connection.close.assert_called_once()

    def test_refused_policy_has_no_network_even_if_lease_action_was_requested(self):
        context=dict(self.context,admission=False,refuse=True,work_deadline_monotonic=84.5)
        with patch.object(t.time,'monotonic',return_value=80.),patch.object(t.http.client,'HTTPConnection') as http:
            with self.assertRaises(ValueError):t.worker(context)
        http.assert_not_called()

    def test_context_source_pin_is_checked_before_worker_spawn(self):
        self.context['gate_module_sha256']='b'*64
        with patch.object(t.time,'monotonic',return_value=80.),patch.object(t.subprocess,'Popen') as spawn:
            with self.assertRaises(ValueError):t.run_health_worker(self.context,{})
        spawn.assert_not_called()

    def test_invalid_deadline_mode_or_live_endpoint_never_spawns(self):
        for change in [dict(address='127.0.0.1:7970'),dict(admission=1),dict(contact_deadline=86.),
                       dict(contact_deadline=float('nan')),dict(contact_deadline=True),dict(contact_deadline=80.)]:
            with self.subTest(change=change),patch.object(t.time,'monotonic',return_value=80.),patch.object(t.subprocess,'Popen') as spawn:
                with self.assertRaises(ValueError):t.run_health_worker(dict(self.context,**change),{})
                spawn.assert_not_called()

    def test_timeout_has_bounded_cleanup_and_preserved_partial_raw(self):
        child=self.child(-9);first=subprocess.TimeoutExpired('fake',4.5,output=b'part',stderr=b'early')
        child.communicate.side_effect=[first,(b'part plus tail',b'early plus tail')];trace={}
        with self.assertRaises(subprocess.TimeoutExpired) as caught:self.run_fake(child,trace)
        self.assertIs(caught.exception,first);child.kill.assert_called_once()
        self.assertIn('timeout',child.communicate.call_args.kwargs)
        self.assertLessEqual(child.communicate.call_args.kwargs['timeout'],5.)
        self.assertEqual(bytes.fromhex(trace['stdout_hex']),b'part plus tail')
        self.assertEqual(bytes.fromhex(trace['timeout_stdout_hex']),b'part')

    def test_cleanup_failure_preserves_first_exception_and_marks_unknown(self):
        child=self.child(None);first=subprocess.TimeoutExpired('fake',4.5,output=b'partial')
        child.communicate.side_effect=[first,subprocess.TimeoutExpired('cleanup',.5,output=b'partial still')]
        trace={}
        with self.assertRaises(subprocess.TimeoutExpired) as caught:self.run_fake(child,trace)
        self.assertIs(caught.exception,first)
        self.assertFalse(trace['client_reaped']);self.assertFalse(trace['cleanup_verified'])
        self.assertEqual(trace['worker_pid'],999)

    def test_kill_failure_does_not_replace_original_timeout(self):
        child=self.child(-9);first=subprocess.TimeoutExpired('fake',4.5)
        child.communicate.side_effect=[first,(b'',b'')];child.kill.side_effect=ProcessLookupError('gone')
        with self.assertRaises(subprocess.TimeoutExpired) as caught:self.run_fake(child,{})
        self.assertIs(caught.exception,first)

    def test_nonzero_worker_preserves_raw_before_refusal(self):
        child=self.child(1);child.communicate.return_value=(b'raw worker prefix',b'HTTP failure');trace={}
        with self.assertRaises(ValueError):self.run_fake(child,trace)
        self.assertEqual(bytes.fromhex(trace['stdout_hex']),b'raw worker prefix')
        self.assertEqual(bytes.fromhex(trace['stderr_hex']),b'HTTP failure');self.assertEqual(trace['returncode'],1)

    def test_oversize_worker_preserves_bounded_prefix_before_refusal(self):
        context=dict(self.context,work_deadline_monotonic=84.5)
        connection=Mock();connection.getresponse.return_value.status=200
        connection.getresponse.return_value.read.return_value=b'x'*65537
        with patch.object(t.time,'monotonic',return_value=80.),patch.object(t.http.client,'HTTPConnection',return_value=connection),patch('builtins.print') as printed:
            with self.assertRaises(ValueError):t.worker(context)
        record=t.decode(printed.call_args.args[0]);self.assertTrue(record['prefix_only'])
        self.assertEqual(len(bytes.fromhex(record['raw_hex'])),65537)

    def test_http_failure_duplicate_json_and_nonfinite_are_refused(self):
        for status,raw in [(503,b'{}'),(True,b'{}'),(200,b'{"x":1,"x":2}'),(200,b'{"x":NaN}')]:
            with self.subTest(status=status,raw=raw),self.assertRaises(ValueError):t.validate_health(raw,self.identity,status=status)

    def test_false_is_not_zero_and_all_nine_identity_types_are_strict(self):
        for field,value in [('dim',1024.),('dual',1),('max_chars_per_text',True),('modele','other')]:
            with self.subTest(field=field),self.assertRaises(ValueError):
                t.validate_health(json.dumps(dict(self.health,**{field:value})).encode(),self.identity,status=200)
        for usage in [dict(baux=False,en_cours=0),dict(baux=0,en_cours=False),dict(baux=1,en_cours=0),{}]:
            with self.subTest(usage=usage),self.assertRaises(ValueError):
                t.validate_health(json.dumps(dict(self.health,usage=usage)).encode(),self.identity,status=200)


if __name__=='__main__':unittest.main()
