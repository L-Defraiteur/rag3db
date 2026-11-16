#!/usr/bin/env python3
"""Témoins CPU du vrai mode --pile, avec padding produit par Git."""
import os
from pathlib import Path
import re
import shutil
import subprocess
import tempfile
import unittest

GUARD = Path(__file__).with_name('push_garde.sh').resolve()
GIT = shutil.which('git')


class StackHeaders(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)
        self.env = dict(os.environ, GIT_CONFIG_NOSYSTEM='1',
                        GIT_CONFIG_GLOBAL='/dev/null', GIT_AUTHOR_NAME='fixture',
                        GIT_AUTHOR_EMAIL='fixture@example.invalid',
                        GIT_COMMITTER_NAME='fixture',
                        GIT_COMMITTER_EMAIL='fixture@example.invalid')
        self.git('init', '-q')
        self.commit('base.md', 'base', 'base')
        self.before = self.git('rev-parse', 'HEAD')
        self.commit('extension/rag3weaver/src/header.rs', 'fn tested() {}', 'code')
        self.code = self.git('rev-parse', 'HEAD')
        for i in range(13):
            self.commit(f'docs/{i}.md', str(i), f'doc {i}')
        self.tested = self.git('rev-parse', 'HEAD')

    def git(self, *args):
        return subprocess.check_output([GIT, *args], cwd=self.root,
                                       env=self.env, text=True).strip()

    def commit(self, path, content, subject):
        file = self.root / path
        file.parent.mkdir(parents=True, exist_ok=True)
        file.write_text(content + '\n')
        self.git('add', path)
        self.git('commit', '-qm', subject)

    def judge(self, target, expected_rc, message, fake_range=None):
        env = dict(self.env)
        if fake_range is not None:
            bin_dir = self.root / 'bin'
            bin_dir.mkdir()
            wrapper = bin_dir / 'git'
            import shlex
            wrapper.write_text('#!/bin/bash\nif [ "$1" = range-diff ]; then\n'
                               + fake_range + '\nfi\nexec '
                               + shlex.quote(GIT) + ' "$@"\n')
            wrapper.chmod(0o755)
            env['PATH'] = str(bin_dir) + os.pathsep + env['PATH']
        else:
            raw = subprocess.check_output([GIT, 'range-diff', '--no-color',
                  '--creation-factor=100', f'{self.before}..{self.tested}',
                  f'{self.before}..{target}'], cwd=self.root, env=env, text=True)
            self.assertRegex(raw, r'(?m)^ +1:')
            self.assertGreaterEqual(len(re.findall(r'(?m)^\s*\d+:', raw)), 14)
        result = subprocess.run(['bash', str(GUARD), '--pile', self.before,
                                 self.tested, self.before, target],
                                cwd=self.root, env=env, text=True,
                                stdout=subprocess.PIPE, stderr=subprocess.STDOUT,
                                timeout=10)
        self.assertEqual(result.returncode, expected_rc, result.stdout)
        self.assertIn(message, result.stdout)
        if not expected_rc:
            self.assertEqual(result.stdout.count('  = égal :'), 14, result.stdout)
        if expected_rc:
            self.assertNotIn('pile tenue :', result.stdout)

    def test_padded_equal(self):
        self.judge(self.tested, 0, '  = égal :')

    def test_padded_added_code(self):
        self.commit('extension/rag3weaver/src/extra.rs', 'fn extra() {}', 'extra')
        self.judge('HEAD', 1, 'un commit ajouté après le rejeu touche autre chose')

    def test_padded_missing(self):
        self.git('checkout', '-q', '--detach', self.before)
        self.git('cherry-pick', f'{self.code}..{self.tested}')
        self.judge('HEAD', 1, "son patch n'est pas sur master")

    def test_padded_changed(self):
        self.git('checkout', '-q', '--detach', self.before)
        self.commit('extension/rag3weaver/src/header.rs', 'fn tested() { changed(); }', 'code modifié')
        self.git('cherry-pick', f'{self.code}..{self.tested}')
        self.judge('HEAD', 1, 'a changé hors des .md')

    def test_no_headers(self):
        self.judge(self.tested, 1, 'aucun en-tête', 'exit 0')

    def test_malformed_header(self):
        self.judge(self.tested, 1, 'en-tête range-diff invalide',
                   "printf '  1: not-a-hash = 1: also-invalid subject\\n'; exit 0")

    def test_range_diff_failure(self):
        self.judge(self.tested, 1, 'range-diff a échoué', 'exit 42')


if __name__ == '__main__':
    unittest.main()
