#!/usr/bin/env python3
"""Run the agreed pinned qualification, preserving the first failure and logs."""
from pathlib import Path
import hashlib
import json
import os
import re
import shutil
import subprocess
import sys
import time

ROOT = Path.home() / '.cache/rag3weaver-build'
WORKTREE = Path.home() / 'git_workspaces/rag3db-cx-archi-pas-va-au-bout-r2'
CRATE = WORKTREE / 'extension/rag3weaver'
PROOFS = ROOT / 'cx-archi-pas-va-au-bout-r2-59ac-pass1'
TARGET = ROOT / 'target-cx-archi-pas-va-au-bout-r2'
PASS = ROOT / 'passe_epinglee.sh'
PASS_SHA = '2150fcd847b1df7de04908d357a0c78abc05e50ce599e2793ff08ab48108825d'
LIB = 'rag3db-build-59ac9c86bc2b3529b590a7462ab7051e3ad26b8a-Release-p12-v11-g17-s18-rcOFF-stOFF-asanOFF-tsanOFF-ubsanOFF'
BASE = '68590270e21d00285b29800bd9cdecf0cb921461'
RELATIVE = 'docs/rag3weaver-archi/16-11-2026/pas-va-au-bout/qualify.py'
FILTER = 'daemon_host::teammate_tests::a_step_that_answers_without_step_done_is_continued_without_waiting'


def digest(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def git(*args):
    return subprocess.check_output(['git', '-C', str(WORKTREE), *args], text=True).strip()


def guard(expected):
    assert git('rev-parse', 'HEAD') == expected
    assert not git('status', '--porcelain')
    assert digest(PASS) == PASS_SHA and PASS.stat().st_mode & 0o777 == 0o755
    assert digest(PASS) == digest(WORKTREE / 'tools/poste/passe_epinglee.sh')
    assert git('submodule', 'status').startswith('bd133850f89d6b12a3ca2b2f6ce238785ed37e3a')
    assert subprocess.check_output(['git', '-C', str(CRATE / 'codeparsers'), 'rev-parse', 'HEAD'], text=True).strip() == 'bd133850f89d6b12a3ca2b2f6ce238785ed37e3a'
    assert digest(__file__) == digest(WORKTREE / RELATIVE)


def inside(expected):
    guard(expected)
    subprocess.run(['python3', str(CRATE / 'scripts/confiner_validation.py'), 'verifier'], check=True)
    receipt = json.loads(Path(os.environ['RAG3WEAVER_VALIDATION_RECU']).read_text())
    env = dict(os.environ)
    env.pop('RAG3WEAVER_MOTEUR_ANCIEN', None)
    env.update(CIBLE=TARGET.name, SUITE_DU_TEMOIN='lib',
               SUITES='e2e_demon_hote,e2e_lectures,e2e_equipier,e2e_hooks_claude_code,e2e_code',
               GOOGLE_APPLICATION_CREDENTIALS='/home/lucied/git_workspaces/rag3db/.vault/vertex-sa.json',
               GOOGLE_CLOUD_PROJECT='lr-hub-472010', GOOGLE_CLOUD_LOCATION='global',
               RAG3WEAVER_EXIGER_GEMINI='1', RAG3WEAVER_EXIGER_NAVIGATEUR='1')
    with (PROOFS / 'pass-driver.log').open('w') as log:
        child = subprocess.Popen(['bash', str(PASS), WORKTREE.name, PROOFS.name, FILTER],
                                 cwd='/var/tmp', env=env, stdout=log, stderr=subprocess.STDOUT)
        while child.poll() is None:
            for path in PROOFS.glob('*.log'):
                text = path.read_text(errors='replace')
                failed = re.search(r'^test [^\n]* \.\.\. FAILED\b|^ROUGE\b|^CHECK_FEATURES ROUGE\b', text, re.M)
                if failed:
                    with (PROOFS / 'first-red.json').open('x') as record:
                        json.dump(dict(log=path.name, signal=failed.group(), observed=time.monotonic()), record)
                    child.terminate()
                    raise RuntimeError('first failure: ' + path.name)
            assert time.monotonic() < receipt['echeance'] - 5
            time.sleep(.2)
        assert child.wait() == 0
    assert (PROOFS / 'resume.log').read_text().splitlines()[-1] == 'VERT'
    guard(expected)


def main():
    expected = sys.argv[-1]
    assert re.fullmatch('[0-9a-f]{40}', expected)
    if sys.argv[1] == '--inside':
        inside(expected)
        return
    assert os.uname().nodename.lower() == 'luciepc'
    guard(expected)
    assert not PROOFS.exists() and not TARGET.exists()
    PROOFS.mkdir(mode=0o700)
    shutil.copy2(__file__, PROOFS / 'qualify.py')
    (PROOFS / 'base').write_text(BASE + '\n')
    library = ROOT.parent / 'rag3db-libs' / LIB / 'librag3db.so'
    (PROOFS / 'source.json').write_text(json.dumps(dict(source=expected, base=BASE, library=LIB,
        library_sha256=digest(library), launcher_sha256=digest(__file__), pass_sha256=PASS_SHA,
        target=str(TARGET), gitlink=git('ls-tree', 'HEAD', 'extension/rag3weaver/codeparsers'))))
    env = dict(os.environ, POSTE_MEM_MAX='32G', RAG3WEAVER_BUILD_MEMORY_HIGH='0', CARGO_BUILD_JOBS='8')
    env.pop('RAG3WEAVER_MOTEUR_ANCIEN', None)
    runtime = Path('/run/user') / str(os.getuid())
    bus = runtime / 'bus'
    assert runtime.resolve() == runtime and bus.resolve() == bus and bus.is_socket()
    assert runtime.stat().st_uid == bus.stat().st_uid == os.getuid()
    env.update(XDG_RUNTIME_DIR=str(runtime), DBUS_SESSION_BUS_ADDRESS='unix:path=' + str(bus))
    command = ['timeout', '--signal=TERM', '--kill-after=15s', '7200s', 'python3',
        str(CRATE / 'scripts/confiner_validation.py'), 'lancer', '--delai', '7100',
        '--preuves', str(PROOFS / 'supervision'), '--', 'python3', str(PROOFS / 'qualify.py'), '--inside', expected]
    with (PROOFS / 'supervision.log').open('w') as log:
        result = subprocess.run(command, cwd='/var/tmp', env=env, stdout=log, stderr=subprocess.STDOUT)
    verdicts = list((PROOFS / 'supervision').glob('validation-*/verdict.json'))
    assert len(verdicts) == 1
    verdict = json.loads(verdicts[0].read_text())
    assert result.returncode == 0 and verdict['verdict'] == 'VERT' and verdict['restes'] == [] and verdict['nettoye']
    guard(expected)
    print('VERT qualification; proofs:', PROOFS)


if __name__ == '__main__':
    main()
