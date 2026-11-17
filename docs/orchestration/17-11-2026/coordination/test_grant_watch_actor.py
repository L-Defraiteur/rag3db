"""Hermetic admission/loop witnesses: no real child, /proc or network."""
import copy
import hashlib
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest

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


if __name__ == '__main__':
    unittest.main()
