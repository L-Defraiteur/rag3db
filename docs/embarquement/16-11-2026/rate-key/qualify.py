#!/usr/bin/env python3
"""Neuf ciblés hermétiques d'abord, puis la passe courante, dans une seule portée."""
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import sys
import time

W = Path(__file__).resolve().parents[4]
C = W / 'extension/rag3weaver'
R = Path.home() / '.cache/rag3weaver-build'
T = R / 'target-activation'
P = R / 'cx-rate-key-59ac-v1'
F = 'rag3db-native,burn-embedder,burn-ocr,code,daemon,postgres,openai-llm,anthropic-llm'
SDK = 'rag3db-build-59ac9c86bc2b3529b590a7462ab7051e3ad26b8a-Release-p12-v11-g17-s18-rcOFF-stOFF-asanOFF-tsanOFF-ubsanOFF'
PIN_SHA = '2150fcd847b1df7de04908d357a0c78abc05e50ce599e2793ff08ab48108825d'
SOURCE = sys.argv[1]
assert re.fullmatch('[0-9a-f]{40}', SOURCE)
AUTH = json.loads(Path(os.environ['RAG3WEAVER_VALIDATION_RECU']).read_text())
DEADLINE = AUTH['echeance']
assert time.monotonic() < DEADLINE - 20
P.mkdir(exist_ok=False)
E = os.environ.copy()
for key in list(E):
    if key.startswith(('RAG3WEAVER_SERVICE_', 'RAG3WEAVER_EMBED_')) or key in ('RAG3WEAVER_MOTEUR_ANCIEN', 'RAG3WEAVER_PG'):
        E.pop(key, None)
E.update(CARGO_TARGET_DIR=str(T), CARGO_BUILD_JOBS='8', CARGO_INCREMENTAL='0',
         RAG3WEAVER_TARGET_MAX_GO='32', RAG3WEAVER_TARGET_REPRENDRE='1',
         RAG3DB_LIB=SDK, RAG3DB_LIBS_DIR=str(Path.home()/'.cache/rag3db-libs'),
         CARGO_MANIFEST_DIR=str(C), RUST_TEST_THREADS='8', LUCIVY_SCHEDULER_THREADS='8',
         RAG3WEAVER_STATE_DIR=str(P/'state'), RAG3WEAVER_JOURNAUX_DIR=str(P/'journals'),
         RAG3WEAVER_BUILTIN_TEMPLATES_DIR=str(P/'templates'))
TESTS = ['catalog::tests::'+name for name in (
    'the_measured_rate_is_noted_under_the_called_address_and_host',
    'an_in_process_embedder_is_noted_as_local_at_its_host',
    'the_known_rate_falls_back_to_the_old_key_then_prefers_the_new_one',
    'an_unknown_remote_rate_address_never_becomes_local_or_calls_the_provider',
    'rate_keys_keep_addresses_and_client_hosts_isolated',
    'the_known_rate_tracks_the_taken_address_without_provider_calls',
    'rate_place_access_requires_an_address_and_host_without_legacy_writes',
    'a_shared_embedder_forwards_its_called_address',
)] + ['model_source::activation::tests::reading_the_rate_address_never_starts_an_on_demand_provider']
phases = []

def save(name, value):
    (P/name).write_text(json.dumps(value, ensure_ascii=False, indent=2)+'\n')

def git(*args):
    return subprocess.check_output(['git', *args], cwd=W, text=True).strip()

def guard():
    assert git('rev-parse', 'HEAD') == SOURCE, 'source changed'
    assert not git('status', '--porcelain'), 'source dirty'
    for name in ('rebase-merge', 'rebase-apply'):
        assert not Path(git('rev-parse', '--path-format=absolute', '--git-path', name)).exists()
    assert time.monotonic() < DEADLINE-15, 'global budget exhausted'

def size_guard():
    if T.exists():
        size = int(subprocess.check_output(['du', '-sB1', str(T)], text=True).split()[0])
        assert size <= 34359738368, 'target exceeds 32 GiB; no automatic cleanup'
        return size
    return 0

def run(name, argv, export=False, extra=None):
    guard(); size_guard()
    env = E.copy(); env.update(extra or {})
    command = (['bash', '-c', '. scripts/lib_du_magasin.sh; exporter_la_lib || exit 2; exec "$@"', 'rate-key'] + argv) if export else argv
    started = time.monotonic()
    print('START', name, 'remaining', int(DEADLINE-started), flush=True)
    with (P/(name+'.log')).open('w') as output:
        result = subprocess.run(command, cwd=C, env=env, stdout=output, stderr=subprocess.STDOUT,
                                timeout=max(1, DEADLINE-time.monotonic()-15))
    phases.append(dict(name=name, code=result.returncode, seconds=time.monotonic()-started))
    save('phases.json', phases)
    assert result.returncode == 0, (name, result.returncode)
    guard(); size_guard()
    print('PASS', name, flush=True)
    return (P/(name+'.log')).read_text(errors='replace')

verdict = 'ROUGE'
try:
    guard()
    script = R/'passe_epinglee.sh'
    assert hashlib.sha256(script.read_bytes()).hexdigest() == PIN_SHA
    assert hashlib.sha256((W/'tools/poste/passe_epinglee.sh').read_bytes()).hexdigest() == PIN_SHA
    refs = []; unknown = []
    for q in Path('/proc').iterdir():
        if not q.name.isdigit() or int(q.name) == os.getpid(): continue
        try:
            if q.stat().st_uid != os.getuid(): continue
            for link in [q/'exe', q/'cwd', *list((q/'fd').iterdir())]:
                try: dest = os.readlink(link)
                except FileNotFoundError: continue
                except PermissionError: unknown.append(str(link)); continue
                if dest == str(T) or dest.startswith(str(T)+'/'):
                    refs.append(dict(pid=int(q.name), entry=str(link), target=dest))
        except FileNotFoundError: continue
        except PermissionError: unknown.append(str(q))
    save('target-quiescence.json', dict(references=refs, inaccessible=unknown,
         limitation='Inaccessible foreign descriptors remain unknown, never counted as zero.'))
    assert not refs, 'active references to inherited target'
    save('source.json', dict(source=SOURCE, tree=git('rev-parse', 'HEAD^{tree}'),
         base=git('merge-base', 'HEAD', 'origin/master'), sdk=SDK, pin_sha256=PIN_SHA,
         deadline=DEADLINE, budget_seconds=5400, target=str(T), initial_target_bytes=size_guard(), tests=TESTS))
    run('scope', ['python3', 'scripts/confiner_validation.py', 'verifier'])
    run('target-ownership', ['bash', '-c', '. scripts/garde_target.sh; garder_le_target "$CARGO_TARGET_DIR" "$PWD"'])
    build = run('compile-targeted', ['bash', 'scripts/cargo_magasin.sh', 'test', '--lib', '--features', F,
                                   '--no-run', '--message-format=json'])
    artifacts = []
    for line in build.splitlines():
        try: item = json.loads(line)
        except ValueError: continue
        if item.get('reason') == 'compiler-artifact' and item.get('executable') and item.get('profile', {}).get('test') and item.get('target', {}).get('src_path') == str(C/'src/lib.rs'):
            artifacts.append(item)
    assert len(artifacts) == 1, 'expected one test library artifact'
    binary = Path(artifacts[0]['executable'])
    assert binary.resolve().is_relative_to(T.resolve()) and binary.is_file()
    with binary.open('rb') as handle: binary_sha = hashlib.file_digest(handle, 'sha256').hexdigest()
    save('artifact.json', dict(artifact=artifacts[0], sha256=binary_sha))
    for index, test in enumerate(TESTS, 1):
        listed = run(f'target-{index:02}-list', [str(binary), test, '--exact', '--list', '--format', 'terse'], export=True)
        assert listed.count(test+': test') == 1, 'test absent or ambiguous'
        log = run(f'target-{index:02}', [str(binary), test, '--exact', '--nocapture', '--test-threads=1'], export=True)
        assert re.findall(r'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;', log)[-1] == ('1', '0', '0')
    run('official-pass', ['bash', str(script), W.name, P.name, TESTS[0].rsplit('::', 1)[1]],
        extra=dict(CIBLE=T.name, SUITE_DU_TEMOIN='lib', SUITES='e2e_code_sync,e2e_lectures,e2e_embarqueur_a_la_demande'))
    summary = (P/'resume.log').read_text().splitlines()
    assert summary and summary[0].startswith('commit '+git('rev-parse', '--short', 'HEAD')+' ') and summary[-1] == 'VERT'
    assert (P/'base').read_text().strip() == git('merge-base', 'HEAD', 'origin/master')
    verdict = 'VERT'
finally:
    save('result.json', dict(verdict=verdict, source=SOURCE, deadline=DEADLINE,
                            remaining_seconds=DEADLINE-time.monotonic(), phases=phases))
    print('RESULT', verdict, flush=True)
