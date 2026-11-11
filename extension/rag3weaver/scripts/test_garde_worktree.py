#!/usr/bin/env python3
"""Fixtures locales ; aucun dépôt réel ni accès réseau."""
import contextlib
import importlib.util
import io
from unittest.mock import patch
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest

SCRIPT = Path(__file__).with_name('garde_worktree.py')


def git(path, *args):
    env = dict(os.environ, GIT_CONFIG_NOSYSTEM='1', GIT_CONFIG_GLOBAL=os.devnull,
               GIT_OPTIONAL_LOCKS='0')
    return subprocess.check_output(['git', '-C', str(path), '-c', 'user.name=Témoin',
                                   '-c', 'user.email=temoin@example.invalid', *args],
                                  env=env, stderr=subprocess.PIPE).decode().strip()


def fingerprint(root):
    return {str(p.relative_to(root)): hashlib.sha256(p.read_bytes()).hexdigest()
            for p in root.rglob('*') if p.is_file() and not p.is_symlink()}


class GardeWorktreeTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory(prefix='garde-worktree-')
        self.addCleanup(self.tmp.cleanup)
        self.base = Path(self.tmp.name)
        self.repo = self.base / 'repo'
        self.repo.mkdir()
        git(self.repo, 'init', '-b', 'master')
        (self.repo / 'source').write_text('source parent\n')
        git(self.repo, 'add', 'source')
        git(self.repo, 'commit', '-m', 'source')
        self.parent = self.base / 'parent'
        git(self.repo, 'worktree', 'add', '-b', 'lot', str(self.parent))
        self.admin = Path(git(self.parent, 'rev-parse', '--absolute-git-dir'))
        self.report = self.base / 'rapport.json'

    def module(self):
        origin = self.base / 'module-origin'
        origin.mkdir()
        git(origin, 'init', '-b', 'master')
        (origin / 'source').write_text('source module\n')
        git(origin, 'add', 'source')
        git(origin, 'commit', '-m', 'module')
        git(self.parent, '-c', 'protocol.file.allow=always', 'submodule', 'add', str(origin), 'module')
        git(self.parent, 'commit', '-am', 'ajoute module')
        return self.parent / 'module'

    def archive(self):
        target = self.base / 'archive'
        shutil.copytree(self.admin, target, symlinks=True)
        return target

    def check(self, archives=(), report=None):
        args = [sys.executable, str(SCRIPT), '--depot', str(self.repo),
                '--worktree', str(self.parent), '--rapport', str(report or self.report)]
        for source, copy in archives:
            args += ['--archive', str(source), str(copy)]
        return subprocess.run(args, capture_output=True, text=True)

    def test_partage_refuse_meme_archive_et_ne_change_rien(self):
        module = self.module()
        other = self.base / 'autre'
        git(module, 'worktree', 'add', '-b', 'locale-privee', str(other))
        (other / 'source').write_text('modification privée\n')
        (other / 'non-suivi').write_text('donnée privée\n')
        admin = Path(git(other, 'rev-parse', '--absolute-git-dir'))
        before = (fingerprint(other), fingerprint(admin), fingerprint(self.admin),
                  git(other, 'show-ref'), git(other, 'rev-parse', 'HEAD'))
        archive = self.archive()
        result = self.check([(self.admin, archive)])
        self.assertEqual(result.returncode, 2, result.stderr)
        self.assertTrue(any('autre worktree' in x for x in json.loads(self.report.read_text())['refus']))
        after = (fingerprint(other), fingerprint(admin), fingerprint(self.admin),
                 git(other, 'show-ref'), git(other, 'rev-parse', 'HEAD'))
        self.assertEqual(before, after)

    def test_nominal_administration_archivee_fidelement(self):
        self.module()
        archive = self.archive()
        result = self.check([(self.admin, archive)])
        self.assertEqual(result.returncode, 0, result.stderr + result.stdout + self.report.read_text())
        self.assertEqual(json.loads(self.report.read_text())['verdict'], 'VERT')

    def test_reference_locale_non_preservee_refuse(self):
        module = self.module()
        archive = self.archive()
        git(module, 'branch', 'locale-apres-copie')
        result = self.check([(self.admin, archive)])
        self.assertEqual(result.returncode, 2, result.stderr)
        self.assertIn('diffère', ' '.join(json.loads(self.report.read_text())['refus']))

    def test_gitfile_casse_refuse(self):
        module = self.module()
        (module / '.git').write_text('gitdir: /absent/fixture-seulement\n')
        result = self.check([(self.admin, self.archive())])
        self.assertEqual(result.returncode, 2, result.stderr)

    def test_archive_absente_et_rapport_existant_refuses(self):
        self.assertEqual(self.check().returncode, 2)
        before = self.report.read_bytes()
        self.assertEqual(self.check().returncode, 1)
        self.assertEqual(before, self.report.read_bytes())

    def test_donnees_ignorees_non_archivees_refusees(self):
        (self.parent / '.gitignore').write_text('prive\n')
        git(self.parent, 'add', '.gitignore')
        git(self.parent, 'commit', '-m', 'ignore')
        (self.parent / 'prive').write_text('donnée ignorée\n')
        result = self.check([(self.admin, self.archive())])
        self.assertEqual(result.returncode, 2, result.stderr)
        self.assertTrue((self.parent / 'prive').exists())

    def test_rapport_dans_administration_refuse_sans_ecriture(self):
        report = self.admin / 'rapport.json'
        self.assertEqual(self.check(report=report).returncode, 1)
        self.assertFalse(report.exists())

    def protected_check(self, arguments):
        spec = importlib.util.spec_from_file_location('garde_fixture', SCRIPT)
        guard = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(guard)
        argv = [str(SCRIPT), '--depot', str(self.repo), '--worktree', str(self.parent),
                '--rapport', str(self.report)] + arguments
        with patch.object(sys, 'argv', argv), patch.object(guard, 'git', side_effect=AssertionError('Git inspecté trop tôt')), \
                patch.object(guard, 'inventory', side_effect=AssertionError('archive lue')), \
                contextlib.redirect_stderr(io.StringIO()):
            self.assertEqual(guard.main(), 1)
        self.assertFalse(self.report.exists())

    def test_archive_protegee_refusee_avant_lecture(self):
        for name in ('.vault', '.env'):
            copy = self.base / name / 'archive'
            shutil.copytree(self.admin, copy)
            self.protected_check(['--archive', str(self.admin), str(copy)])

    def test_archive_alias_canonique_protege_refuse(self):
        copy = self.base / '.vault' / 'archive'
        shutil.copytree(self.admin, copy)
        alias = self.base / 'alias'
        alias.symlink_to(copy, target_is_directory=True)
        self.protected_check(['--archive', str(self.admin), str(alias)])

    def test_composant_lexical_protege_meme_suivi_de_parent(self):
        (self.base / '.vault').mkdir()
        self.archive()
        copy = self.base / '.vault' / '..' / 'archive'
        self.protected_check(['--archive', str(self.admin), str(copy)])

    def test_rapport_protege_refuse_sans_inspection_ni_ecriture(self):
        for name in ('.vault', '.env'):
            directory = self.base / name
            directory.mkdir()
            self.protected_check(['--rapport', str(directory / 'rapport.json')])
            self.assertFalse((directory / 'rapport.json').exists())

    def test_depot_cible_et_administration_proteges_refuses(self):
        protected = self.base / '.vault' / 'inexistant'
        for option in ('--depot', '--worktree'):
            self.protected_check([option, str(protected)])
        self.protected_check(['--archive', str(protected), str(self.base / 'copie')])

    def test_zone_env_du_worktree_refusee_avant_parcours_et_status(self):
        directory = self.parent / '.env'
        directory.mkdir()
        (directory / 'fixture').write_text('contenu de fixture à ne pas lire\n')
        spec = importlib.util.spec_from_file_location('garde_fixture', SCRIPT)
        guard = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(guard)
        original_git, original_scan = guard.git, os.scandir

        def checked_git(path, *args, **kwargs):
            self.assertNotIn('status', args)
            return original_git(path, *args, **kwargs)

        def checked_scan(path):
            self.assertNotIn('.env', Path(path).parts)
            return original_scan(path)

        argv = [str(SCRIPT), '--depot', str(self.repo), '--worktree', str(self.parent),
                '--rapport', str(self.report)]
        with patch.object(sys, 'argv', argv), patch.object(guard, 'git', side_effect=checked_git), \
                patch.object(guard.os, 'scandir', side_effect=checked_scan), \
                patch.object(guard, 'inventory', side_effect=AssertionError('archive lue')), \
                contextlib.redirect_stdout(io.StringIO()):
            self.assertEqual(guard.main(), 2)

    def test_rapport_dans_cible_refuse_sans_ecriture(self):
        report = self.parent / 'rapport.json'
        self.assertEqual(self.check(report=report).returncode, 1)
        self.assertFalse(report.exists())


if __name__ == '__main__':
    unittest.main()
