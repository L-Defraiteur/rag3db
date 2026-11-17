"""Grouped real-core guard witnesses BEFORE fixes. Ten cases, NOT RUN.

The three original cases are loaded unchanged. Seven additional cases use the
real entries/engine, with modeled clocks, identities, bytes and Popen only.
No successful model is physical evidence; unexpected exceptions remain ERROR.
"""
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import sys
import unittest
from unittest.mock import patch

HERE = Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location('unchanged_guard_cases', HERE / 'test_publication_guard_counterexamples.py')
original = importlib.util.module_from_spec(spec)
spec.loader.exec_module(original)
fixtures, engine = original.fixtures, original.engine
ROOT = HERE.parents[3]
PILOT = ROOT / 'docs/codeparsers/16-11-2026/rust-probe/pilot.py'
LEGACY = ROOT / 'docs/codeparsers/16-11-2026/rust-red/pilot.py'
PHYSICAL = PILOT.with_name('physical_guard.py')


class FakeChild:
    """Boundary Popen data: time is consumed, never an OS process or signal."""
    def __init__(self, clock):
        self.clock = clock
        self.returncode = None
        self.killed = False
    def poll(self):
        return self.returncode
    def communicate(self, payload, timeout):
        self.clock.now += timeout
        raise subprocess.TimeoutExpired('fixture-ssh', timeout)
    def terminate(self):
        pass
    def kill(self):
        self.killed = True
    def wait(self, timeout):
        self.clock.now += timeout
        if not self.killed:
            raise subprocess.TimeoutExpired('fixture-reap', timeout)
        self.returncode = -9
        return self.returncode


class AdditionalPublicationGuards(unittest.TestCase):
    def gate_fixture(self):
        case = original.PublicationGuardCounterexamples('test_common_request_respects_phase_watchdog_as_well_as_whole')
        self.addCleanup(case.doCleanups)
        base, pair, pin = case.gate_fixture()
        return base, pair, pin

    def set_clock_envelope(self, base, pair, pin, offset):
        # Proposed origin descriptor: independent modeled MONOTONIC facts, not
        # inferred from proc start. Its operational producer/trust needs review.
        bootstrap_path = base.root / 'relay-bootstrap.json'
        bootstrap = json.loads(bootstrap_path.read_text())
        hard = bootstrap['hard_timeout']
        hard['start'] = str((9000 + offset) * os.sysconf('SC_CLK_TCK'))
        base.processes[hard['pid']]['start'] = hard['start']
        bootstrap['queue_end'] = 9300
        bootstrap['whole_window'] = dict(clock='CLOCK_MONOTONIC', host='luciepc',
            started=9000, soft_end=11098, kill_end=11100, identity=dict(hard))
        bootstrap_path.write_text(json.dumps(bootstrap))
        outer = base.processes[9]
        outer['exe'] = '/timeout'
        outer['start'] = str((9000 + offset) * os.sysconf('SC_CLK_TCK'))
        (base.root / 'controls.json').write_text(json.dumps([outer]))
        (base.root / 'outer-supervision.json').write_text(json.dumps({
            'clock': 'CLOCK_MONOTONIC', 'started': 9000, 'soft_end': 10798,
            'kill_end': 10800, 'timeout_identity': outer}))

    def test_proc_identity_offset_cannot_inflate_whole_remaining(self):
        base, pair, pin = self.gate_fixture()
        self.set_clock_envelope(base, pair, pin, 5000)
        with patch.object(engine.time, 'monotonic', return_value=10700):
            requested = engine.common_request_budget(pair, pin, 'baseline', base.gate.guard)
        # Whole END11098 minus current10700 minus margin30, independent of
        # proc start14000 (BOOTTIME model). Phase may only tighten this limit.
        self.assertLessEqual(requested, 368)

    def test_proc_identity_offset_is_not_a_queue_timestamp(self):
        base, pair, pin = self.gate_fixture()
        self.set_clock_envelope(base, pair, pin, 5000)
        state = base.gate.starting()
        self.assertGreater(state['phase_remaining'], 0)
        self.assertLessEqual(state['phase_remaining'], 1798)

    def reject_before_import(self, entry, changed):
        base, pair, pin = self.gate_fixture()
        sources = {'relay.py': Path(fixtures.module.__file__),
            'gate.py': Path(base.gate_module.__file__), 'pilot.py': PILOT,
            'legacy.py': LEGACY, 'physical_guard.py': PHYSICAL}
        for name, path in sources.items():
            (base.control / name).write_bytes(path.read_bytes())
            pair['controllers'][name] = fixtures.module.digest(base.control / name)
        prepared = Path(pair['publication_layout']['prepared'])
        prepared.write_text(json.dumps(pair))
        sentinel = 'UNPINNED_ENTRY_DEPENDENCY_EXECUTED'
        (base.control / changed).write_text("raise RuntimeError('" + sentinel + "')\n")
        module_spec = importlib.util.spec_from_file_location('guarded_entry_probe', base.control / entry)
        module = importlib.util.module_from_spec(module_spec)
        cached = {name: sys.modules.pop(name, None) for name in ('relay', 'publication_relay')}
        argv = [str(base.control / entry), '--proofs', pair['proofs'], '--prepared', str(prepared),
                '--phase', 'baseline', '--allocation', 'fixture-only-not-a-runtime-grant']
        try:
            with patch.object(sys, 'path', [str(base.control), *sys.path]), \
                 patch.object(sys, 'argv', argv), \
                 patch.dict(os.environ, {'RUST_PROBE_TOOL_ROOT': str(ROOT)}):
                try:
                    with self.assertRaises(AssertionError):
                        module_spec.loader.exec_module(module)
                except RuntimeError as error:
                    if str(error) != sentinel:
                        raise
                    self.fail('UNPINNED_DEPENDENCY_EXECUTED_VIA_' + entry + ':' + changed)
        finally:
            for name, previous in cached.items():
                sys.modules.pop(name, None)
                if previous is not None:
                    sys.modules[name] = previous

    def test_gate_entry_rejects_unpinned_core_before_import(self):
        self.reject_before_import('gate.py', 'publication_relay.py')

    def test_pilot_entry_rejects_unpinned_legacy_before_import(self):
        self.reject_before_import('pilot.py', 'legacy.py')

    def test_pilot_entry_rejects_unpinned_physical_guard_before_import(self):
        self.reject_before_import('pilot.py', 'physical_guard.py')

    def transport_fixture(self):
        base, pair, pin = self.gate_fixture()
        clock = fixtures.Clock()
        clock.now = 9000
        child = FakeChild(clock)
        transport = engine.SSHTransport.__new__(engine.SSHTransport)
        transport.pair, transport.pin, transport.phase = pair, pin, 'baseline'
        transport.control, transport.entry_file = base.control, base.control / 'relay.py'
        transport.root = base.root / 'relay-local'
        transport.root.mkdir()
        transport.guard, transport.host = base.gate.guard, 'luciepc'
        transport.counter, transport.active_call = 0, None
        transport.child, transport.handles = child, []
        transport.ready_record = {'identity': dict(base.processes[11])}
        transport.remote_cancelled = False
        transport.plan = base.gate.plan
        return base, transport, clock, child

    def test_rpc_reap_graces_are_inside_original_call_budget(self):
        base, transport, clock, child = self.transport_fixture()
        before = clock.now
        with patch.object(engine, '__file__', str(base.control / 'publication_relay.py')), \
             patch.object(engine.os, 'getpid', return_value=11), \
             patch.object(engine.time, 'monotonic', side_effect=clock), \
             patch.object(engine.subprocess, 'Popen', return_value=child):
            with self.assertRaises((subprocess.TimeoutExpired, TimeoutError)) as caught:
                transport.call('snapshot', .4)
        if isinstance(caught.exception, subprocess.TimeoutExpired):
            self.assertEqual(caught.exception.cmd, 'fixture-ssh', 'cleanup replaced the first RPC cause')
        self.assertLessEqual(clock.now - before, .4 + 1e-9)

    def test_stop_requires_and_preserves_original_local_deadline(self):
        base, transport, clock, child = self.transport_fixture()
        end = clock.now + .4
        with patch.object(engine.time, 'monotonic', side_effect=clock):
            try:
                try:
                    transport.stop(deadline=end)
                except TimeoutError:
                    pass  # Refusal/UNKNOWN allowed; extra unbounded time is not.
            except TypeError as error:
                if "unexpected keyword argument 'deadline'" not in str(error):
                    raise
                self.fail('STOP_ORIGINAL_DEADLINE_UNSUPPORTED_BY_REAL_TRANSPORT')
        self.assertLessEqual(clock.now, end + 1e-9)


def load_tests(loader, tests, pattern):
    # Exactly original3 + additional7. Do not discover the 36/3/8 fixtures.
    suite = unittest.TestSuite()
    suite.addTests(loader.loadTestsFromTestCase(original.PublicationGuardCounterexamples))
    suite.addTests(loader.loadTestsFromTestCase(AdditionalPublicationGuards))
    return suite


if __name__ == '__main__':
    unittest.main()
