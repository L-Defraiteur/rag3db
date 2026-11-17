"""Run only after a distinct GO: fresh admission, Common, bounded final evidence."""
import hashlib
import json
import math
import os
from pathlib import Path
import runpy
import stat
import subprocess
import sys
import time


def main():
    start = time.monotonic()  # After actual poste, before metadata preflight.
    deadline = start + 285
    package = Path(__file__).absolute().parent
    for location in [package, Path(__file__).absolute(), package / 'preflight.json',
                     package / 'qualify-fs-admission-red.py']:
        assert location.is_absolute() and '..' not in location.parts
        current = Path('/')
        for part in location.parts[1:]:
            current /= part
            assert not stat.S_ISLNK(current.lstat().st_mode), f'link refused: {current}'
        assert location.stat().st_uid == os.getuid()
    assert Path.cwd() == package
    raw_preflight = (package / 'preflight.json').read_bytes()
    manifest = json.loads(raw_preflight)
    armed = json.loads((package / 'exporter-armed.json').read_text())
    assert armed['nonce'] == manifest['nonce'] and armed['source'] == manifest['source']
    assert start < armed['launch_by'], 'exporter was not armed immediately before launch'
    deadline = min(deadline, armed['physical_deadline'] - 5)
    pilot = package / 'qualify-fs-admission-red.py'
    assert hashlib.sha256(pilot.read_bytes()).hexdigest() == manifest['copy_hashes'][pilot.name]
    functions = runpy.run_path(str(pilot), run_name='reviewed_checks_only')
    data = functions['check_preflight'](str(package / 'preflight.json'), scope_exists=False)
    scope = Path(data['scope'])
    scope.mkdir(exist_ok=False)
    bound = dict(nonce=data['nonce'], start=start, deadline=deadline, uid=os.getuid(),
                 reserve_external_final_seconds=10)

    def publish(path, payload):
        assert time.monotonic() < deadline
        with path.open('x') as file:
            assert time.monotonic() < deadline
            json.dump(payload, file, indent=2)
            file.flush(); os.fsync(file.fileno())
        assert time.monotonic() < deadline

    publish(scope / 'scope-provenance.json', bound)
    # A single START/END; Common gets the remaining budget minus final reserve.
    remaining = math.floor(deadline - time.monotonic()) - 12
    assert remaining > 15
    argv = [sys.executable, str(package / 'support/confiner_validation.py'), 'lancer',
            '--delai', str(remaining), '--preuves', str(scope), '--', sys.executable,
            str(pilot), '--preflight', str(package / 'preflight.json')]
    code = subprocess.call(argv)
    # The Common wrapper has actually returned. No new scope or deadline.
    authorities = list(scope.glob('validation-*/autorite.json'))
    assert len(authorities) == 1
    authority = json.loads(authorities[0].read_text())
    verdict = json.loads(authorities[0].with_name('verdict.json').read_text())
    children = authorities[0].with_name('enfants.json')
    final_data = dict(data, old_children=str(children))
    shared = functions['SHARED']
    records = shared['collect_scan'](final_data)
    assessment = shared['classify_scan'](records, data['baseline_unknown'])
    snapshot = shared['save_scan'](final_data, records, assessment, 'external-final')
    old_pid = int(authority['pid'])
    owned_common_present = False
    try:
        fields = Path(f'/proc/{old_pid}/stat').read_text().rsplit(') ', 1)[1].split()
        owned_common_present = fields[19] == authority['debut'] and fields[0] != 'Z'
    except FileNotFoundError:
        pass
    payload = dict(source=data['source'], nonce=data['nonce'], start=start, deadline=deadline,
        observed=time.monotonic(), wrapper_code=code, verdict=verdict,
        authority=dict(pid=old_pid, start=authority['debut'], state=authority['etat'],
                       deadline=authority['echeance'], script_sha256=authority['script_sha256']),
        common_owned_present=owned_common_present, scan=str(snapshot), assessment=assessment,
        preflight_sha256=hashlib.sha256(raw_preflight).hexdigest(),
        universal_quiescence_claimed=False)
    # Preserve final evidence before asserting a failure. Never publish the capability token.
    publish(package / 'final-evidence.json', payload)
    assert authority['etat'] == 'termine' and not owned_common_present
    assert verdict['erreur'] is None and not verdict['restes'] and verdict['nettoye']
    assert assessment['admitted']
    assert (package / 'preflight.json').read_bytes() == raw_preflight
    if code:
        return code if code > 0 else 1
    assert verdict['code'] == 0
    result = json.loads((Path(data['output']) / 'result.json').read_text())
    for row in result['final_artifacts'].values():
        assert time.monotonic() < deadline
        assert functions['sha'](functions['checked_path'](row['path'], kind='file')) == row['sha256']
    assert subprocess.check_output(['git', '-C', data['worktree'], 'rev-parse', 'HEAD'], text=True).strip() == data['source']
    assert not subprocess.check_output(['git', '-C', data['worktree'], 'status', '--porcelain'], text=True).strip()
    for relative, digest in data['copy_hashes'].items():
        assert time.monotonic() < deadline
        assert functions['sha'](package / relative) == digest
    assert not list(Path(data['scratch']).iterdir())
    assert time.monotonic() < deadline
    effective_lock = functions['checked_path'](str(Path(data['worktree']) / 'Cargo.lock'), kind='file')
    effective_lock_sha256 = functions['sha'](effective_lock)
    assert effective_lock_sha256 == data['copy_hashes']['Cargo.lock.seed'], \
        'effective Cargo.lock changed before final acceptance'
    assert time.monotonic() < deadline
    publish(package / 'final-accepted.json', dict(effective_lock_sha256=effective_lock_sha256,
        lock_invariant=True, source=data['source'], nonce=data['nonce'],
        observed=time.monotonic(), deadline=deadline, semantic_reds=5, positive=1,
        physical_elf_rehash=True, frozen_copies_rechecked=True, actual_poste_return_pending=True))
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
