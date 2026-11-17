"""Late-lock counterexample at the real admit.main final verdict, no subprocess.

Common/preflight/Git/process metadata are models. The exact published main,
its budget, manifest checks, hashes and final-accepted publication run on owned
TempDir files. No source rewriting, virtual clock, real proc, SSH or ELF.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import runpy
import subprocess
import sys
import tempfile
import time
import unittest
from unittest.mock import patch

SOURCE = '0f610f437a341f745e5936492114142ff3afedd4'
OPTIONS = None


def exercise(drift):
    source = Path(OPTIONS.admission).absolute().read_bytes()
    assert hashlib.sha256(source).hexdigest() == OPTIONS.admission_sha256
    proof = Path(OPTIONS.proof).absolute() / ('drift' if drift else 'unchanged')
    proof.mkdir(mode=0o700, parents=True, exist_ok=False)
    events = []
    with tempfile.TemporaryDirectory(prefix='lucivy-final-lock-entry-') as temp:
        root = Path(temp)
        package = root / 'package'; package.mkdir()
        worktree = root / 'source'; worktree.mkdir()
        subject = package / 'admit-fs-admission-red.py'; subject.write_bytes(source)
        pilot = package / 'qualify-fs-admission-red.py'; pilot.write_text('# modelled pilot, never executed\n')
        seed = package / 'Cargo.lock.seed'
        seed.write_text('version = 4\n[[package]]\nname = "fixture"\nversion = "1.0.0"\n')
        effective = worktree / 'Cargo.lock'; effective.write_bytes(seed.read_bytes())
        artifact = root / 'artifact.txt'; artifact.write_text('plain artifact identity fixture, not executable\n')
        scratch = package / 'scratch'; scratch.mkdir()
        scans = package / 'scans'; scans.mkdir()
        scope = package / 'scope'
        output = package / 'logs'
        nonce = '0123456789abcdef0123456789abcdef'
        hashes = {p.name: hashlib.sha256(p.read_bytes()).hexdigest() for p in [pilot, seed]}
        data = dict(package=str(package), source=SOURCE, nonce=nonce, scope=str(scope),
            output=str(output), worktree=str(worktree), scratch=str(scratch),
            baseline_unknown=[], copy_hashes=hashes)
        preflight = package / 'preflight.json'; preflight.write_text(json.dumps(data))
        (package / 'exporter-armed.json').write_text(json.dumps(dict(
            nonce=nonce, source=SOURCE, launch_by=time.monotonic()+30,
            physical_deadline=time.monotonic()+300)))
        model = dict(common_returned=False, source_checked=False, seed_rehashed_after_common=False)
        before_sha = hashlib.sha256(effective.read_bytes()).hexdigest()

        def sha(path):
            path = Path(path)
            if model['common_returned'] and path == seed:
                model['seed_rehashed_after_common'] = True
            if model['common_returned'] and path == effective:
                events.append('effective-lock-hashed-at-final')
            return hashlib.sha256(path.read_bytes()).hexdigest()

        def check_preflight(filename, scope_exists):
            assert filename == str(preflight) and scope_exists is False
            assert sha(effective) == sha(seed)
            events.append('preflight-lock-valid')
            return data

        def checked_path(path, kind='dir', **kwargs):
            path = Path(path)
            assert path.is_absolute() and path.is_relative_to(root) and not path.is_symlink()
            assert (path.is_file() if kind == 'file' else path.is_dir())
            return path

        def call_common(argv):
            assert argv[1] == str(package / 'support/confiner_validation.py')
            assert (scope / 'scope-provenance.json').is_file()
            child = scope / 'validation-fixture'; child.mkdir()
            (child / 'autorite.json').write_text(json.dumps(dict(pid=424242, debut='1',
                etat='termine', echeance=time.monotonic()+240, script_sha256='fixture-common')))
            (child / 'verdict.json').write_text(json.dumps(dict(code=0, erreur=None, restes=[], nettoye=True)))
            (child / 'enfants.json').write_text('[]')
            output.mkdir()
            (output / 'result.json').write_text(json.dumps(dict(fixture_only=True,
                final_artifacts=dict(fixture=dict(path=str(artifact), sha256=sha(artifact))))))
            events.append('compile-and-seven-judges-modelled-success')
            if drift:
                effective.write_bytes(seed.read_bytes().replace(b'1.0.0', b'2.0.0'))
                events.append('effective-lock-mutated-after-modelled-judges')
            model['common_returned'] = True
            return 0

        def git_read(argv, **kwargs):
            assert argv[:3] == ['git', '-C', str(worktree)] and kwargs.get('text') is True
            if argv[3:] == ['rev-parse', 'HEAD']:
                model['source_checked'] = True
                return SOURCE + '\n'
            assert argv[3:] == ['status', '--porcelain']
            return ''  # Models the ignored Cargo.lock; not a real Git invocation.

        def save_scan(data, records, assessment, phase):
            path = scans / 'modelled-final.json'
            path.write_text(json.dumps(dict(fixture_only=True, assessment=assessment)))
            return path

        functions = dict(check_preflight=check_preflight, sha=sha, checked_path=checked_path,
            SHARED=dict(collect_scan=lambda data: [], classify_scan=lambda records, baseline:
                dict(admitted=True, blocked=[], unknown=[]), save_scan=save_scan))
        real_run = runpy.run_path

        def dispatch(path, **kwargs):
            if Path(path) == pilot:
                return functions
            return real_run(path, **kwargs)

        original_open = Path.open

        def open_path(path, mode='r', *args, **kwargs):
            if path == Path('/proc/424242/stat'):
                raise FileNotFoundError('modelled supervisor absent; no proc read')
            return original_open(path, mode, *args, **kwargs)

        status, error = None, None
        cwd = Path.cwd()
        try:
            os.chdir(package)
            with patch.object(runpy, 'run_path', dispatch), \
                 patch.object(subprocess, 'call', call_common), \
                 patch.object(subprocess, 'check_output', git_read), \
                 patch.object(Path, 'open', open_path):
                try:
                    runpy.run_path(str(subject), run_name='__main__')
                except SystemExit as exit_status:
                    status = exit_status.code
                except AssertionError as assertion:
                    error = str(assertion)
        finally:
            os.chdir(cwd)
        seed_after = hashlib.sha256(seed.read_bytes()).hexdigest()
        effective_after = hashlib.sha256(effective.read_bytes()).hexdigest()
        result = dict(source_sha256=OPTIONS.admission_sha256, drift=drift, real_admission_main=True,
            common_preflight_git_process_models=True, compiler_or_judges_executed=False,
            events=events, model=model, source_status=status, source_error=error,
            initial_lock_valid=before_sha == hashes[seed.name],
            seed_intact=seed_after == hashes[seed.name], effective_lock_sha256=effective_after,
            expected_lock_sha256=hashes[seed.name],
            effective_lock_changed=effective_after != hashes[seed.name],
            final_accepted=(package / 'final-accepted.json').exists(),
            final_evidence_written=(package / 'final-evidence.json').exists(),
            source_rewritten=False, virtual_clock=False, real_proc=False,
            real_subprocess=False, real_cargo=False, real_elf=False, real_corpus=False)
        (proof / 'observation.json').write_text(json.dumps(result, indent=2))
    return result


class FinalLock(unittest.TestCase):
    def setup_reached(self, result):
        self.assertTrue(result['initial_lock_valid'])
        self.assertTrue(result['seed_intact'])
        self.assertTrue(all(result['model'].values()))
        self.assertTrue(result['final_evidence_written'])

    def test_late_lock_drift_is_refused_before_final_acceptance(self):
        result = exercise(True)
        self.setup_reached(result)
        self.assertTrue(result['effective_lock_changed'])
        self.assertFalse(result['final_accepted'],
            'real final verdict accepted a changed effective Cargo.lock after the modelled seven judges')
        self.assertIn('effective Cargo.lock', result['source_error'] or '')

    def test_unchanged_lock_can_reach_final_acceptance(self):
        result = exercise(False)
        self.setup_reached(result)
        self.assertFalse(result['effective_lock_changed'])
        self.assertEqual(result['source_status'], 0)
        self.assertIsNone(result['source_error'])
        self.assertTrue(result['final_accepted'])


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--admission', required=True)
    parser.add_argument('--admission-sha256', required=True)
    parser.add_argument('--proof', required=True)
    OPTIONS = parser.parse_args()
    unittest.main(argv=[sys.argv[0]], verbosity=2)
