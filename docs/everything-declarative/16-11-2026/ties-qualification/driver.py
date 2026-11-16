#!/usr/bin/env python3
"""One receipt, two distinct tie witnesses, then the official pass."""
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import sys
import time

Q = Path(__file__).parent
CONFIG = json.loads((Q / 'prepared.json').read_text())
W = Path(CONFIG['worktree'])
C = W / 'extension/rag3weaver'
T = Path(CONFIG['target'])
P = Path(CONFIG['proofs'])
R = P.parents[1]
RECEIPT = Path(os.environ['RAG3WEAVER_VALIDATION_RECU'])
AUTH = json.loads(RECEIPT.read_text())
END = AUTH['echeance']
NAMES = ['fts_handle::tests::a_search_returns_every_tie_of_its_cut',
         'backend::tests::ties_keep_the_same_order_from_a_rewritten_base_to_a_fresh_one']
FAILED = re.compile(r'^test .* \.\.\. FAILED$|^\s*FAILED\s*$|^test result: FAILED\.', re.M)

def digest(path):
    with Path(path).open('rb') as f:
        return hashlib.file_digest(f, 'sha256').hexdigest()

def check():
    assert time.monotonic() < END - 10
    assert subprocess.check_output(['git', '-C', str(W), 'rev-parse', 'HEAD'], text=True).strip() == CONFIG['app']
    assert not subprocess.check_output(['git', '-C', str(W), 'status', '--porcelain'], text=True).strip()
    assert digest(R / 'passe_epinglee.sh') == CONFIG['pass_sha256']
    assert all(digest(Q / n) == sha for n, sha in CONFIG['controllers'].items())

def first_red(reason):
    try:
        with (P / 'first-red.json').open('x') as f:
            json.dump(dict(reason=reason, observed=time.monotonic()), f)
            f.flush(); os.fsync(f.fileno())
    except FileExistsError:
        pass

def library_environment():
    # Independent of the deliberately private XDG cache.
    return dict(RAG3DB_LIB=CONFIG['lib'], RAG3DB_LIBS_DIR=CONFIG['library_store'],
                RAG3DB_BUFFER_POOL_SIZE='1073741824')

def run(name, args, env):
    check()
    with (P / (name + '.log')).open('w') as output:
        child = subprocess.Popen(args, cwd=C, env=env, stdout=output, stderr=subprocess.STDOUT)
        try:
            while True:
                assert time.monotonic() < END - 10
                for log in P.rglob('*.log'):
                    text = log.read_text(errors='replace')
                    if FAILED.search(text) or (log.name == 'check_features.log' and re.search(r'^(?:CHECK_FEATURES )?ROUGE\b', text, re.M)):
                        raise AssertionError('first red in ' + str(log))
                if child.poll() is not None:
                    break
                time.sleep(.1)
            assert child.wait() == 0, (name, child.returncode)
        except BaseException as error:
            first_red(repr(error))
            if child.poll() is None:
                child.terminate()
            # All descendants and escapees are cleaned by the common supervisor.
            raise
    check()
    return (P / (name + '.log')).read_text(errors='replace')

def main():
    P.mkdir(parents=True, exist_ok=False)
    assert AUTH['etat'] == 'actif'
    remaining = json.loads((Q / 'remaining-after-poste.json').read_text())
    assert 60 < json.loads((RECEIPT.parent / 'recette.json').read_text())['delai'] == remaining['duration']
    assert END <= CONFIG['original_end'] and remaining['original_end'] == CONFIG['original_end']
    assert remaining['original_start'] == CONFIG['original_start']
    cg = Path('/proc/self/cgroup').read_text().strip().split('::', 1)[1]
    group = Path('/sys/fs/cgroup' + cg)
    assert (group / 'memory.max').read_text().strip() == '34359738368'
    assert (group / 'memory.high').read_text().strip() == 'max'
    (P / 'authority-path.json').write_text(json.dumps(dict(path=str(RECEIPT), deadline=END)))
    env = {k: os.environ[k] for k in ['HOME', 'USER', 'LOGNAME', 'PATH', 'SHELL', 'LANG',
           'LC_ALL', 'TZ', 'CARGO_HOME', 'RUSTUP_HOME', 'DBUS_SESSION_BUS_ADDRESS',
           'RAG3WEAVER_VALIDATION_RECU', 'RAG3WEAVER_VALIDATION_JETON'] if k in os.environ}
    for name in ['tmp', 'config', 'data', 'cache', 'xdg-state', 'state-fixture']:
        (P / name).mkdir()
    env.update(CARGO_TARGET_DIR=str(T), CARGO_BUILD_JOBS='8', CARGO_INCREMENTAL='0',
               CARGO_TERM_COLOR='never', RUST_TEST_THREADS='8', RUST_BACKTRACE='1',
               CIBLE=T.name, POSTE_MEM_MAX='32G',
               RAG3WEAVER_BUILD_MEMORY_HIGH='0', RAG3WEAVER_TARGET_MAX_GO='32',
               RAG3WEAVER_PAQUETS_REACT=str(R / 'I-paquets-react'),
               RAG3WEAVER_EXIGER_NAVIGATEUR='1', TMPDIR=str(P / 'tmp'),
               XDG_CONFIG_HOME=str(P / 'config'), XDG_DATA_HOME=str(P / 'data'),
               XDG_CACHE_HOME=str(P / 'cache'), XDG_STATE_HOME=str(P / 'xdg-state'),
               RAG3WEAVER_STATE_DIR=str(P / 'state-fixture'))
    env.update(library_environment())
    for suffix in ['', '_EMBEDDER', '_RERANKER', '_OCR']:
        env['RAG3WEAVER_BURN_DEVICE' + suffix] = 'cpu'
    (P / 'configuration.json').write_text(json.dumps(dict(CONFIG, cgroup=cg,
        environment_keys=sorted(env), initial=CONFIG['original_start'], deadline=END,
        selected_library_environment=library_environment(), distinct_witnesses=NAMES), indent=2))
    check()
    assert T.is_dir() and T.resolve() == T, 'only the known same-owner v1 target'
    assert (T / '.arbre-du-target').read_text().strip() == str(C)
    assert not any(T.rglob('.fingerprint'))
    run('target-guard', ['bash', '-c', '. "$1"; garder_le_target "$2" "$3"',
        'initial-empty-target', str(C / 'scripts/garde_target.sh'), str(T), str(C)], env)
    assert (T / '.arbre-du-target').read_text().strip() == str(C)
    assert not any(T.rglob('.fingerprint'))
    for index, name in enumerate(NAMES):
        text = run('exact-' + str(index), ['bash', './run_e2e.sh', '--features',
            'openai-llm,anthropic-llm', '--lib', name, '--exact'], env)
        summaries = re.findall(r'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;', text)
        assert summaries and summaries[-1] == ('1', '0', '0')
    env.update(SUITE_DU_TEMOIN='lib', SUITES='e2e_code,e2e_lectures')
    run('official-pass', ['bash', str(R / 'passe_epinglee.sh'), W.name,
        str(P.relative_to(R) / 'official'), NAMES[1]], env)
    summary = P / 'official/resume.log'
    assert summary.read_text().splitlines()[-1] == 'VERT'
    assert not (P / 'first-red.json').exists()
    for name, sha in CONFIG['lib_hashes'].items():
        assert digest(R.parent / 'rag3db-libs' / CONFIG['lib'] / name) == sha
    check()
    verdict = dict(verdict='VERT', source=CONFIG['app'], deadline=END,
                   elapsed=time.monotonic() - CONFIG['original_start'], distinct_witnesses=2,
                   targeted_invocations_before_full_lib=3, official_summary=str(summary))
    (P / 'driver-verdict.json').write_text(json.dumps(verdict, indent=2))

if __name__ == '__main__':
    try:
        main()
    except BaseException as error:
        if P.exists():
            first_red(repr(error))
        raise
