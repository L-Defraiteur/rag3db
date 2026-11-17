"""Fenêtres prépubliées avant leur implantation : vrai Gate, identités/horloge factices."""
import hashlib
import importlib.util
import json
import sys
from pathlib import Path
import unittest
from unittest.mock import patch

BASE = Path(__file__).resolve().parent.parent / 'rust-probe-relay/test_relay.py'
# Les wrappers du vrai Gate importent relay depuis ce dossier ; aucun module factice.
sys.path.insert(0, str(BASE.parent))
spec = importlib.util.spec_from_file_location('window_baseline_fixtures', BASE)
fixtures = importlib.util.module_from_spec(spec)
spec.loader.exec_module(fixtures)
engine = getattr(fixtures.module, 'engine', fixtures.module)


class WindowCases(unittest.TestCase):
    def setUp(self):
        self.base = fixtures.GateTests('test_real_active_gate_returns_pc_remaining_duration')
        self.base.setUp()
        self.addCleanup(self.base.doCleanups)
        self.gate = self.base.gate
        self.root, self.control = self.base.root, self.base.control
        self.path = self.root / 'global-window.json'
        self.ledger_sha = 'a' * 64
        self.gate.pin = '1' * 64
        self.gate.pair['execution_id'] = 'window-fixture'
        self.gate.pair['window_events'] = {'global': {'path': str(self.path),
            'owner_role': 'driver', 'event_kind': 'publication-window-opened'}}
        source = Path(engine.__file__)
        (self.control / 'publication_relay.py').write_bytes(source.read_bytes())
        self.gate.pair['controllers'].update({
            'pilot.py': fixtures.module.digest(self.control / 'pilot.py'),
            'publication_relay.py': fixtures.module.digest(self.control / 'publication_relay.py')})
        loader = self.control / 'publication_loader.py'
        loader.write_bytes(source.with_name('publication_loader.py').read_bytes())
        self.gate.pair['controllers']['publication_loader.py'] = fixtures.module.digest(loader)
        for name in self.gate.pair['controllers']:
            path = self.control / name
            if not path.exists():
                path.write_bytes(b'synthetic controller source, not executed')
            self.gate.pair['controllers'][name] = fixtures.module.digest(path)
        self.base.authority['script_sha256'] = self.gate.pair['controllers']['confiner_validation.py']
        self.base.receipt.write_text(json.dumps(self.base.authority))
        self.gate.pair['owner_roles'] = {'driver': {'descriptor': 'driver_identity',
            'entry_file': 'pilot.py', 'entry_sha256': fixtures.module.digest(self.control / 'pilot.py'),
            'component_hashes': dict(self.gate.pair['controllers'])}}

    def window_api(self):
        required = ['observe_active', 'record_window_start', 'observe_window']
        missing = [name for name in required if not callable(getattr(self.gate, name, None))]
        if missing:
            self.fail('WINDOW_LIFECYCLE_UNSUPPORTED_BY_REAL_CORE: ' + ','.join(missing))
        plan = engine.validate_plan({'protocol_version': 1, 'execution_id': 'window-fixture',
            'phase_id': 'baseline', 'stages': [{'label': 'global', 'ledger_file': 'ledger',
            'files': {'ledger': 'ledger'}, 'executables': {},
            'source_contract': {'kind': 'window-fixture'}, 'window_seconds': 90}]})
        self.gate.plan = plan
        return plan.stage('global'), self.gate.observe_active()

    def create(self, stage, phase):
        with patch.object(engine.os, 'getpid', return_value=11):
            return self.gate.record_window_start(stage, phase, self.ledger_sha)

    def test_window_uses_pc_remaining_clamped_to_operation_end(self):
        stage, phase = self.window_api()
        window = self.create(stage, phase)
        # now9000, ENDCommon9060, ENDop9045 : 45s, pas90s ni nouveau globalEND.
        self.assertEqual(window.remaining, 45)
        self.assertTrue(self.path.is_file())
        observed = self.gate.observe_window(stage, phase, self.ledger_sha)
        self.assertEqual(observed.event_sha256, hashlib.sha256(self.path.read_bytes()).hexdigest())
        authority = json.loads(self.base.receipt.read_text())
        self.assertEqual(authority['echeance'], 9060)

    def test_window_requires_real_declared_owner_as_caller(self):
        stage, phase = self.window_api()
        with patch.object(engine.os, 'getpid', return_value=999):
            with self.assertRaises(AssertionError):
                self.gate.record_window_start(stage, phase, self.ledger_sha)
        self.assertFalse(self.path.exists())

    def test_window_refuses_owner_pid_reuse(self):
        stage, phase = self.window_api()
        self.create(stage, phase)
        self.base.processes[11]['start'] = 'reused'
        with self.assertRaises(AssertionError):
            self.gate.observe_window(stage, phase, self.ledger_sha)

    def test_window_refuses_another_ledger(self):
        stage, phase = self.window_api()
        self.create(stage, phase)
        with self.assertRaises(AssertionError):
            self.gate.observe_window(stage, phase, 'b' * 64)

    def test_window_refuses_another_phase_even_with_valid_json(self):
        stage, phase = self.window_api()
        self.create(stage, phase)
        data = json.loads(self.path.read_text())
        data['phase_id'] = 'another-phase'
        self.path.write_text(json.dumps(data))
        with self.assertRaises(AssertionError):
            self.gate.observe_window(stage, phase, self.ledger_sha)

    def test_window_event_is_one_shot_no_late_reset(self):
        stage, phase = self.window_api()
        self.create(stage, phase)
        before = self.path.read_bytes()
        with self.assertRaises((AssertionError, FileExistsError)):
            self.create(stage, phase)
        self.assertEqual(self.path.read_bytes(), before)

    def test_expired_window_is_not_reopened_by_observation(self):
        stage, phase = self.window_api()
        self.create(stage, phase)
        data = json.loads(self.path.read_text())
        data['started_pc'] = 8900
        self.path.write_text(json.dumps(data))
        before = self.path.read_bytes()
        with self.assertRaises((AssertionError, TimeoutError)):
            self.gate.observe_window(stage, phase, self.ledger_sha)
        self.assertEqual(before, self.path.read_bytes())

    def test_window_path_cannot_come_from_a_ledger(self):
        stage, phase = self.window_api()
        self.gate.pair['window_events'] = {}
        with self.assertRaises(AssertionError):
            self.create(stage, phase)
        self.assertFalse(self.path.exists())


if __name__ == '__main__':
    unittest.main()
