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
implementation = getattr(module, "engine", module)


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
        self.prepared_sha256 = '1' * 64
        self.phase = 'baseline'
        self.pair = {'execution_id': 'fixture-execution', 'sdk': 'fixture-sdk',
            'publication_limits': module.default_limits(),
            'corpus_manifest_sha256': 'c' * 64,
            'phases': {'baseline': {'source': 'f' * 40, 'parser': 'b' * 40}}}
        self.plan = module.default_plan(self.pair, self.phase)
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
        return self.wire(self.stage, ledger, files)
    def wire(self, label, ledger, files):
        plan = implementation.validate_plan(self.plan)
        declared = plan.stage(label)
        header = {'protocol_version': 1, 'execution_id': plan.execution_id,
            'phase_id': plan.phase_id, 'sequence': plan.position(label), 'stage': label,
            'prepared_sha256': self.prepared_sha256, 'plan_sha256': plan.sha256,
            'binding_sha256': declared.binding_sha256}
        observed = {'fixture_stage': label, 'ledger_keys': sorted(ledger)}
        return {'state': 'active', 'remaining': self.remaining, 'phase_remaining': self.remaining,
            'stage': label, 'ledger': ledger, 'header': header,
            'files': {k:hashlib.sha256(v).hexdigest() for k,v in files.items()},
            'file_sizes': {k:len(v) for k,v in files.items()},
            'observed_binding': observed,
            'observed_binding_sha256': hashlib.sha256(json.dumps(observed,sort_keys=True,
                separators=(',', ':')).encode()).hexdigest(),
            'elf_map': {artifact: {'stage': ref['stage'], 'file': ref['file'],
                'sha256': hashlib.sha256(self.contents(ref['stage'])[1][ref['file']]).hexdigest()}
                for artifact,ref in declared.executables.items()},
            'window': None, 'publication_remaining': None,
            'coordinator': {'host': 'peer'}}
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
    def test_finish_past_phase_deadline_never_returns_success(self):
        old = self.transport.finish
        def finish(timeout):
            self.clock.now += 25
            return old(timeout)
        self.transport.finish = finish
        with self.assertRaises(TimeoutError):
            self.relay.run()
        self.assertTrue(self.transport.closed)
    def test_launch_ssh_contains_remote_physical_timeout(self):
        import shlex
        transport = module.SSHTransport.__new__(module.SSHTransport)
        transport.pair = {'proofs': '/proofs', 'timeout': '/usr/bin/timeout',
            'publication_limits': module.default_limits()}
        transport.control = Path('/proofs/control')
        transport.phase, transport.pin, transport.host = 'baseline', 'pin', 'luciepc'
        argv = transport.command('launch', '--allocation', 'GO')
        remote = shlex.split(argv[-1].removeprefix('exec '))
        self.assertEqual(remote[:4], ['/usr/bin/timeout', '--signal=TERM', '--kill-after=2s', '2098s'])
    def test_real_cli_execs_physical_timeout_before_creating_relay(self):
        from unittest.mock import patch
        import os
        root = Path(self.tmp.name)
        control = root / 'control'
        control.mkdir()
        relay_path = control / 'relay.py'
        relay_path.write_bytes(Path(module.__file__).read_bytes())
        timeout_path = control / 'fake-timeout'
        timeout_path.write_bytes(b'fake timeout executable, never executed')
        core_path = control / 'publication_relay.py'
        core_path.write_bytes(Path(implementation.__file__).read_bytes())
        (control / 'physical_guard.py').write_text(
            'def process_record(pid): return {"pid":pid,"exe":"/unbounded","argv":[],"uid":0}\n')
        pair = {'proofs': str(root), 'control': str(control), 'peer_host': 'fake-peer',
            'publication_limits': module.default_limits(),
            'controllers': {'relay.py': module.digest(relay_path),
                'publication_relay.py': module.digest(core_path)},
            'peer_timeout': {'path': str(timeout_path), 'sha256': module.digest(timeout_path)},
            'relay': {'queue_budget': 300, 'transfer_cap': 30, 'control_cap': 5, 'stop_reserve': 5}}
        (root / 'prepared.json').write_text(json.dumps(pair))
        argv = [str(relay_path), '--proofs', str(root), '--phase', 'baseline', '--allocation', 'GO']
        class ExecTransition(Exception):
            pass
        original_cwd = Path.cwd()
        try:
            with patch.object(module, '__file__', str(relay_path)), patch.object(implementation, '__file__', str(core_path)), \
                 patch.object(module.sys, 'argv', argv), \
                 patch.object(module.socket, 'gethostname', return_value='fake-peer'), \
                 patch.object(module.os, 'execve', side_effect=ExecTransition) as execute:
                with self.assertRaises(ExecTransition):
                    module.main()
                command = execute.call_args.args[1]
                self.assertEqual(command[:4], [str(timeout_path), '--signal=TERM', '--kill-after=2s', '2098s'])
                self.assertFalse((root / 'baseline/relay-local').exists())
        finally:
            os.chdir(original_cwd)
    def test_fsync_during_relay_publication_cannot_leave_green(self):
        from unittest.mock import patch
        path = Path(self.tmp.name) / 'result.json'
        ticks = iter([9, 10])
        result = module.publish_result(path, {'verdict': 'VERT'}, 10, lambda: next(ticks))
        self.assertEqual(result['verdict'], 'ROUGE')
        self.assertEqual(json.loads(path.read_text())['verdict'], 'ROUGE')
    def test_late_close_after_publication_rewrites_only_relay_red(self):
        from unittest.mock import patch
        original = module.publish_result
        def late(path, record, deadline, clock):
            result = original(path, record, deadline, clock)
            self.clock.now += 25
            return result
        with patch.object(implementation, 'publish_result', side_effect=late):
            with self.assertRaises(TimeoutError):
                self.relay.run()
        result = json.loads((Path(self.tmp.name) / 'relay-result.json').read_text())
        self.assertEqual(result['verdict'], 'ROUGE')
        self.assertTrue(result['native_verdict_unchanged'])
    def test_publication_expiry_still_requests_remote_stop_in_reserved_phase_time(self):
        def expired(timeout):
            return {'state': 'active', 'remaining': 0, 'phase_remaining': 15, 'stage': 'build'}
        self.transport.snapshot = expired
        with self.assertRaises(TimeoutError):
            self.relay.run()
        self.assertIn('abort', self.transport.calls)
        self.assertTrue(self.transport.closed)
    def test_queue_expiry_uses_stop_reserve_inside_queue_deadline(self):
        old = self.transport.start_phase
        def late_queue():
            old()
            self.clock.now += 296
        self.transport.start_phase = late_queue
        self.transport.snapshot = lambda timeout: {'state': 'waiting'}
        with self.assertRaises(TimeoutError):
            self.relay.run()
        self.assertIn('abort', self.transport.calls)
        self.assertLessEqual(self.clock.now, 1000300)
    def test_exhausted_cleanup_budget_is_unknown_not_silently_quiet(self):
        self.transport.snapshot = lambda timeout:{'state': 'active', 'remaining': 0,
            'phase_remaining': 0, 'stage': 'build'}
        with self.assertRaises(TimeoutError):
            self.relay.run()
        result = json.loads((Path(self.tmp.name) / 'cleanup-refused.json').read_text())
        self.assertEqual(result['remote_quiet'], 'UNKNOWN')
        self.assertNotIn('abort', self.transport.calls)
        self.assertTrue(self.transport.closed)
    def test_closed_native_phase_is_not_reopened_for_late_relay_refusal(self):
        old = self.transport.finish
        def finish(timeout):
            result = old(timeout)
            self.clock.now += 25
            return result
        self.transport.finish = finish
        with self.assertRaises(TimeoutError):
            self.relay.run()
        self.assertNotIn('abort', self.transport.calls)
        result = json.loads((Path(self.tmp.name) / 'cleanup-not-required.json').read_text())
        self.assertEqual(result['phase_ssh_rc'], 0)
        self.assertTrue(result['native_verdict_unchanged'])
    def test_finish_wait_leaves_cleanup_reserve_inside_phase_deadline(self):
        requested = []
        old = self.transport.finish
        def finish(timeout):
            requested.append(timeout)
            return old(timeout)
        self.transport.finish = finish
        self.assertEqual(self.relay.run(), 0)
        self.assertLessEqual(requested[0], 15)
        self.assertGreater(requested[0], 0)
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
        (self.control / 'gate.py').write_bytes(Path(self.gate_module.__file__).read_bytes())
        scope = self.root / 'scope/validation-fake'
        scope.mkdir(parents=True)
        self.receipt = scope / 'autorite.json'
        self.authority = {'etat': 'actif', 'pid': 10, 'debut': '1000', 'cgroup': '/scope',
            'echeance': 9060, 'script': str(self.control / 'confiner_validation.py'),
            'script_sha256': 'common-sha'}
        self.receipt.write_text(json.dumps(self.authority))
        (scope / 'recette.json').write_text(json.dumps({'delai': 60}))
        pair = {'control': str(self.control), 'proofs': str(self.root.parent),
            'publication_limits': module.default_limits(),
            'controllers': {'confiner_validation.py': 'common-sha',
                'gate.py': module.digest(self.control / 'gate.py'),
                'pilot.py': module.digest(self.control / 'pilot.py')}, 'timeout': '/timeout',
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
        self.gate.pair, self.gate.pin, self.gate.phase = pair, '1' * 64, 'baseline'
        self.gate.root, self.gate.control = self.root, self.control
        template = FakeTransport(Clock())
        self.gate.plan = implementation.validate_plan(template.plan)
        self.gate.guard = SimpleNamespace(process_record=lambda pid: self.processes[pid])
        def phase_info(*args):
            data = json.loads((self.root / 'authority-path.json').read_text())
            return dict(data, scope_root=str(scope.parent), common_entry='confiner_validation.py',
                driver_entry='pilot.py', driver_argv=driver_args,
                common_argv=json.loads((self.root / 'common.command.json').read_text()),
                controls_path=str(self.root / 'controls.json'), deadline_role='outer-timeout',
                deadline_argv=timeout_args, original_common_end=9060)
        self.gate.pilot = SimpleNamespace(
            publication_phase_info=phase_info,
            outer_command=lambda *args: ['poste', 'lourd', *timeout_args])
        # Données de processus seulement : bootstrap et ses gardes restent réels.
        hard = dict(base, pid=8, start=str(9000 * os.sysconf('SC_CLK_TCK')), parent=1,
            exe='/timeout', argv=['/timeout', '--signal=TERM', '--kill-after=2s', '2098s'])
        self.processes[8] = hard
        (self.root / 'relay-bootstrap.json').write_text(json.dumps({
            'prepared_sha256': '1' * 64, 'gate_sha256': pair['controllers']['gate.py'],
            'hard_timeout': hard}))
        self.gate.verify = lambda: None
        self.gate.launcher = lambda: {'coordinator': {'host': 'peer'}}
        original_digest = self.gate_module.digest
        patcher = patch.object(getattr(self.gate_module, 'engine', self.gate_module), 'digest', lambda path:
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
        (scope / 'recette.json').write_text(json.dumps({'delai': 60}))
        with self.assertRaises(AssertionError):
            self.gate.active()

    def test_final_scope_verdict_refuses_active_gate(self):
        self.receipt.with_name('verdict.json').write_text('{}')
        with self.assertRaises(AssertionError):
            self.gate.active()

    def test_pc_expired_refuses_active_gate(self):
        self.authority['echeance'] = 9010
        self.receipt.write_text(json.dumps(self.authority))
        (scope / 'recette.json').write_text(json.dumps({'delai': 60}))
        with self.assertRaises(AssertionError):
            self.gate.active()

    def test_waiting_abort_does_not_require_active_native_authority(self):
        from unittest.mock import Mock
        self.gate.active = Mock(side_effect=AssertionError('not active yet'))
        self.gate.cancel_launcher = Mock(return_value={'launcher_absent': True})
        result = self.gate.abort('queue interrupted')
        self.assertTrue(result['launcher_absent'])
        self.gate.active.assert_not_called()
    def test_launch_does_not_publish_future_argv_as_live_identity(self):
        from unittest.mock import patch
        import os
        self.gate.root = self.root / 'fresh-phase'
        self.gate.root.mkdir()
        relay = self.control / 'relay.py'
        relay.write_text('synthetic relay')
        self.gate.pair['peer_host'] = 'peer'
        self.gate.pair['tool_root'] = '/fake/tool-root'
        self.gate.pair['controllers']['relay.py'] = module.digest(relay)
        identity = {'pid': os.getpid(), 'uid': os.getuid(), 'start': '1234',
            'state': 'R', 'exe': '/python', 'cgroup': '0::/scope', 'parent': os.getppid(),
            'argv': [self.gate_module.sys.executable, '-B', str(self.control / 'gate.py'), 'launch']}
        self.processes[os.getpid()] = identity
        prefix = ['/timeout', '--signal=TERM', '--kill-after=2s', '2098s']
        args = [self.gate_module.sys.executable, '-B', str(self.control / 'gate.py'), 'launch',
            '--proofs', self.gate.pair['proofs'], '--phase', 'baseline', '--pin', '1' * 64, '--allocation', 'GO']
        self.processes[os.getppid()] = dict(identity, pid=os.getppid(), parent=1,
            argv=prefix + args, exe='/timeout')
        ready = {'host': 'peer', 'identity': {'uid': os.getuid(), 'pid': 123, 'start': '1234',
                 'argv': [self.gate_module.sys.executable, '-B', str(relay)]},
                 'prepared_sha256': '1' * 64, 'phase': 'baseline', 'script_sha256': module.digest(relay)}
        class ExecTransition(Exception):
            pass
        with patch.object(self.gate_module.os, 'execve', side_effect=ExecTransition):
            with self.assertRaises(ExecTransition):
                self.gate.launch('GO', ready)
        self.assertFalse((self.gate.root / 'relay-launch.json').exists(), 'future argv exposed before exec')
        bootstrap = json.loads((self.gate.root / 'relay-bootstrap.json').read_text())
        self.assertEqual(bootstrap['identity']['argv'], identity['argv'])
    def test_real_snapshot_accepts_bounded_exec_transition(self):
        hard = dict(self.processes[9], exe='/timeout', argv=['/timeout', '--signal=TERM', '--kill-after=2s', '2098s'])
        identity = dict(self.processes[11], argv=['python', str(self.control / 'gate.py'), 'launch'])
        planned = ['python', str(self.control / 'pilot.py'), 'run']
        self.processes[9], self.processes[11] = hard, identity
        bootstrap = {'identity': identity, 'run_argv': planned, 'hard_timeout': hard,
            'prepared_sha256': '1' * 64, 'gate_sha256': self.gate.pair['controllers']['gate.py'],
            'coordinator': {'host': 'peer'}, 'transition_end': 9005, 'queue_end': 9300}
        (self.root / 'relay-bootstrap.json').write_text(json.dumps(bootstrap))
        # Employer les vraies méthodes de rendez-vous, pas le stub de l'ancienne fixture active.
        self.gate.launcher = self.gate_module.Gate.launcher.__get__(self.gate)
        self.assertEqual(self.gate.snapshot()['state'], 'waiting')
        self.processes[11] = dict(identity, argv=planned)
        self.assertEqual(self.gate.snapshot()['state'], 'waiting')
        (self.root / 'authority-path.json').unlink()
        (self.root / 'relay-launch.json').write_text(json.dumps({'identity': self.processes[11],
            'prepared_sha256': '1' * 64, 'coordinator': bootstrap['coordinator'],
            'launcher_script_sha256': self.gate.pair['driver_sha256']}))
        self.assertEqual(self.gate.snapshot()['state'], 'waiting')
        self.processes[11]['start'] = 'reused'
        with self.assertRaises(AssertionError):
            self.gate.snapshot()
    def test_real_queued_cancel_uses_pidfd_and_attests_both_roles_gone(self):
        from unittest.mock import patch, Mock
        import os
        hard = dict(self.processes[9], exe='/timeout', argv=['/timeout', '--signal=TERM', '--kill-after=2s', '2098s'])
        launcher = dict(self.processes[11], argv=['python', str(self.control / 'pilot.py'), 'run'])
        self.processes = {9: hard, 11: launcher}
        def process(pid):
            if pid not in self.processes:
                raise FileNotFoundError()
            return self.processes[pid]
        self.gate.guard.process_record = process
        self.gate.verify_contract = lambda: None
        self.gate.active = Mock(side_effect=AssertionError('no native authority'))
        self.gate.pilot.first_red = lambda root, error: module.atomic_json(root / 'first-red.json', {'reason': repr(error)})
        (self.root / 'authority-path.json').unlink()
        (self.root / 'controls.json').unlink()
        import shutil
        shutil.rmtree(self.root / 'scope')  # Une vraie attente queued n'a pas créé Common.
        bootstrap = {'identity': launcher, 'run_argv': launcher['argv'], 'hard_timeout': hard,
            'prepared_sha256': '1' * 64, 'gate_sha256': self.gate.pair['controllers']['gate.py']}
        (self.root / 'relay-bootstrap.json').write_text(json.dumps(bootstrap))
        sent = []
        def send(fd, sig):
            sent.append((fd - 100000, sig))
            self.processes.clear()  # Faux timeout : son groupe attribué se termine, pas de signal OS.
        original_close = os.close
        with patch.object(self.gate_module.os, 'pidfd_open', side_effect=lambda pid:100000+pid), \
             patch.object(self.gate_module.signal, 'pidfd_send_signal', side_effect=send), \
             patch.object(self.gate_module.os, 'close', side_effect=lambda fd:None if fd>=100000 else original_close(fd)):
            result = self.gate.cancel_launcher('queue interrupted')
        self.assertTrue(result['launcher_absent'])
        self.assertEqual(result['unknown'], [])
        self.assertEqual(sent[0][0], 9)
        self.gate.active.assert_not_called()
    def test_cancel_with_spent_loop_budget_never_invents_remote_quiet(self):
        from unittest.mock import patch
        import os, shutil
        hard = dict(self.processes[9], exe='/timeout', argv=['/timeout', '--signal=TERM', '--kill-after=2s', '2098s'])
        launcher = dict(self.processes[11], argv=['python', str(self.control / 'pilot.py'), 'run'])
        self.processes = {9: hard, 11: launcher}
        self.gate.guard.process_record = lambda pid:self.processes[pid]
        self.gate.verify_contract = lambda: None
        self.gate.pilot.first_red = lambda root, error: module.atomic_json(root / 'first-red.json', {'reason': repr(error)})
        (self.root / 'authority-path.json').unlink()
        (self.root / 'controls.json').unlink()
        shutil.rmtree(self.root / 'scope')
        (self.root / 'relay-bootstrap.json').write_text(json.dumps({'identity': launcher,
            'run_argv': launcher['argv'], 'hard_timeout': hard, 'prepared_sha256': '1' * 64,
            'gate_sha256': self.gate.pair['controllers']['gate.py']}))
        ticks = iter([0, 0, 4, 4, 4])
        original_close = os.close
        with patch.object(self.gate_module.time, 'monotonic', side_effect=lambda:next(ticks, 4)), \
             patch.object(self.gate_module.os, 'pidfd_open', side_effect=lambda pid:100000+pid), \
             patch.object(self.gate_module.signal, 'pidfd_send_signal'), \
             patch.object(self.gate_module.os, 'close', side_effect=lambda fd:None if fd>=100000 else original_close(fd)):
            with self.assertRaises(AssertionError):
                self.gate.cancel_launcher('expired cancellation, roles still alive')
        result = json.loads((self.root / 'relay-cancel.json').read_text())
        self.assertFalse(result['launcher_absent'])
        self.assertTrue(result['alive_roles'])
    def test_final_observation_keeps_unknown_instead_of_absence(self):
        from unittest.mock import Mock
        self.gate.role_alive = Mock(side_effect=PermissionError('fake protected process'))
        identity = self.processes[9]
        alive, unknown, observations = self.gate.final_roles([('watchdog', identity, None)])
        self.assertEqual(alive, [])
        self.assertTrue(unknown)
        self.assertEqual(observations[0]['status'], 'UNKNOWN')
        self.assertEqual(observations[0]['start'], identity['start'])
    def test_real_ack_gate_rechecks_after_fsync_before_link(self):
        self.gate.pair['peer_host'] = 'peer'
        template = FakeTransport(Clock())
        state = template.snapshot(5)
        expected_files = {name: str(self.root / 'relay-local/build' / name) for name in state['files']}
        ack = {'kind': 'build', 'phase': 'baseline', 'host': 'peer', 'prepared_sha256': '1' * 64,
            'coordinator': state['coordinator'], 'peer_files': expected_files,
            'header': state['header'], 'files': state['files'], 'file_sizes': state['file_sizes'],
            'elf_map': state['elf_map'], 'observed_binding_sha256': state['observed_binding_sha256'],
            'ledger_sha256': state['files']['ledger'], 'elf_sha256': state['elf_map']['probe']['sha256'],
            'copied_and_rehashed': True}
        from unittest.mock import Mock
        self.gate.stage = Mock(side_effect=[(state, {}), AssertionError('phase ended before link')])
        with self.assertRaisesRegex(AssertionError, 'phase ended before link'):
            self.gate.acknowledge('build', ack)
        self.assertFalse((self.root / 'build-peer-ack.json').exists())


if __name__ == '__main__':
    unittest.main()
