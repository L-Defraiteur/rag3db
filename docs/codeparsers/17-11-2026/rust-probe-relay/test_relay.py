"""Témoins publiés avant implantation : vrai Relay, transport et horloge factices."""
import importlib.util
import hashlib
import json
from pathlib import Path
import tempfile
import unittest

spec = importlib.util.spec_from_file_location('rust_probe_relay_test', Path(__file__).with_name('relay.py'))
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)


class Clock:
    def __init__(self):
        self.now = 1000000.0
    def __call__(self):
        return self.now
    def sleep(self, seconds):
        self.now += seconds


class FakeTransport:
    """Aucune logique de relais ici : réponses réseau scénarisées seulement."""
    def __init__(self, clock):
        self.clock = clock
        self.calls = []
        self.failed_file = None
        self.closed = False
        self.phase_running = True
        self.stage = 'build'
        self.expired = False
        self.red = False
        self.latency = 0
        self.remaining = 20.0
        self.payload = b'ELF synthetic bytes'
    def contents(self, kind):
        ledger = {'elf_sha256': hashlib.sha256(self.payload).hexdigest()}
        files = {'elf': self.payload} if kind == 'build' else {'all': b'test: test', 'ignored': b'test: test'}
        if kind == 'runtime':
            ledger.update(inventory_sha256=hashlib.sha256(files['all']).hexdigest(),
                ignored_inventory_sha256=hashlib.sha256(files['ignored']).hexdigest())
        files['ledger'] = json.dumps(ledger).encode()
        return ledger, files
    def ready(self):
        self.calls.append('ready')
    def start_phase(self):
        self.calls.append('start')
    def snapshot(self, timeout):
        self.calls.append('snapshot')
        self.clock.now += self.latency
        if self.red:
            raise RuntimeError('PC first red')
        if self.expired:
            return {'state': 'active', 'remaining': 0, 'stage': self.stage}
        ledger, files = self.contents(self.stage)
        return {'state': 'active', 'remaining': self.remaining, 'stage': self.stage,
                'ledger': ledger, 'files': {k:hashlib.sha256(v).hexdigest() for k,v in files.items()}}
    def fetch(self, kind, name, destination, timeout):
        self.calls.append('fetch:' + kind + ':' + name)
        if name == self.failed_file:
            raise OSError('fixture network failure')
        data = self.contents(kind)[1][name]
        Path(destination).write_bytes(data)
        return hashlib.sha256(data).hexdigest()
    def publish_ack(self, kind, ack, timeout):
        self.calls.append('ack:' + kind)
        if kind == 'build':
            self.stage = 'runtime'
        else:
            self.stage = 'done'
            self.phase_running = False
    def running(self):
        return self.phase_running
    def finish(self, timeout):
        self.calls.append('finish')
        return 0
    def abort(self, reason, timeout):
        self.calls.append('abort')
    def stop(self):
        self.calls.append('stop')
        self.closed = True


class RelayTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.clock = Clock()
        self.transport = FakeTransport(self.clock)
        self.relay = module.Relay(self.transport, Path(self.tmp.name), self.clock, self.clock.sleep,
                                  queue_budget=300, transfer_cap=30, control_cap=5)
    def test_ready_precedes_start_and_two_publications_are_ordered(self):
        self.assertEqual(self.relay.run(), 0)
        calls = self.transport.calls
        self.assertLess(calls.index('ready'), calls.index('start'))
        self.assertLess(calls.index('fetch:build:elf'), calls.index('ack:build'))
        self.assertLess(calls.index('ack:build'), calls.index('fetch:runtime:ledger'))
        self.assertLess(calls.index('fetch:runtime:ignored'), calls.index('ack:runtime'))
        self.assertTrue(self.transport.closed)
    def test_pc_remaining_is_a_duration_not_an_absolute_local_clock(self):
        self.assertEqual(self.relay.run(), 0)
        self.assertLessEqual(self.relay.deadline, 1000020)
    def test_round_trip_latency_consumes_remaining(self):
        self.transport.latency = 21
        with self.assertRaises(TimeoutError):
            self.relay.run()
        self.assertFalse(any(c.startswith('fetch:') for c in self.transport.calls))
        self.assertTrue(self.transport.closed)
    def test_pc_expired_refuses_even_first_copy(self):
        self.transport.expired = True
        with self.assertRaises(TimeoutError):
            self.relay.run()
        self.assertFalse(any(c.startswith('fetch:') for c in self.transport.calls))
    def test_pc_red_never_gets_ack(self):
        self.transport.red = True
        with self.assertRaises(RuntimeError):
            self.relay.run()
        self.assertFalse(any(c.startswith('ack:') for c in self.transport.calls))
        self.assertTrue(self.transport.closed)
    def test_first_copy_error_stops_without_retry_or_ack(self):
        self.transport.failed_file = 'elf'
        with self.assertRaises(OSError):
            self.relay.run()
        self.assertEqual(self.transport.calls.count('fetch:build:elf'), 1)
        self.assertNotIn('ack:build', self.transport.calls)
        self.assertTrue(self.transport.closed)
    def test_pc_red_after_copy_cannot_publish_ack(self):
        old = self.transport.fetch
        def fetch(kind, name, destination, timeout):
            result = old(kind, name, destination, timeout)
            self.transport.red = True
            return result
        self.transport.fetch = fetch
        with self.assertRaises(RuntimeError):
            self.relay.run()
        self.assertNotIn('ack:build', self.transport.calls)
        self.assertFalse((Path(self.tmp.name) / 'build/ledger').exists())
    def test_corrupted_copy_cannot_be_linked_or_acknowledged(self):
        old = self.transport.fetch
        def fetch(kind, name, destination, timeout):
            result = old(kind, name, destination, timeout)
            Path(destination).write_bytes(b'corrupt')
            return result
        self.transport.fetch = fetch
        with self.assertRaises(AssertionError):
            self.relay.run()
        self.assertNotIn('ack:build', self.transport.calls)
    def test_no_extra_time_after_active_phase_started(self):
        self.relay.refresh()
        initial = self.relay.deadline
        self.clock.now += 1
        self.transport.remaining = 200
        self.relay.refresh()
        self.assertEqual(self.relay.deadline, initial)
    def test_atomic_json_links_only_after_fsync_and_guard(self):
        from unittest.mock import patch
        order = []
        original_link = module.os.link
        original_fsync = module.os.fsync
        def fsync(fd):
            order.append('fsync')
            return original_fsync(fd)
        def link(src, dest):
            order.append('link')
            return original_link(src, dest)
        path = Path(self.tmp.name) / 'atomic.json'
        with patch.object(module.os, 'fsync', fsync), patch.object(module.os, 'link', link):
            module.atomic_json(path, {'ready': True}, lambda: order.append('guard'))
        self.assertEqual(order[:3], ['fsync', 'guard', 'link'])
    def test_guard_failure_after_fsync_leaves_no_visible_marker(self):
        def expired():
            raise TimeoutError('PC ended')
        path = Path(self.tmp.name) / 'atomic.json'
        with self.assertRaises(TimeoutError):
            module.atomic_json(path, {}, expired)
        self.assertFalse(path.exists())
    def test_phase_exit_before_build_is_failure(self):
        self.transport.phase_running = False
        with self.assertRaises(RuntimeError):
            self.relay.run()
        self.assertNotIn('ack:build', self.transport.calls)
    def test_signal_exception_stops_owned_transport(self):
        def interrupted(timeout):
            raise KeyboardInterrupt('fake SIG')
        self.transport.snapshot = interrupted
        with self.assertRaises(KeyboardInterrupt):
            self.relay.run()
        self.assertTrue(self.transport.closed)


class GateTests(unittest.TestCase):
    def setUp(self):
        from types import SimpleNamespace
        from unittest.mock import patch
        import os
        gate_spec = importlib.util.spec_from_file_location('relay_gate_test', Path(__file__).with_name('gate.py'))
        self.gate_module = importlib.util.module_from_spec(gate_spec)
        gate_spec.loader.exec_module(self.gate_module)
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)
        self.control = self.root / 'control'
        self.control.mkdir()
        (self.control / 'pilot.py').write_text('synthetic source')
        scope = self.root / 'scope/validation-fake'
        scope.mkdir(parents=True)
        self.receipt = scope / 'autorite.json'
        self.authority = {'etat': 'actif', 'pid': 10, 'debut': '1000', 'cgroup': '/scope',
            'echeance': 9060, 'script': str(self.control / 'confiner_validation.py'),
            'script_sha256': 'common-sha'}
        self.receipt.write_text(json.dumps(self.authority))
        pair = {'control': str(self.control), 'proofs': str(self.root.parent),
            'controllers': {'confiner_validation.py': 'common-sha'}, 'timeout': '/timeout',
            'timeout_sha256': 'timeout-sha', 'driver_sha256': module.digest(self.control / 'pilot.py')}
        command = ['python', str(self.control / 'confiner_validation.py'), 'lancer']
        driver_args = [self.gate_module.sys.executable, '-B', str(self.control / 'pilot.py'),
            'inside', '--proofs', pair['proofs'], '--phase', 'baseline']
        base = {'uid': os.getuid(), 'exe': '/python', 'cgroup': '0::/scope', 'state': 'S'}
        common = dict(base, pid=10, start='1000', parent=1, argv=command)
        driver = dict(base, pid=11, start='1001', parent=10, argv=driver_args)
        timeout_args = ['/timeout', '--signal=TERM', '--kill-after=2s', '1798s']
        timeout = dict(base, pid=9, start=str(9000 * os.sysconf('SC_CLK_TCK')), parent=1,
            argv=timeout_args, role='outer-timeout')
        self.processes = {10: dict(common), 11: dict(driver), 9: dict(timeout)}
        info = {'path': str(self.receipt), 'driver_identity': driver, 'supervisor_identity': common,
                'driver_script_sha256': pair['driver_sha256']}
        (self.root / 'authority-path.json').write_text(json.dumps(info))
        (self.root / 'common.command.json').write_text(json.dumps(command))
        (self.root / 'controls.json').write_text(json.dumps([timeout]))
        self.gate = self.gate_module.Gate.__new__(self.gate_module.Gate)
        self.gate.pair, self.gate.pin, self.gate.phase = pair, 'pin', 'baseline'
        self.gate.root, self.gate.control = self.root, self.control
        self.gate.guard = SimpleNamespace(process_record=lambda pid: self.processes[pid])
        self.gate.pilot = SimpleNamespace(outer_command=lambda *args: ['poste', 'lourd', *timeout_args])
        self.gate.verify = lambda: None
        self.gate.launcher = lambda: {'coordinator': {'host': 'peer'}}
        original_digest = self.gate_module.digest
        patcher = patch.object(self.gate_module, 'digest', lambda path:
            'timeout-sha' if str(path) == '/timeout' else original_digest(path))
        patcher.start()
        self.addCleanup(patcher.stop)
        timer = patch.object(self.gate_module.time, 'monotonic', return_value=9000)
        timer.start()
        self.addCleanup(timer.stop)

    def test_real_active_gate_returns_pc_remaining_duration(self):
        self.assertEqual(self.gate.active()['remaining'], 45)

    def test_pid_reuse_refuses_active_gate(self):
        self.processes[10]['start'] = 'reused'
        with self.assertRaises(AssertionError):
            self.gate.active()

    def test_changed_driver_script_refuses_active_gate(self):
        (self.control / 'pilot.py').write_text('changed source')
        with self.assertRaises(AssertionError):
            self.gate.active()

    def test_ended_authority_refuses_active_gate(self):
        self.authority['etat'] = 'termine'
        self.receipt.write_text(json.dumps(self.authority))
        with self.assertRaises(AssertionError):
            self.gate.active()

    def test_final_scope_verdict_refuses_active_gate(self):
        self.receipt.with_name('verdict.json').write_text('{}')
        with self.assertRaises(AssertionError):
            self.gate.active()

    def test_pc_expired_refuses_active_gate(self):
        self.authority['echeance'] = 9010
        self.receipt.write_text(json.dumps(self.authority))
        with self.assertRaises(AssertionError):
            self.gate.active()

    def test_real_ack_gate_rechecks_after_fsync_before_link(self):
        self.gate.pair['peer_host'] = 'peer'
        state = {'coordinator': {'host': 'peer'}, 'files': {'ledger': 'ledger-sha'},
            'ledger': {'elf_sha256': 'elf-sha'}}
        expected_files = {name: str(self.root / 'relay-local/build' / name) for name in ['ledger', 'elf']}
        ack = {'kind': 'build', 'phase': 'baseline', 'host': 'peer', 'prepared_sha256': 'pin',
            'coordinator': state['coordinator'], 'peer_files': expected_files,
            'ledger_sha256': 'ledger-sha', 'elf_sha256': 'elf-sha', 'copied_and_rehashed': True}
        from unittest.mock import Mock
        self.gate.stage = Mock(side_effect=[(state, {}), AssertionError('phase ended before link')])
        with self.assertRaisesRegex(AssertionError, 'phase ended before link'):
            self.gate.acknowledge('build', ack)
        self.assertFalse((self.root / 'build-peer-ack.json').exists())


if __name__ == '__main__':
    unittest.main()
