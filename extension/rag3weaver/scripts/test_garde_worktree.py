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

    def check(self, archives=(), report=None, cwd=None):
        args = [sys.executable, str(SCRIPT), '--depot', str(self.repo),
                '--worktree', str(self.parent), '--rapport', str(report or self.report)]
        for source, copy in archives:
            args += ['--archive', str(source), str(copy)]
        return subprocess.run(args, capture_output=True, text=True, cwd=cwd)

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

    def test_desinitialise_n_attribue_pas_le_cwd_et_ne_change_rien(self):
        module = self.module()
        module_admin = Path(git(module, 'rev-parse', '--absolute-git-dir'))
        git(self.parent, 'submodule', 'deinit', '--all')
        archive = self.archive()
        before = (fingerprint(self.parent), fingerprint(self.admin))
        observations = []
        for name in ('cwd-exterieur-a', 'cwd-exterieur-b'):
            cwd = self.base / name
            cwd.mkdir()
            raw = subprocess.run(['git', '--git-dir', str(module_admin), '-c', 'gc.auto=0', 'worktree', 'list', '--porcelain', '-z'], cwd=cwd, env=dict(os.environ, GIT_OPTIONAL_LOCKS='0'), capture_output=True, text=True, check=True)
            report = self.base / (name + '.json')
            result = self.check([(self.admin, archive)], report=report, cwd=cwd)
            observations.append(dict(cwd=str(cwd), git_brut=raw.stdout, code=result.returncode,
                                     rapport=json.loads(report.read_text())))
        after = (fingerprint(self.parent), fingerprint(self.admin))
        preuves = os.environ.get('GARDE_WORKTREE_TEST_PREUVES')
        if preuves:
            root = Path(preuves)
            root.mkdir(parents=True, exist_ok=True)
            (root/'deinit-cwd.json').write_text(json.dumps(dict(observations=observations, sources_inchangees=before == after), ensure_ascii=False, indent=2))
        self.assertEqual(before, after)
        for row in observations:
            self.assertEqual(row['code'], 0, json.dumps(row, ensure_ascii=False))
            self.assertEqual(row['rapport']['verdict'], 'VERT')
            self.assertNotIn(row['cwd'], json.dumps(row['rapport'], ensure_ascii=False))

    def test_nominal_administration_archivee_fidelement(self):
        self.module()
        archive = self.archive()
        result = self.check([(self.admin, archive)])
        self.assertEqual(result.returncode, 0, result.stderr + result.stdout + self.report.read_text())
        self.assertEqual(json.loads(self.report.read_text())['verdict'], 'VERT')

    def test_desinitialise_partage_refuse_et_preserve_le_worktree_lie(self):
        module = self.module()
        other = self.base / 'lie-externe'
        git(module, 'worktree', 'add', '-b', 'locale-privee', str(other))
        git(self.parent, 'submodule', 'deinit', '--all')
        (other/'source').write_text('modification privée liée\n')
        before = (fingerprint(other), fingerprint(self.admin))
        result = self.check([(self.admin, self.archive())])
        self.assertEqual(result.returncode, 2, result.stderr)
        self.assertIn('autre worktree', ' '.join(json.loads(self.report.read_text())['refus']))
        self.assertEqual(before, (fingerprint(other), fingerprint(self.admin)))

    def test_desinitialise_avec_contenu_refuse_meme_archive(self):
        module = self.module()
        git(self.parent, 'submodule', 'deinit', '--all')
        (module/'prive').write_text('donnée non effaçable\n')
        before = fingerprint(self.parent)
        result = self.check([(self.admin, self.archive())])
        self.assertEqual(result.returncode, 2, result.stderr)
        self.assertIn('non vide', ' '.join(json.loads(self.report.read_text())['refus']))
        self.assertEqual(before, fingerprint(self.parent))

    def test_desinitialise_sans_gitlink_refuse(self):
        self.module()
        git(self.parent, 'submodule', 'deinit', '--all')
        git(self.parent, 'rm', '--cached', 'module')
        git(self.parent, 'commit', '-m', 'retire le gitlink, conserve le résidu')
        result = self.check([(self.admin, self.archive())])
        self.assertEqual(result.returncode, 2, result.stderr)
        self.assertIn('gitlink', ' '.join(json.loads(self.report.read_text())['refus']))

    def test_desinitialise_archive_perimee_refuse(self):
        module = self.module()
        admin = git(module, 'rev-parse', '--absolute-git-dir')
        git(self.parent, 'submodule', 'deinit', '--all')
        archive = self.archive()
        git(self.parent, '--git-dir', admin, 'branch', 'locale-apres-copie')
        result = self.check([(self.admin, archive)])
        self.assertEqual(result.returncode, 2, result.stderr)
        self.assertIn('diffère', ' '.join(json.loads(self.report.read_text())['refus']))

    def test_configure_sans_gitfile_ne_devient_pas_desinitialise(self):
        module = self.module()
        (module/'.git').unlink()
        result = self.check([(self.admin, self.archive())])
        self.assertEqual(result.returncode, 2, result.stderr)
        self.assertIn('ancrage', ' '.join(json.loads(self.report.read_text())['refus']))

    def test_worktree_lie_verrouille_refuse(self):
        git(self.repo, 'worktree', 'lock', str(self.parent))
        result = self.check([(self.admin, self.archive())])
        self.assertEqual(result.returncode, 2, result.stderr)
        self.assertIn('verrouillée', ' '.join(json.loads(self.report.read_text())['refus']))

    def test_inscription_liee_sans_gitfile_refuse(self):
        module = self.module()
        other = self.base/'lie-manquant'
        git(module, 'worktree', 'add', '-b', 'lie', str(other))
        (other/'.git').unlink()
        result = self.check([(self.admin, self.archive())])
        self.assertEqual(result.returncode, 2, result.stderr)

    def test_administration_inconnue_sans_core_worktree_refuse(self):
        admin = self.parent/'admin-inconnue'
        external = self.base/'source-inconnue'
        external.mkdir()
        git(external, 'init', '-b', 'master', '--separate-git-dir', str(admin))
        (external/'source').write_text('source ailleurs\n')
        git(external, 'add', 'source')
        git(external, 'commit', '-m', 'source')
        git(self.parent, 'add', 'admin-inconnue')
        git(self.parent, 'commit', '-m', 'administration sans identité de source')
        saved = self.base/'archive-inconnue'
        shutil.copytree(admin, saved, symlinks=True)
        result = self.check([(self.admin, self.archive()), (admin, saved)])
        self.assertEqual(result.returncode, 2, result.stderr)
        self.assertIn('source principale attestée', ' '.join(json.loads(self.report.read_text())['refus']))

    def test_ancrage_ne_demande_jamais_de_checkout_a_git_dir(self):
        self.module()
        spec = importlib.util.spec_from_file_location('garde_fixture_ancrage', SCRIPT)
        guard = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(guard)
        original = guard.git
        def checked_git(path, *args, **kwargs):
            if kwargs.get('admin'):
                self.assertNotIn('worktree', args)
                self.assertNotIn('--show-toplevel', args)
            return original(path, *args, **kwargs)
        for i in range(2):
            if i:
                git(self.parent, 'submodule', 'deinit', '--all')
            saved = self.base/('archive-ancrage-'+str(i))
            shutil.copytree(self.admin, saved, symlinks=True)
            report = self.base/('rapport-ancrage-'+str(i)+'.json')
            with patch.object(guard, 'git', side_effect=checked_git):
                result = guard.inspect(self.repo, self.parent, {self.admin: saved}, report)
            self.assertEqual(result['verdict'], 'VERT', result['refus'])

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

    def test_lien_source_suivi_vers_parent_est_feuille_sans_visite(self):
        link = self.parent / 'retour-parent'
        link.symlink_to('..', target_is_directory=True)
        git(self.parent, 'add', 'retour-parent')
        git(self.parent, 'commit', '-m', 'lien source')
        archive = self.archive()
        before = fingerprint(self.repo)
        spec = importlib.util.spec_from_file_location('garde_fixture', SCRIPT)
        guard = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(guard)
        original = os.scandir

        def checked_scan(path):
            self.assertNotIn('retour-parent', Path(path).parts)
            return original(path)

        argv = [str(SCRIPT), '--depot', str(self.repo), '--worktree', str(self.parent),
                '--archive', str(self.admin), str(archive), '--rapport', str(self.report)]
        with patch.object(sys, 'argv', argv), patch.object(guard.os, 'scandir', side_effect=checked_scan), \
                contextlib.redirect_stdout(io.StringIO()):
            self.assertEqual(guard.main(), 0)
        self.assertEqual(before, fingerprint(self.repo))
        entry = json.loads(self.report.read_text())['liens_sources'][0]
        self.assertEqual(entry['readlink'], '..')
        self.assertEqual(entry['index_mode'], '120000')

    def test_lien_source_modifie_refuse(self):
        link = self.parent / 'retour-parent'
        link.symlink_to('..', target_is_directory=True)
        git(self.parent, 'add', 'retour-parent')
        git(self.parent, 'commit', '-m', 'lien source')
        link.unlink()
        link.symlink_to('../repo', target_is_directory=True)
        self.assertEqual(self.check([(self.admin, self.archive())]).returncode, 2)

    def test_lien_source_non_suivi_refuse(self):
        (self.parent / 'non-suivi').symlink_to('..', target_is_directory=True)
        self.assertEqual(self.check([(self.admin, self.archive())]).returncode, 2)

    def test_administration_separee_sous_sources_partagee_refuse(self):
        admin = self.parent / 'donnees'
        external = self.base / 'externe'
        external.mkdir()
        git(external, 'init', '-b', 'master', '--separate-git-dir', str(admin))
        git(external, 'config', 'core.worktree', str(external))
        (external / 'source').write_text('source externe\n')
        git(external, 'add', 'source')
        git(external, 'commit', '-m', 'externe')
        git(external, 'worktree', 'add', '-b', 'autre', str(self.base / 'autre-externe'))
        git(self.parent, 'add', 'donnees')
        git(self.parent, 'commit', '-m', 'administration sans nom git dans source')
        self.assertEqual(git(self.parent, 'status', '--porcelain', '--ignored=matching'), '')
        before = fingerprint(admin)
        result = self.check([(self.admin, self.archive())])
        self.assertEqual(result.returncode, 2, result.stderr + result.stdout)
        self.assertTrue(any('autre worktree' in x for x in json.loads(self.report.read_text())['refus']))
        self.assertEqual(before, fingerprint(admin))
        self.report.rename(self.base / 'rapport-sans-archive-separee.json')
        saved = self.base / 'archive-separee'
        shutil.copytree(admin, saved)
        result = self.check([(self.admin, self.base / 'archive'), (admin, saved)])
        self.assertEqual(result.returncode, 2, result.stderr)
        self.assertTrue(any('autre worktree' in x for x in json.loads(self.report.read_text())['refus']))
        self.assertEqual(before, fingerprint(admin))

    def test_racines_liees_non_protegees_refusees(self):
        archive = self.archive()
        for name, source, option in [('depot', self.repo, '--depot'),
                                     ('cible', self.parent, '--worktree')]:
            alias = self.base / ('alias-' + name)
            alias.symlink_to(source, target_is_directory=True)
            self.protected_check([option, str(alias)])
        alias = self.base / 'alias-archive'
        alias.symlink_to(archive, target_is_directory=True)
        self.protected_check(['--archive', str(self.admin), str(alias)])

    def test_rapport_dans_cible_refuse_sans_ecriture(self):
        report = self.parent / 'rapport.json'
        self.assertEqual(self.check(report=report).returncode, 1)
        self.assertFalse(report.exists())


if __name__ == '__main__':
    unittest.main(failfast=True)
