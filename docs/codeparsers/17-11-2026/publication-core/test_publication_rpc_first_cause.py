"""Prepublished causal witness, before fix. NOT RUN; no OS child or SSH.

Original ten witnesses remain unchanged. Real SSHTransport.call receives a
modeled Popen whose communicate and reap fail for distinguishable reasons.
"""
import importlib.util
import json
from pathlib import Path
import subprocess
import unittest
from unittest.mock import patch

spec = importlib.util.spec_from_file_location('causal_group_fixtures', Path(__file__).with_name('test_publication_guards_grouped.py'))
group = importlib.util.module_from_spec(spec)
spec.loader.exec_module(group)
engine = group.engine


class CleanupLostChild(group.FakeChild):
    def communicate(self, payload, timeout):
        self.clock.now += timeout
        self.original_error = subprocess.TimeoutExpired('fixture-ssh', timeout)
        raise self.original_error
    def wait(self, timeout):
        self.clock.now += timeout
        raise subprocess.TimeoutExpired('fixture-reap', timeout)


class RPCFirstCause(unittest.TestCase):
    def test_cleanup_failure_preserves_authentic_first_rpc_error_and_unknown(self):
        case = group.AdditionalPublicationGuards('test_rpc_reap_graces_are_inside_original_call_budget')
        self.addCleanup(case.doCleanups)
        base, transport, clock, child = case.transport_fixture()
        lost = CleanupLostChild(clock)
        transport.child = lost
        with patch.object(engine, '__file__', str(base.control / 'publication_relay.py')), \
             patch.object(engine.os, 'getpid', return_value=11), \
             patch.object(engine.time, 'monotonic', side_effect=clock), \
             patch.object(engine.subprocess, 'Popen', return_value=lost):
            with self.assertRaises(subprocess.TimeoutExpired) as caught:
                transport.call('snapshot', 3)
        # A generic TimeoutError or a new cleanup TimeoutExpired is not evidence
        # of the original communicate failure. Preserve this exact exception.
        self.assertIs(caught.exception, lost.original_error)
        self.assertEqual(caught.exception.cmd, 'fixture-ssh')
        records = list(transport.root.glob('*.result.json'))
        self.assertTrue(records, 'missing cleanup evidence after interrupted RPC')
        evidence = json.loads(records[-1].read_text())
        self.assertEqual(evidence['cleanup']['remote_quiet'], 'UNKNOWN')
        self.assertIn('fixture-reap', evidence['cleanup']['error'])
        self.assertIn('fixture-ssh', evidence['original_error'])


if __name__ == '__main__':
    unittest.main()
