"""Five causal witnesses before residual fixes. NOT RUN, engine/loader e15.

Actual loader/Relay/SSHTransport, with file read/write and process boundaries
modeled only. No native child, signal, SSH, phase or replacement kernel.
"""
import importlib.util
import json
from pathlib import Path
import sys
from types import SimpleNamespace
import unittest
from unittest.mock import Mock, patch

HERE = Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location('residual_group_fixtures', HERE / 'test_publication_guards_grouped.py')
group = importlib.util.module_from_spec(spec)
spec.loader.exec_module(group)
fixtures, engine = group.fixtures, group.engine


class ResidualCounterexamples(unittest.TestCase):
    def relay_fixture(self):
        case = fixtures.RelayTests('test_ready_precedes_start_and_two_publications_are_ordered')
        case.setUp()
        self.addCleanup(case.doCleanups)
        first = RuntimeError('FIRST_RPC_FIXTURE_ERROR')
        case.transport.snapshot = Mock(side_effect=first)
        return case, first

    def assert_failure_record(self, owner, first, *secondary_messages):
        # In-memory fallback contract, not a claim that a disk write succeeded.
        record = owner.failure_record
        self.assertEqual(record['first_error'], repr(first))
        self.assertEqual(record['remote_quiet'], 'UNKNOWN')
        serialized = json.dumps(record['secondary_errors'])
        for message in secondary_messages:
            self.assertIn(message, serialized)

    def failing_log(self, name):
        original = engine.atomic_json
        def write(path, value, *args, **kwargs):
            if Path(path).name == name:
                raise OSError('LOG_FIXTURE_FAILED:' + name)
            return original(path, value, *args, **kwargs)
        return write

    def test_first_red_write_failure_preserves_first_error_and_still_aborts(self):
        case, first = self.relay_fixture()
        with patch.object(engine, 'atomic_json', side_effect=self.failing_log('first-red.json')):
            with self.assertRaises((RuntimeError, OSError)) as caught:
                case.relay.run()
        self.assertIs(caught.exception, first)
        self.assertIn('abort', case.transport.calls)
        self.assertTrue(case.transport.closed)
        self.assert_failure_record(case.relay, first, 'LOG_FIXTURE_FAILED:first-red.json')

    def test_abort_refusal_log_failure_cannot_replace_first_error(self):
        case, first = self.relay_fixture()
        case.transport.abort = Mock(side_effect=OSError('ABORT_FIXTURE_FAILED'))
        with patch.object(engine, 'atomic_json', side_effect=self.failing_log('cleanup-refused.json')):
            with self.assertRaises((RuntimeError, OSError)) as caught:
                case.relay.run()
        self.assertIs(caught.exception, first)
        case.transport.abort.assert_called_once()
        self.assertTrue(case.transport.closed)
        self.assert_failure_record(case.relay, first,
            'ABORT_FIXTURE_FAILED', 'LOG_FIXTURE_FAILED:cleanup-refused.json')

    def test_local_stop_refusal_log_failure_cannot_replace_first_error(self):
        case, first = self.relay_fixture()
        case.transport.stop = Mock(side_effect=OSError('STOP_FIXTURE_FAILED'))
        with patch.object(engine, 'atomic_json', side_effect=self.failing_log('local-stop-refused.json')):
            with self.assertRaises((RuntimeError, OSError)) as caught:
                case.relay.run()
        self.assertIs(caught.exception, first)
        self.assertIn('abort', case.transport.calls)
        case.transport.stop.assert_called_once()
        self.assert_failure_record(case.relay, first,
            'STOP_FIXTURE_FAILED', 'LOG_FIXTURE_FAILED:local-stop-refused.json')

    def test_stop_log_close_preserves_prior_reap_failure_and_unknown(self):
        case = group.AdditionalPublicationGuards('test_rpc_reap_graces_are_inside_original_call_budget')
        self.addCleanup(case.doCleanups)
        base, transport, clock, child = case.transport_fixture()
        first = OSError('REAP_FIXTURE_FIRST_ERROR')
        transport.child = SimpleNamespace(pid=101, returncode=None,
            poll=Mock(return_value=None), terminate=Mock(), kill=Mock(),
            wait=Mock(side_effect=first))
        transport.handles = [SimpleNamespace(close=Mock(side_effect=OSError('CLOSE_FIXTURE_SECOND_ERROR')))]
        with patch.object(engine.time, 'monotonic', side_effect=clock):
            with self.assertRaises(OSError) as caught:
                transport.stop(deadline=clock.now + 5)
        self.assertIs(caught.exception, first)
        self.assert_failure_record(transport, first, 'CLOSE_FIXTURE_SECOND_ERROR')

    def test_prepared_json_comes_from_exact_bytes_that_matched_the_pin(self):
        case = group.AdditionalPublicationGuards('test_rpc_reap_graces_are_inside_original_call_budget')
        self.addCleanup(case.doCleanups)
        base, pair, pin = case.gate_fixture()
        entry = base.control / 'relay.py'
        entry.write_bytes(Path(fixtures.module.__file__).read_bytes())
        pair['controllers']['relay.py'] = fixtures.module.digest(entry)
        prepared = Path(pair['publication_layout']['prepared'])
        prepared.write_text(json.dumps(pair))
        approved_bytes = prepared.read_bytes()
        pin = fixtures.module.digest(prepared)
        approved = json.loads(approved_bytes)
        unverified = json.loads(approved_bytes)
        unverified['publication_layout']['execution_host'] = 'HOST_FROM_UNVERIFIED_SECOND_READ'
        original_read_text = Path.read_text
        def second_read(path, *args, **kwargs):
            if path == prepared:
                return json.dumps(unverified)
            return original_read_text(path, *args, **kwargs)
        argv = [str(entry), '--proofs', pair['proofs'], '--prepared', str(prepared), '--pin', pin]
        loader = fixtures.module._bootstrap_module
        with patch.object(sys, 'argv', argv), patch.object(Path, 'read_text', second_read):
            context = loader.verify_entry(entry, source_entry=Path(fixtures.module.__file__))
        self.assertEqual(context[1], pin)
        self.assertEqual(context[0], approved)


if __name__ == '__main__':
    unittest.main()
