"""Only AST-extracted pilot functions and temporary proof files. No subprocess."""
import ast
import hashlib
import json
from pathlib import Path
import signal
import tempfile
import time
import types
import unittest

SOURCE = Path(__file__).with_name('driver.py')


def load(names, namespace):
    tree = ast.parse(SOURCE.read_text())
    selected = [n for n in tree.body if isinstance(n, ast.FunctionDef) and n.name in names]
    assert len(selected) == len(names)
    exec(compile(ast.Module(body=selected, type_ignores=[]), str(SOURCE), 'exec'), namespace)


class Preparation(unittest.TestCase):
    def test_fresh_cache_check_precedes_the_actual_launch(self):
        with tempfile.TemporaryDirectory(prefix='cx-pilot-proof-') as d:
            events = []
            class Launched(Exception): pass
            def launch(*a, **kw):
                events.append('launch')
                raise Launched
            ns = dict(P=Path(d), C=Path(d), E={}, END=1000, time=types.SimpleNamespace(monotonic=lambda: 1),
                      guard=lambda: events.append('guard'), state=lambda *a: None,
                      require=lambda ok, reason: self.assertTrue(ok, reason),
                      fresh_cache_guard=lambda: events.append('fresh'),
                      subprocess=types.SimpleNamespace(Popen=launch, STDOUT=None))
            load({'run'}, ns)
            with self.assertRaises(Launched): ns['run']('cache-reprise', ['fixture-not-executed'], 120)
            self.assertEqual(events, ['guard', 'fresh', 'launch'])

    def test_old_empty_scan_cannot_hide_an_old_worktree_reader(self):
        ns = {}
        load({'in_cache_context'}, ns)
        old_snapshot = {'quiescence': []}
        self.assertFalse(old_snapshot['quiescence'])
        roots = [Path('/private/target'), Path('/private/old-worktree')]
        self.assertTrue(ns['in_cache_context']('/private/old-worktree/src', roots))
        self.assertTrue(ns['in_cache_context']('/private/target/debug/deps/lib.rlib', roots))
        self.assertFalse(ns['in_cache_context']('/private/target-other/debug', roots))

    def test_only_exact_ancestor_roles_can_ignore_control_arguments(self):
        ns = {}
        load({'controller_role', 'controller_fd_allowed', 'in_cache_context'}, ns)
        roles = {
            'launcher': dict(uid=1000, exe='/usr/bin/python', exe_sha256='python-sha', cwd='/var/tmp',
                             argv_tail=['/control/lancer.py', 'fixed-pilot'], script='/control/lancer.py'),
            'poste': dict(uid=1000, exe='/usr/bin/bash', exe_sha256='bash-sha', cwd='/var/tmp',
                          argv_tail=['/source/scripts/poste', 'lourd', 'reviewed-command'], script='/source/scripts/poste'),
        }
        ancestors = {10: dict(starttime='launch-start', ppid=1, cgroup='/caller'),
                     20: dict(starttime='poste-start', ppid=10, cgroup='/caller')}
        def info(pid, name):
            policy = roles[name]
            return dict(pid=pid, **ancestors[pid], uid=policy['uid'], exe=policy['exe'],
                        exe_sha256=policy['exe_sha256'], cwd=policy['cwd'], argv=['interpreter', *policy['argv_tail']])
        launcher, poste = info(10, 'launcher'), info(20, 'poste')
        self.assertEqual(ns['controller_role'](launcher, ancestors, roles), 'launcher')
        self.assertEqual(ns['controller_role'](poste, ancestors, roles), 'poste')
        # Same cgroup or ancestry alone is not an authorization.
        reader = dict(launcher, argv=['python', '/old/read.py'])
        self.assertIsNone(ns['controller_role'](reader, ancestors, roles))
        self.assertIsNone(ns['controller_role'](dict(launcher, pid=99), ancestors, roles))
        for change in [dict(starttime='reused'), dict(cgroup='/other'), dict(exe_sha256='other'), dict(uid=0)]:
            self.assertIsNone(ns['controller_role'](dict(launcher, **change), ancestors, roles))
        protected = [Path('/target'), Path('/old')]
        self.assertTrue(ns['controller_fd_allowed']('poste', '/source/scripts/poste', roles, protected))
        for role in ('poste', 'launcher'):
            self.assertFalse(ns['controller_fd_allowed'](role, '/target/debug/lib', roles, protected))
            self.assertFalse(ns['controller_fd_allowed'](role, '/old/Cargo.toml', roles, protected))
        self.assertFalse(ns['controller_fd_allowed']('launcher', '/control/lancer.py', roles, protected))
        self.assertFalse(ns['controller_fd_allowed']('poste', '/source/Cargo.toml', roles, protected))

    def test_bad_attempt_and_first_red_are_immutable(self):
        with tempfile.TemporaryDirectory(prefix='cx-pilot-proof-') as d:
            ns = dict(P=Path(d), attempts={}, phase='features', hashlib=hashlib, json=json, time=time)
            load({'record_attempt', 'latch_first_red', 'require'}, ns)
            good = dict(pid=12, actual={'SDK': 'expected'}, diff={})
            bad = dict(pid=12, actual={'SDK': None}, diff={'SDK': 'missing'})
            first = ns['record_attempt']('cargo', good)
            wrong = ns['record_attempt']('cargo', bad)
            self.assertNotEqual(first, wrong)
            with self.assertRaises(AssertionError): ns['require'](False, 'SDK missing', bad)
            frozen = (Path(d) / 'first-red.json').read_bytes()
            ns['latch_first_red']('cleanup now looks valid', good)
            self.assertEqual((Path(d) / 'first-red.json').read_bytes(), frozen)
            self.assertEqual(ns['record_attempt']('cargo', good), first)
            self.assertEqual(json.loads((Path(d) / wrong).read_text())['actual'], bad['actual'])

    def test_cleanup_mode_and_metadata_phase_are_explicit(self):
        calls = []
        ns = dict(observe=lambda **kw: calls.append(kw), owned={}, child=None, signal=signal,
                  time=types.SimpleNamespace(sleep=lambda _: None), save=lambda *a: calls.append('error'),
                  METADATA_BEFORE_EXPORT={'SDK': None}, CARGO_PROFILE={'SDK': 'exact'})
        load({'stop', 'cargo_expected_for'}, ns)
        ns['stop']()
        self.assertEqual(calls, [{'cleanup': True}])
        self.assertEqual(ns['cargo_expected_for']('cache-reprise'), {'SDK': None})
        for phase in ('features', 'large', 'navigateur'):
            self.assertEqual(ns['cargo_expected_for'](phase), {'SDK': 'exact'})


if __name__ == '__main__': unittest.main()
