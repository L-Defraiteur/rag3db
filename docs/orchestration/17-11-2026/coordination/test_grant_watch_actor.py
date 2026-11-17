"""Hermetic admission/loop witnesses: no real child, /proc or network."""
import copy
import hashlib
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
from types import SimpleNamespace
import os

spec = importlib.util.spec_from_file_location('actor', Path(__file__).with_name('grant_watch_actor.py'))
a = importlib.util.module_from_spec(spec)
spec.loader.exec_module(a)


def fixture():
    process = dict(pid=72, ppid=60, start='123', uid=1000, cgroup='/scope',
                   exe='/python', exe_sha256='e'*64, argv=['/python', '-B', '/actor', '--watch', '/config', 'c'*64],
                   script_path='/actor', script_sha256='s'*64, state='S', observed_monotonic=11.)
    config = dict(clock='CLOCK_MONOTONIC@luciepc', pilot='p'*40, allocation='C7-test',
                  prepared_path='/Q/prepared.json', prepared_sha256='q'*64,
                  actor_nonce='actor-one', ready_nonce='ready-one', issued_monotonic=10.,
                  arm_deadline_monotonic=20., common_end_monotonic=900., controller=dict(pid=60, start='100'),
                  authority=dict(pid=60, start='100', uid=1000, cgroup='/scope'),
                  actor=dict(script_path='/actor', script_sha256='s'*64, exe='/python', exe_sha256='e'*64,
                             argv_prefix=process['argv'][:-1]))
    receipt = dict(kind='ARMED', config_sha256='c'*64, actor_nonce='actor-one', ready_nonce='ready-one',
                   pilot=config['pilot'], allocation=config['allocation'], common_end_monotonic=900.,
                   process=copy.deepcopy(process), loop_counter=1, loop_observed_monotonic=11.)
    hb = dict(kind='HEARTBEAT', config_sha256='c'*64, actor_nonce='actor-one', pid=72, start='123',
              loop_counter=1, phase='WAIT_READY', observed_monotonic=11.)
    return config, receipt, process, hb


class Admission(unittest.TestCase):
    def test_complete_physical_admission(self):
        c, r, p, h = fixture()
        self.assertEqual(a.validate_armed(r, c, 'c'*64, p, 72), r)
        self.assertEqual(a.validate_heartbeat(r, c, h, 12.), h)

    def test_live_transition_is_not_identity_change(self):
        c, r, p, _ = fixture()
        r['process']['state'] = 'R'
        self.assertEqual(a.validate_armed(r, c, 'c'*64, p, 72), r)

    def test_reject_incomplete_arm_identity_and_dead_process(self):
        c, r, p, _ = fixture()
        for key, bad in [('pid', 73), ('ppid', 61), ('start', '124'), ('uid', 1001),
                         ('cgroup', '/other'), ('exe_sha256', 'x'*64), ('argv', ['wrong']),
                         ('script_sha256', 'x'*64), ('state', 'Z')]:
            with self.subTest(key=key):
                actual = dict(p, **{key: bad})
                with self.assertRaises(ValueError):
                    a.validate_armed(r, c, 'c'*64, actual, 72)
        for key, bad in [('loop_counter', 0), ('loop_observed_monotonic', 21.), ('actor_nonce', 'other')]:
            with self.subTest(key=key):
                with self.assertRaises(ValueError):
                    a.validate_armed(dict(r, **{key: bad}), c, 'c'*64, p, 72)

    def test_stale_future_or_unbound_heartbeat(self):
        c, r, _, h = fixture()
        for key, bad in [('observed_monotonic', 1.), ('observed_monotonic', 13.),
                         ('pid', 73), ('start', 'wrong'), ('config_sha256', 'wrong'),
                         ('phase', 'UNKNOWN'), ('loop_counter', 0)]:
            with self.subTest(key=key, bad=bad):
                with self.assertRaises(ValueError):
                    a.validate_heartbeat(r, c, dict(h, **{key: bad}), 12.)


class PhysicalSampler(unittest.TestCase):
    def test_real_sampler_on_fake_proc_accepts_R_then_S(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            process = root/'72'
            process.mkdir()
            executable = root/'python'
            executable.write_bytes(b'fake executable')
            (process/'exe').symlink_to(executable)
            script = root/'actor.py'
            script.write_bytes(b'fake actor')
            argv = ['/python', '-B', str(script), '--watch', '/config', 'c'*64]
            (process/'cmdline').write_bytes(('\0'.join(argv)+'\0').encode())
            (process/'cgroup').write_text('0::/scope\n')
            original = Path.read_text
            states = iter(('R', 'S'))
            def read(path, *args, **kwargs):
                if path == process/'stat':
                    fields = [next(states), '60']+['0']*17+['123']
                    return '72 (fake name) '+' '.join(fields)
                return original(path, *args, **kwargs)
            with patch.object(Path, 'read_text', read):
                value = a.process_snapshot(72, proc_root=root, clock=lambda: 11.)
            self.assertEqual(value['state'], 'S')
            self.assertEqual(value['start'], '123')
            self.assertEqual(value['script_sha256'], hashlib.sha256(b'fake actor').hexdigest())


class Loop(unittest.TestCase):
    def exercise(self, mode):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            config, _, physical, _ = fixture()
            keys = ('ready', 'grant', 'armed', 'grant_ack', 'result', 'release', 'first_red', 'evidence', 'heartbeat')
            config['paths'] = {k: str(root/(k+'.json')) for k in keys}
            path = root/'config.json'
            raw = json.dumps(config).encode()
            path.write_bytes(raw)
            sha = hashlib.sha256(raw).hexdigest()
            physical['argv'][-1] = sha
            tick = [11.]
            calls = []
            def snapshot(pid):
                self.assertEqual(pid, 72)
                return dict(physical, observed_monotonic=tick[0])
            def scope(_):
                calls.append('scope')
                if mode == 'authority':
                    raise ValueError('authority changed')
            def publish(cfg, ready_path, ready_sha, nonce):
                self.assertTrue(Path(cfg['paths']['armed']).exists())
                self.assertEqual(json.loads(Path(cfg['paths']['heartbeat']).read_text())['phase'], 'PUBLISHING')
                self.assertEqual(nonce, config['ready_nonce'])
                calls.append('contact')
                grant = dict(nonce=nonce, ready_sha256=ready_sha)
                Path(cfg['paths']['grant']).write_text(json.dumps(grant))
                return grant
            policy = SimpleNamespace(preflight=lambda c: calls.append('preflight'), publish_ready=publish)
            def sleeper(delay):
                tick[0] += delay
                if mode == 'red':
                    Path(config['paths']['first_red']).write_text('{}')
                elif mode == 'dead':
                    physical['state'] = 'Z'
                elif mode == 'release_early':
                    Path(config['paths']['release']).write_text('{}')
                elif not Path(config['paths']['ready']).exists():
                    ready = dict(clock=a.CLOCK, nonce=config['ready_nonce'], deadline=900.,
                                 ready_observed_monotonic=tick[0], grant_deadline_monotonic=tick[0]+120,
                                 phase=dict(pilot=config['pilot'], allocation=config['allocation'],
                                            authority=config['authority'], first_red_path=config['paths']['first_red']))
                    Path(config['paths']['ready']).write_text(json.dumps(ready))
                else:
                    release = dict(kind='DRIVER_FINISHED', actor_nonce=config['actor_nonce'],
                                   config_sha256=sha, pilot=config['pilot'], allocation=config['allocation'])
                    Path(config['paths']['release']).write_text(json.dumps(release))
            if mode == 'late_exec':
                tick[0] = 21.
            if mode == 'ready_before_arm':
                Path(config['paths']['ready']).write_text('{}')
            with patch.object(a.os, 'getpid', return_value=72):
                invoke = lambda: a.run_actor(path, sha, clock=lambda: tick[0], sleeper=sleeper,
                                            snapshot=snapshot, scope_check=scope,
                                            policy_loader=lambda c: policy, inputs_check=lambda *args: None)
                if mode == 'success':
                    self.assertEqual(invoke()['kind'], 'RELEASED')
                    self.assertEqual(calls.count('contact'), 1)
                    self.assertEqual(json.loads(Path(config['paths']['grant_ack']).read_text())['kind'], 'GRANT')
                else:
                    with self.assertRaises(ValueError):
                        invoke()
                    self.assertNotIn('contact', calls)
                    self.assertEqual(json.loads(Path(config['paths']['result']).read_text())['kind'], 'REFUSED')

    def test_arm_ready_single_publish_then_authenticated_release(self):
        self.exercise('success')

    def test_firstred_during_wait(self):
        self.exercise('red')

    def test_actor_dies_after_arm(self):
        self.exercise('dead')

    def test_release_cannot_precede_grant(self):
        self.exercise('release_early')

    def test_late_exec_before_any_contact(self):
        self.exercise('late_exec')

    def test_ready_cannot_precede_physical_arm(self):
        self.exercise('ready_before_arm')

    def test_authority_refusal_before_policy_import(self):
        self.exercise('authority')


if __name__ == '__main__':
    unittest.main()
