"""Témoins publiés avant implantation : vrai Relay, transport et horloge factices."""
import importlib.util
import hashlib
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
        return {'state': 'active', 'remaining': self.remaining, 'stage': self.stage,
                'ledger': {'elf_sha256': hashlib.sha256(self.payload).hexdigest()}}
    def fetch(self, kind, name, destination, timeout):
        self.calls.append('fetch:' + kind + ':' + name)
        if name == self.failed_file:
            raise OSError('fixture network failure')
        Path(destination).write_bytes(self.payload)
        return hashlib.sha256(self.payload).hexdigest()
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


if __name__ == '__main__':
    unittest.main()
