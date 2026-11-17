"""Three prepublished guard witnesses, before fixes. NOT RUN.

Real publication core and wrapper; synthetic bytes/process records/clock only.
Unexpected import/fixture exceptions remain ERROR, never an invented red.
"""
import importlib.util
import json
from pathlib import Path
import sys
import unittest
from unittest.mock import patch

BASE = Path(__file__).resolve().parent.parent / 'rust-probe-relay/test_relay.py'
sys.path.insert(0, str(BASE.parent))
spec = importlib.util.spec_from_file_location('guard_baseline_fixtures', BASE)
fixtures = importlib.util.module_from_spec(spec)
spec.loader.exec_module(fixtures)
engine = fixtures.implementation


class PublicationGuardCounterexamples(unittest.TestCase):
    def gate_fixture(self):
        base = fixtures.GateTests('test_real_active_gate_returns_pc_remaining_duration')
        base.setUp()
        self.addCleanup(base.doCleanups)
        pair = base.gate.pair
        core = base.control / 'publication_relay.py'
        core.write_bytes(Path(engine.__file__).read_bytes())
        (base.control / 'publication_loader.py').write_bytes(
            Path(engine.__file__).with_name('publication_loader.py').read_bytes())
        for name in set(pair['publication_layout']['entries'].values()) | {'publication_relay.py', 'publication_loader.py'}:
            path = base.control / name
            if not path.exists():
                path.write_bytes(b'synthetic controller bytes, not imported')
            pair['controllers'][name] = fixtures.module.digest(path)
        prepared = Path(pair['publication_layout']['prepared'])
        prepared.write_text(json.dumps(pair))
        pin = fixtures.module.digest(prepared)
        base.gate.pin = pin
        bootstrap_path = base.root / 'relay-bootstrap.json'
        bootstrap = json.loads(bootstrap_path.read_text())
        bootstrap['prepared_sha256'] = pin
        bootstrap['whole_window']['prepared_sha256'] = pin
        base.write_bootstrap(bootstrap)
        phase_window = json.loads((base.root / 'outer-supervision.json').read_text())['window']
        phase_window['prepared_sha256'] = pin
        base.write_phase_window(phase_window)
        return base, pair, pin

    def test_common_request_respects_phase_watchdog_as_well_as_whole(self):
        base, pair, pin = self.gate_fixture()
        # Phase starts at9000; Common requested sixty seconds later. The whole
        # watchdog has more room, but the phase watchdog still ends at10798.
        with patch.object(engine.time, 'monotonic', return_value=9060):
            requested = engine.common_request_budget(pair, pin, 'baseline', base.gate.guard)
        phase_end = 9000 + pair['publication_limits']['phase_soft']
        self.assertLessEqual(requested, phase_end - 9060 - pair['publication_limits']['common_margin'])

    def test_local_hard_end_survives_finish_and_close_before_final_result(self):
        base = fixtures.RelayTests('test_ready_precedes_start_and_two_publications_are_ordered')
        base.setUp()
        self.addCleanup(base.doCleanups)
        base.relay.hard_end = base.clock.now + 10
        original = base.transport.stop
        def late_close(*, deadline=None):
            original(deadline=deadline)
            base.clock.now += 11
        base.transport.stop = late_close
        with self.assertRaises(TimeoutError):
            base.relay.run()
        result = Path(base.tmp.name) / 'relay-result.json'
        if result.exists():
            self.assertNotEqual(json.loads(result.read_text())['verdict'], 'VERT')

    def test_unpinned_core_is_rejected_before_wrapper_executes_it(self):
        base, pair, pin = self.gate_fixture()
        wrapper = base.control / 'relay.py'
        wrapper.write_bytes(Path(fixtures.module.__file__).read_bytes())
        pair['controllers']['relay.py'] = fixtures.module.digest(wrapper)
        prepared = Path(pair['publication_layout']['prepared'])
        prepared.write_text(json.dumps(pair))
        # Preserve the approved core SHA in prepared, replace ONLY its bytes.
        (base.control / 'publication_relay.py').write_text(
            "raise RuntimeError('UNPINNED_PUBLICATION_CORE_EXECUTED')\n")
        cached = sys.modules.pop('publication_relay', None)
        loader = importlib.util.spec_from_file_location('unpinned_wrapper_probe', wrapper)
        module = importlib.util.module_from_spec(loader)
        argv = [str(wrapper), '--proofs', pair['proofs'], '--prepared', str(prepared),
                '--phase', 'baseline', '--allocation', 'fixture-only-not-a-runtime-grant',
                '--pin', fixtures.module.digest(prepared)]
        try:
            with patch.object(sys, 'argv', argv):
                try:
                    with self.assertRaises(AssertionError):
                        loader.loader.exec_module(module)
                except RuntimeError as error:
                    if str(error) != 'UNPINNED_PUBLICATION_CORE_EXECUTED':
                        raise
                    self.fail('WRAPPER_EXECUTED_UNPINNED_CORE_BEFORE_VALIDATION')
        finally:
            sys.modules.pop('publication_relay', None)
            if cached is not None:
                sys.modules['publication_relay'] = cached


if __name__ == '__main__':
    unittest.main()
