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
import urllib.request

W = Path(__file__).resolve().parents[4]
C = W / 'extension/rag3weaver'
R = Path.home() / '.cache/rag3weaver-build'
T = R / 'target-activation'
P = R / 'cx-rate-key-59ac-v1'
F = 'rag3db-native,burn-embedder,burn-ocr,code,daemon,postgres,openai-llm,anthropic-llm'
SDK = 'rag3db-build-59ac9c86bc2b3529b590a7462ab7051e3ad26b8a-Release-p12-v11-g17-s18-rcOFF-stOFF-asanOFF-tsanOFF-ubsanOFF'
PIN_SHA = '2150fcd847b1df7de04908d357a0c78abc05e50ce599e2793ff08ab48108825d'
PROFILE = json.loads((Path(__file__).parent/'sdk-profile-luciepc.json').read_text())
assert PROFILE['sdk'] == SDK
STORE = Path(PROFILE['store'])
SDK_ROOT = STORE / SDK
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
JOBS = min(8, int(os.environ.get('POSTE_FILS', '8')))
assert JOBS >= 1
E.update(CARGO_TARGET_DIR=str(T), CARGO_BUILD_JOBS=str(JOBS), CARGO_INCREMENTAL='0',
         RAG3WEAVER_TARGET_MAX_GO='32', RAG3WEAVER_TARGET_REPRENDRE='1',
         RAG3DB_LIB=SDK, RAG3DB_LIBS_DIR=str(STORE),
         RAG3DB_LIBRARY_DIR=str(SDK_ROOT), RAG3DB_INCLUDE_DIR=str(SDK_ROOT), RAG3DB_SHARED='1',
         CARGO_MANIFEST_DIR=str(C), RUST_TEST_THREADS=str(JOBS), LUCIVY_SCHEDULER_THREADS=str(JOBS),
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
    record = dict(name=name, command=command, cwd=str(C), code=None,
                  stdout=name+'.log', stderr=name+'.stderr.log', state='running')
    phases.append(record)
    save('phases.json', phases)
    print('START', name, 'remaining', int(DEADLINE-started), flush=True)
    with (P/record['stdout']).open('w') as output, (P/record['stderr']).open('w') as errors:
        try:
            result = subprocess.run(command, cwd=C, env=env, stdout=output, stderr=errors,
                                    timeout=max(1, DEADLINE-time.monotonic()-15))
        except BaseException as error:
            record.update(state='interrupted', error=repr(error), seconds=time.monotonic()-started)
            save('phases.json', phases)
            raise
    record.update(code=result.returncode, state='finished', seconds=time.monotonic()-started)
    save('phases.json', phases)
    assert result.returncode == 0, (name, result.returncode, record['stdout'], record['stderr'])
    guard(); size_guard()
    print('PASS', name, flush=True)
    return (P/(name+'.log')).read_text(errors='replace')

def health_snapshot(name):
    rows = []
    for address in ('127.0.0.1:7878', '127.0.0.1:7879'):
        try:
            with urllib.request.urlopen('http://'+address+'/sante', timeout=min(5, max(1, DEADLINE-time.monotonic()-15))) as response:
                data = json.load(response)
            keys = ('service', 'modele', 'dim', 'dual', 'sparse', 'precision', 'lot_conseille', 'factice', 'executable', 'max_chars_per_text', 'cut_texts', 'reranker', 'ocr')
            rows.append(dict(address=address, identity={key: data[key] for key in keys if key in data}))
        except Exception as error:
            rows.append(dict(address=address, error=str(error)))
    save(name, dict(observed_monotonic=time.monotonic(), services=rows))
    return rows


def coordinate_canonical():
    # Le dossier est neuf ; le permis de coordination doit répondre à cette demande réelle.
    save('canonical-coordination-request.json', dict(source=SOURCE, stage='canonical',
         requested_monotonic=time.monotonic(), deadline=DEADLINE,
         services=['127.0.0.1:7878', '127.0.0.1:7879'], postgres_fixture='rag3weaver_test'))
    print('WAIT_CANONICAL_COORDINATION', P, flush=True)
    grant_path = P/'canonical-coordination-grant.json'
    while not grant_path.exists():
        guard()
        time.sleep(1)
    guard()
    grant = json.loads(grant_path.read_text())
    assert grant.get('source') == SOURCE and grant.get('stage') == 'canonical'
    assert grant.get('services') == ['127.0.0.1:7878', '127.0.0.1:7879']
    assert grant.get('postgres_fixture') == 'rag3weaver_test' and grant.get('coordinated') is True
    save('canonical-coordination-accepted.json', grant)

verdict = 'ROUGE'
try:
    guard()
    script = R/'passe_epinglee.sh'
    assert hashlib.sha256(script.read_bytes()).hexdigest() == PIN_SHA
    assert hashlib.sha256((W/'tools/poste/passe_epinglee.sh').read_bytes()).hexdigest() == PIN_SHA
    expected_names = dict(shared_library='librag3db.so', vector_extension='libvector.rag3db_extension', build_id_header='include/common/build_id.h')
    assert len(PROFILE['files']) == 3 and {item['role'] for item in PROFILE['files']} == set(expected_names)
    for item in PROFILE['files']:
        path = Path(item['path'])
        assert item['type'] == 'file' and path == SDK_ROOT/expected_names[item['role']]
        assert path.resolve() == path and path.is_file() and not path.is_symlink()
        with path.open('rb') as handle: observed_sha = hashlib.file_digest(handle, 'sha256').hexdigest()
        assert observed_sha == item['sha256'], ('SDK artifact changed', item['role'])
    save('sdk-profile.json', PROFILE)
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
    cgroup = Path('/proc/self/cgroup').read_text().strip().split('::', 1)[1]
    group = Path('/sys/fs/cgroup'+cgroup)
    save('scope-os.json', dict(receipt=os.environ['RAG3WEAVER_VALIDATION_RECU'],
         cgroup=cgroup, memory_max=(group/'memory.max').read_text().strip(),
         memory_high=(group/'memory.high').read_text().strip(),
         cpu_max=(group/'cpu.max').read_text().strip(), poste_threads=os.environ.get('POSTE_FILS'),
         targeted_jobs=JOBS, supervisor='common receipt verifier'))
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
    coordinate_canonical()
    before = health_snapshot('canonical-health-before.json')
    assert all('error' not in row for row in before), 'canonical service health unavailable'
    assert [(row['identity'].get('modele'), row['identity'].get('dim'), row['identity'].get('factice')) for row in before] == [('granite-278m', 768, False), ('bge-m3', 1024, False)]
    try:
        run('official-pass', ['bash', str(script), W.name, P.name, TESTS[0].rsplit('::', 1)[1]],
            extra=dict(CIBLE=T.name, SUITE_DU_TEMOIN='lib', SUITES='e2e_code_sync,e2e_lectures,e2e_embarqueur_a_la_demande'))
    finally:
        after = health_snapshot('canonical-health-after.json')
    assert before == after, 'canonical service identities changed'
    summary = (P/'resume.log').read_text().splitlines()
    assert summary and summary[0].startswith('commit '+git('rev-parse', '--short', 'HEAD')+' ') and summary[-1] == 'VERT'
    assert (P/'base').read_text().strip() == git('merge-base', 'HEAD', 'origin/master')
    verdict = 'VERT'
finally:
    save('result.json', dict(verdict=verdict, source=SOURCE, deadline=DEADLINE,
                            remaining_seconds=DEADLINE-time.monotonic(), phases=phases))
    print('RESULT', verdict, flush=True)
