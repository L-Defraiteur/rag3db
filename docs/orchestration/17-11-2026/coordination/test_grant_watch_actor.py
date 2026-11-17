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


class RoleSampler(unittest.TestCase):
    def test_generic_python_role_uses_expected_script_before_hash(self):
        with tempfile.TemporaryDirectory() as temp:
            root=Path(temp)
            proc=root/'72'
            proc.mkdir()
            executable=root/'python3'
            executable.write_bytes(b'python executable')
            (proc/'exe').symlink_to(executable)
            script=root/'lifecycle.py'
            script.write_bytes(b'expected lifecycle')
            fields=['R','60']+['0']*17+['123']
            (proc/'stat').write_text('72 (python role) '+' '.join(fields))
            (proc/'cgroup').write_text('0::/scope\n')
            for argv in (['/python3','-B',str(script),'--controller'], ['/python3',str(script),'--driver']):
                (proc/'cmdline').write_bytes(('\0'.join(argv)+'\0').encode())
                value=a.python_role_snapshot(72, str(script), proc_root=root, clock=lambda:11.)
                self.assertEqual(value['argv'],argv)
                self.assertEqual(value['script_path'],str(script))
                self.assertEqual(value['script_sha256'],a.digest(script))
            for argv in (['/python3','-c',str(script)], ['/python3','-m',str(script)],
                         ['/env',str(script)], ['/python3',str(root/'unapproved.py')]):
                (proc/'cmdline').write_bytes(('\0'.join(argv)+'\0').encode())
                with self.subTest(argv=argv), self.assertRaises(ValueError):
                    a.python_role_snapshot(72, str(script), proc_root=root, clock=lambda:11.)

    def test_protected_script_is_refused_before_any_digest(self):
        with tempfile.TemporaryDirectory() as temp:
            root=Path(temp)
            proc=root/'72'
            proc.mkdir()
            executable=root/'python3'
            executable.write_bytes(b'fake executable')
            (proc/'exe').symlink_to(executable)
            fields=['R','60']+['0']*17+['123']
            (proc/'stat').write_text('72 (python role) '+' '.join(fields))
            (proc/'cgroup').write_text('0::/scope\n')
            for script in ('/secret/.env', '/secret/.env.local', '/secret/part.vault/actor.py'):
                argv=['/python3','-B',script,'--watch','/config','c'*64]
                (proc/'cmdline').write_bytes(('\0'.join(argv)+'\0').encode())
                with self.subTest(script=script), patch.object(a, 'digest') as spy:
                    with self.assertRaisesRegex(ValueError, 'protected'):
                        a.process_snapshot(72,proc_root=root,clock=lambda:11.)
                    spy.assert_not_called()
                    with self.assertRaisesRegex(ValueError, 'protected'):
                        a.python_role_snapshot(72,script,proc_root=root,clock=lambda:11.)
                    spy.assert_not_called()

    def test_basic_process_is_public_stable_and_live(self):
        sample=dict(pid=72,start='123',uid=1000,cgroup='/scope',state='R')
        with patch.object(a,'_basic_process',side_effect=[sample,dict(sample,state='S')]):
            value=a.basic_process(72,clock=lambda:11.)
        self.assertEqual(value['state'],'S')
        self.assertEqual(value['observed_monotonic'],11.)
        with patch.object(a,'_basic_process',side_effect=[sample,dict(sample,start='124')]):
            with self.assertRaises(ValueError):
                a.basic_process(72,clock=lambda:11.)


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


class BoundInputs(unittest.TestCase):
    def test_all_prepared_sources_verified_before_policy(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            actor = root/'actor.py'
            policy = root/'policy.py'
            dependency = root/'dependency.py'
            for file in (actor, policy, dependency):
                file.write_bytes(b'pinned source')
            prepared = root/'prepared.json'
            prepared.write_text(json.dumps(dict(pilot='pilot', files={p.name:a.digest(p) for p in (actor, policy, dependency)})))
            config = dict(pilot='pilot', prepared_path=str(prepared), prepared_sha256=a.digest(prepared),
                          actor=dict(script_path=str(actor),script_sha256=a.digest(actor)),
                          policy=dict(path=str(policy),sha256=a.digest(policy)))
            path = root/'config.json'
            path.write_text(json.dumps(config))
            sha = a.digest(path)
            a.verify_inputs(path, sha, config)
            dependency.write_bytes(b'changed')
            with self.assertRaisesRegex(ValueError, 'source changed'):
                a.verify_inputs(path, sha, config)
            with self.assertRaisesRegex(ValueError, 'digest changed'):
                a.verify_inputs(path, '0'*64, config)

    def test_exclusive_frame_cannot_overwrite_first_refusal(self):
        with tempfile.TemporaryDirectory() as temp:
            path = Path(temp)/'firstred.json'
            a.write_frame(path, dict(first='immutable'))
            with self.assertRaises(FileExistsError):
                a.write_frame(path, dict(second='forbidden'))
            self.assertEqual(json.loads(path.read_text()), dict(first='immutable'))
            self.assertEqual(len(list(Path(temp).iterdir())), 1)

    def test_last_guard_before_link_prevents_publication(self):
        with tempfile.TemporaryDirectory() as temp:
            path = Path(temp)/'armed.json'
            def refuse():
                raise ValueError('firstred appeared during staging')
            with self.assertRaisesRegex(ValueError, 'firstred'):
                a.write_frame(path, {}, guard=refuse)
            self.assertFalse(path.exists())
            self.assertEqual(list(Path(temp).iterdir()), [])

    def test_sanitized_scope_checks_both_processes_and_refusal(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            authority = root/'authority.json'
            uid = root.stat().st_uid
            public = dict(etat='actif',pid=60,debut='100',cgroup='/scope',echeance=900.,script_sha256='f'*64)
            authority.write_text(json.dumps(dict(public, secret_token='MUST_NOT_RETURN')))
            config = dict(authority=dict(path=str(authority), pid=60,start='100',uid=uid,cgroup='/scope',script_sha256='f'*64),
                          controller=dict(pid=61,start='101'), common_end_monotonic=900.,
                          paths=dict(first_red=str(root/'firstred.json')))
            def process(pid):
                return dict(pid=pid,start='100' if pid==60 else '101',state='S',uid=uid,cgroup='/scope')
            with patch.object(a.os, 'uname', return_value=SimpleNamespace(nodename='luciepc')), patch.object(a, '_basic_process', side_effect=process):
                self.assertIsNone(a.check_scope(config))
                config['controller']['start'] = 'wrong'
                with self.assertRaisesRegex(ValueError, 'process identity'):
                    a.check_scope(config)
                config['controller']['start'] = '101'
                Path(config['paths']['first_red']).write_text('{}')
                with self.assertRaisesRegex(ValueError, 'first red'):
                    a.check_scope(config)

    def test_ready_expired_or_other_authority_never_admitted(self):
        c, _, _, _ = fixture()
        c['paths'] = dict(first_red='/firstred')
        ready = dict(clock=a.CLOCK,nonce=c['ready_nonce'],deadline=900.,ready_observed_monotonic=11.,grant_deadline_monotonic=131.,
                     phase=dict(pilot=c['pilot'],allocation=c['allocation'],authority=c['authority'],first_red_path='/firstred'))
        a.validate_ready(c,ready,12.)
        with self.assertRaises(ValueError):
            a.validate_ready(c,ready,131.)
        wrong = copy.deepcopy(ready)
        wrong['phase']['authority']['start']='other'
        with self.assertRaises(ValueError):
            a.validate_ready(c,wrong,12.)


if __name__ == '__main__':
    unittest.main()
