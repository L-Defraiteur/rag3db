"""Real canonical pass then real tty stages, under one already admitted Common."""
import json
import math
import os
from pathlib import Path
import re
import subprocess
import sys
import time
from strict_target_guard import digest, snapshot, scan, require_source, require_hashes, require_provenance
from qualification_contract import cpu_allocation, contract

q = Path(__file__).parent
c = json.loads((q / 'prepared.json').read_text())
a = json.loads((q / 'allocation.json').read_text())
w, t, p = (Path(c[k]) for k in ['worktree', 'target', 'proofs'])
p.mkdir(mode=0o700)
subprocess.run([sys.executable, str(q / 'confiner_validation.py'), 'verifier'], check=True)
receipt = Path(os.environ['RAG3WEAVER_VALIDATION_RECU'])
(p / 'authority-path.json').write_text(json.dumps(dict(path=str(receipt), end=a['end'])))
require_source(w, c['source'])
require_hashes(c['controllers']); require_hashes(c['sdk']); require_hashes(c['tmux']); require_hashes(c['runtime'])
assert digest(q / 'target-ledger.json') == c['ledger_sha']
require_provenance(t, json.loads((q / 'target-ledger.json').read_text()))
quiet = scan('/proc', [w, Path(c['old_worktree']), t])
(p / 'quiet-before.json').write_text(json.dumps(quiet, indent=2)); assert not quiet['found']
(p / 'cpu-allocation.json').write_text(json.dumps(cpu_allocation(), indent=2))
for name in ['state', 'cache', 'config', 'data', 'fixture']:
    (p / name).mkdir(mode=0o700)
tmp = Path.home() / '.cache/ttyq1'; tmp.mkdir(mode=0o700)
env = {key: os.environ[key] for key in ['HOME', 'USER', 'LOGNAME', 'PATH', 'SHELL', 'LANG', 'LC_ALL', 'TZ',
       'CARGO_HOME', 'RUSTUP_HOME', 'DBUS_SESSION_BUS_ADDRESS', 'RAG3WEAVER_VALIDATION_RECU',
       'RAG3WEAVER_VALIDATION_JETON', 'POSTE_FILS', 'RAYON_NUM_THREADS', 'RUST_TEST_THREADS'] if key in os.environ}
env.update(CARGO_TARGET_DIR=str(t), CARGO_BUILD_JOBS='4', CARGO_INCREMENTAL='0', CARGO_NET_OFFLINE='true',
           CARGO_TERM_COLOR='never', RAG3WEAVER_TARGET_REPRENDRE='1', RAG3WEAVER_TARGET_MAX_GO='32',
           RAG3DB_LIB=c['sdk_id'], RAG3DB_LIBS_DIR=str(Path.home() / '.cache/rag3db-libs'),
           TMPDIR=str(tmp), XDG_STATE_HOME=str(p/'state'), XDG_CACHE_HOME=str(p/'cache'),
           XDG_CONFIG_HOME=str(p/'config'), XDG_DATA_HOME=str(p/'data'), RAG3WEAVER_STATE_DIR=str(p/'fixture'),
           RAG3WEAVER_NAVIGATEUR=c['browser'], RAG3WEAVER_PAQUETS_REACT=c['react'],
           RAG3WEAVER_EXIGER_NAVIGATEUR='1', TTY_CONTROLLER_DIR=str(q))


def stage(name, command, cwd, environment, cap, deadline=None):
    import re
    import threading
    from pathlib import Path
    end = min(a['end'] - 20, time.monotonic() + cap, deadline or a['end'])
    remaining = math.floor(end - time.monotonic())
    assert remaining > 0
    row = dict(name=name, argv=command, cwd=str(cwd), start=time.monotonic(), end=end)
    (p / (name + '-command.json')).write_text(json.dumps(row, indent=2))
    lock, done = threading.Lock(), threading.Event()
    first, observer_errors = [], []

    def observe(raw, path, canonical_summary=False):
        for line in raw.splitlines():
            failed = re.match(r'^test .+ \.\.\. FAILED\s*$|^test result: FAILED|^error(?:\[|:)', line)
            if line.startswith('{'):
                try:
                    message = json.loads(line)
                    failed = failed or (message.get('reason') == 'compiler-message' and message.get('message', {}).get('level') == 'error')
                except ValueError:
                    pass
            if not failed and not (canonical_summary and line.startswith('ROUGE ')):
                continue
            with lock:
                if not first:
                    cause = dict(stage=name, path=str(path), line=line, observed=time.monotonic())
                    first.append(cause)
                    request_common_stop(cause)

    def poll_logs():
        try:
            while not done.is_set():
                paths = [p / (name + '.log')]
                if name == 'canonical':
                    paths.extend(Path(c['canonical_proofs']).glob('*.log'))
                for path in paths:
                    try:
                        observe(path.read_text(), path, path.name == 'resume.log')
                    except (FileNotFoundError, KeyError):
                        pass
                done.wait(.05)
        except BaseException as error:
            observer_errors.append(error)

    class ObservedOutput:
        # Real children use fileno, observed concurrently above. Python transport
        # fakes use write: same observer before their next executable statement.
        def __init__(self, stream):
            self.stream, self.pending = stream, ''

        def fileno(self):
            return self.stream.fileno()

        def flush(self):
            self.stream.flush()

        def write(self, value):
            count = self.stream.write(value)
            self.stream.flush()
            self.pending += value
            complete, separator, tail = self.pending.rpartition('\n')
            if separator:
                observe(complete, p / (name + '.log'))
                self.pending = tail
            return count

    watcher = None
    try:
        with (p / (name + '.log')).open('x') as out:
            watcher = threading.Thread(target=poll_logs, daemon=True)
            watcher.start()
            completed = subprocess.run(command, cwd=cwd, env=environment, stdout=ObservedOutput(out),
                                       stderr=subprocess.STDOUT, timeout=remaining)
            final_paths = [p/(name+'.log')]
            if name == 'canonical':
                final_paths.extend(Path(c['canonical_proofs']).glob('*.log'))
            for path in final_paths:
                try:
                    observe(path.read_text(),path,path.name=='resume.log')
                except (FileNotFoundError,KeyError):
                    pass
        row['rc'] = completed.returncode
    finally:
        done.set()
        if watcher is not None:
            watcher.join(max(0, min(1, end - time.monotonic())))
            assert not watcher.is_alive(), 'log observer did not stop within original stage deadline'
        row['finished'] = time.monotonic()
        row['first_red'] = first
        row['observer_errors'] = [repr(error) for error in observer_errors]
        (p / (name + '-result.json')).write_text(json.dumps(row, indent=2))
    assert not first and not observer_errors and row['rc'] == 0 and row['finished'] < end, row
    return (p / (name + '.log')).read_text()


def results(log, passed=None, allow_ignored=False, kind=None):
    headers = re.findall(r'^\s*(?:Running tests/(\w+)\.rs|Running unittests (src/lib\.rs))', log, re.M)
    labels = [suite or unit for suite, unit in headers]
    if kind is None:
        assert len(labels) == 2 and labels[0] == 'e2e_filets'
        kind = labels[1]
    assert kind in INVENTORIES, 'no pre-execution catalogue for this result'
    expected = INVENTORIES[kind]
    if 'ELF_EXPECTED' in globals():
        binary = re.escape(ELF_EXPECTED[kind])
        binding = (r'^▸ binaire e2e : ' + binary + r' \(' if len(expected) == 2 else
                   r'^\s*Running unittests src/lib\.rs \(' + binary + r'\)')
        assert re.search(binding, log, re.M), 'result not bound to inventoried ELF'
        assert kind in globals().get('ELF_PROVENANCE', {}), 'no actual execution-content provenance'
        provenance = ELF_PROVENANCE[kind]
        catalogue, execution = provenance['catalogue'], provenance['execution']
        original, copy = catalogue['path'], execution['executed_path']
        expected_sha = catalogue['sha256']
        assert re.fullmatch(r'[0-9a-f]{64}',expected_sha)
        assert original == ELF_EXPECTED[kind] == execution['source_path']
        assert copy != original, 'execution must be bound to the preserved content copy'
        assert all(execution[field] == expected_sha for field in ['before_sha256','after_sha256','copy_sha256'])
        # Physical digest is the existing boundary, replaced only by virtual-file
        # bytes in the hermetic witness. Receipt fields alone never suffice.
        assert digest(copy) == digest(original) == expected_sha
    assert labels == (['e2e_filets', kind] if len(expected) == 2 else ['src/lib.rs']), labels
    blocks = re.split(r'^\s*running (\d+) tests?\s*$', log, flags=re.M)
    assert len(blocks) == 1 + 2 * len(expected), 'wrong number or order of Rust blocks'
    ignored_expected = globals().get('IGNORED_EXPECTED', {}).get(kind, set())
    seen_passed = []
    for index, catalogue in enumerate(expected):
        count, body = int(blocks[1+2*index]), blocks[2+2*index]
        assert catalogue and len(set(catalogue)) == len(catalogue)
        tests = re.findall(r'^test (\S+)(?: - should panic)? \.\.\. (ok|ignored(?:, [^\n]*)?)\s*$', body, re.M)
        names = [name for name, status in tests]
        assert count == len(catalogue) == len(names) == len(set(names)), 'missing, duplicate or extra name'
        assert sorted(names) == sorted(catalogue), 'names differ from the pre-execution inventory'
        ignored = {name for name, status in tests if status.startswith('ignored')}
        assert ignored == (set(catalogue) & set(ignored_expected))
        summaries = re.findall(r'^test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored; (\d+) measured;', body, re.M)
        assert len(summaries) == 1
        good, bad, skipped, measured = map(int, summaries[0])
        assert bad == measured == 0 and good == len(catalogue)-len(ignored) and skipped == len(ignored)
        assert allow_ignored or skipped == 0
        seen_passed.append(good)
    assert passed is None or seen_passed[-1] == passed
    assert not re.search(r'^test result: FAILED|^error(?:\[|:)|^SAUTÉ|NON JOUÉES', log, re.M)


def request_common_stop(cause):
    """Same owned-authority/pidfd primitive as the reviewed tty harness watcher."""
    import signal
    authority = json.loads(receipt.read_text())
    assert authority['etat'] == 'actif' and authority['script'] == str(q/'confiner_validation.py')
    pid = authority['pid']
    fd = os.pidfd_open(pid)
    try:
        proc = Path('/proc')/str(pid)
        assert proc.stat().st_uid == os.getuid()
        assert proc.joinpath('stat').read_text().rsplit(') ',1)[1].split()[19] == str(authority['debut'])
        assert proc.joinpath('cgroup').read_text().strip().endswith(authority['cgroup'])
        with (p/'first-red-stop.json').open('x') as marker:
            json.dump(dict(cause=cause,pid=pid,start=authority['debut'],action='TERM own Common via verified pidfd'),marker)
            marker.flush(); os.fsync(marker.fileno())
        # Raw output remains outside the target, and is flushed before TERM.
        for path in [*p.glob('*.log'), *Path(c['canonical_proofs']).glob('*.log')]:
            with path.open('rb') as raw:
                os.fsync(raw.fileno())
        signal.pidfd_send_signal(fd,signal.SIGTERM)
    finally:
        os.close(fd)


def feature_results(log, games):
    rows, started = [], False
    for row in re.findall(r'^VERT  (.+)$',log,re.M):
        # The script first runs these two named guards. They are not feature
        # games; recognise their exact grammar only before the games begin.
        ticket_guard = re.fullmatch(r'tickets : \d+ en-têtes valides \(\d+ ouverts, \d+ fermés\)',row)
        analysis_guard = row == "règles de l'analyse : le lot ne touche pas l'analyse"
        if not started and (ticket_guard or analysis_guard):
            continue
        started = True
        rows.append(row)
    assert rows == games, 'missing, extra, duplicated or reordered feature game'
    assert log.splitlines()[-1]=='CHECK_FEATURES VERT 8/8'


def inventory_build(label, features, targets, phase_end, directory):
    """Compile without playing tests; bind catalogue to Cargo's real artifact/source."""
    build = stage(label+'-compile', ['bash', str(w/'extension/rag3weaver/scripts/cargo_magasin.sh'),
        'test', '--offline', '--locked', '--no-run', '--message-format=json',
        '-j4' if label.startswith('tty-') else '-j8', '--features', features,
        *targets], directory, env, 600, phase_end)
    artifacts = []
    for line in build.splitlines():
        try:
            message = json.loads(line)
        except ValueError:
            continue
        if message.get('reason') == 'compiler-artifact' and message.get('executable') and message.get('profile',{}).get('test'):
            source = Path(message['target']['src_path'])
            binary = Path(message['executable'])
            assert binary.is_relative_to(t)
            if source in [directory/'src/lib.rs', directory/'tests/e2e/main.rs']:
                artifacts.append((source,binary))
    assert len(artifacts) == sum(flag in ['--lib','--test'] for flag in targets)
    assert len(set(artifacts)) == len(artifacts)
    catalogues = {}
    for source,binary in artifacts:
        names = stage(label+'-'+source.parent.name+'-list', [str(binary),'--list','--format','terse'], q, env, 30, phase_end)
        ignored = stage(label+'-'+source.parent.name+'-ignored', [str(binary),'--list','--ignored','--format','terse'], q, env, 30, phase_end)
        catalogue = re.findall(r'^(\S+): test$', names, re.M)
        ignored_names = re.findall(r'^(\S+): test$', ignored, re.M)
        assert catalogue and len(set(catalogue)) == len(catalogue) and set(ignored_names) <= set(catalogue)
        catalogues[str(source)] = dict(binary=str(binary),sha256=digest(binary),names=catalogue,ignored=ignored_names,
                                      source=str(source),source_sha256=digest(source),features=features)
    (p/(label+'-catalogue.json')).write_text(json.dumps(catalogues,indent=2))
    return catalogues


canonical_end = min(a['end'] - 650, time.monotonic() + 2700)
assert canonical_end > time.monotonic() + 60
grant = contract(q, c, a, a['grant_sha256'], canonical_end)
canonical = Path(c['canonical_proofs'])
assert not canonical.exists() and not canonical.is_symlink()
canon_env = dict(env, CIBLE=t.name, SUITE_DU_TEMOIN='e2e_tty', SUITES='e2e_lectures,e2e_code',
                 RAG3WEAVER_PG=next(row['url'] for row in grant['leases'] if row['role'] == 'postgres'))
(p / 'contract-admission.json').write_text(json.dumps(dict(nonce=grant['nonce'], allocation=a['reference'],
                 admitted=time.monotonic(), phase='canonical', phase_end=canonical_end), indent=2))
features='rag3db-native,burn-embedder,burn-ocr,code,daemon,postgres,openai-llm,anthropic-llm'
weaver=w/'extension/rag3weaver'
base_catalogue=inventory_build('root-inventory',features,['--lib','--test','e2e'],canonical_end,weaver)
root_lib=base_catalogue[str(weaver/'src/lib.rs')]
e2e=base_catalogue[str(weaver/'tests/e2e/main.rs')]
nav_catalogue=inventory_build('nav-inventory',features+',navigateur',['--lib'],canonical_end,weaver)
nav_lib=nav_catalogue[str(weaver/'src/lib.rs')]
filets=['chaque_suite_est_montee','aucune_suite_ne_se_relance_par_son_nom_court']
assert set(filets)<=set(e2e['names'])
INVENTORIES={name:[filets,[n for n in e2e['names'] if n.startswith(name+'::')]]
             for name in ['e2e_tty','e2e_lectures','e2e_code']}
assert len(INVENTORIES['e2e_tty'][1])==4
INVENTORIES['lib']=[root_lib['names']]
IGNORED_EXPECTED={'lib':set(root_lib['ignored'])}
nav_filters=set()
for source in (weaver/'src').rglob('*.rs'):
    lines=source.read_text().splitlines()
    for index,line in enumerate(lines):
        if 'feature = "navigateur"), ignore' in line:
            nav_filters.update(re.findall(r'fn ([a-z0-9_]+)\(', '\n'.join(lines[index:index+5])))
assert nav_filters
INVENTORIES['navigateur']=[[n for n in nav_lib['names'] if any(f in n for f in nav_filters)]]
ELF_EXPECTED={name:e2e['binary'] for name in ['e2e_tty','e2e_lectures','e2e_code']}
ELF_EXPECTED.update(lib=root_lib['binary'],navigateur=nav_lib['binary'])
# A real execution-capture producer must populate this from observed/copy-bound
# executions. Never manufacture receipts from the catalogue to satisfy results.
ELF_PROVENANCE={}
(p/'expected-inventories.json').write_text(json.dumps(dict(inventories=INVENTORIES,
    ignored={k:sorted(v) for k,v in IGNORED_EXPECTED.items()},elf=ELF_EXPECTED),indent=2))
(p/'feature-games').mkdir(mode=0o700)
canon_env['CHECK_FEATURES_LOGS']=str(p/'feature-games')
stage('canonical', ['bash', str(q/'passe_epinglee.sh'), w.name, canonical.name, '', '-'], q, canon_env, 2700, canonical_end)
assert (canonical/'base').read_text().strip() == c['base']
summary = (canonical/'resume.log').read_text()
assert summary.splitlines()[-1] == 'VERT' and c['source'][:7] in summary.splitlines()[0]
for name, count in [('temoins', 4), ('e2e_lectures', None), ('e2e_code', None), ('navigateur', None)]:
    results((canonical/(name+'.log')).read_text(), passed=count,kind='e2e_tty' if name=='temoins' else name)
results((canonical/'lib.log').read_text(), allow_ignored=True,kind='lib')
f8=(canonical/'check_features.log').read_text()
games=['sans-feature','daemon','rag3db-native,daemon','rag3db-native,burn-embedder,burn-ocr,code,daemon,postgres',
       features,'rag3db-native,code,daemon,openai-llm','rag3db-native,code','rag3db-native']
feature_results(f8,games)
for game in games:
    assert (p/'feature-games'/(game.replace(',','+')+'.log')).is_file()
proof = (canonical/'preuve.log').read_text()
assert re.search(r'(Checking|Compiling) rag3weaver \S+ \('+re.escape(str(weaver))+r'\)', proof)
assert not re.search(r'^error(?:\[|:)', proof, re.M)
assert c['sdk_id'] in ''.join(path.read_text(errors='replace') for path in canonical.glob('*.log'))
assert (t/'.arbre-du-target').read_text().strip() == str(w/'extension/rag3weaver')
tty = w/'extension/rag3weaver/tty'
tty_catalogue=inventory_build('tty-inventory','vue',['--lib'],a['end']-260,tty)
tty_lib=tty_catalogue[str(tty/'src/lib.rs')]
INVENTORIES['tty-lib']=[tty_lib['names']]
IGNORED_EXPECTED['tty-lib']=set(tty_lib['ignored'])
assert not IGNORED_EXPECTED['tty-lib']
ELF_EXPECTED['tty-lib']=tty_lib['binary']
results(stage('tty-lib', [sys.executable, '-B', str(q/'cargo_garde.py'), 'test', '--offline', '--locked',
                        '-j4', '--lib'], tty, env, 240),kind='tty-lib')
assert (t/'.arbre-du-target').read_text().strip() == str(tty)
stage('tty-build', [sys.executable, '-B', str(q/'cargo_garde.py'), 'build', '--offline', '--locked',
                    '-j4', '--bin', 'rag3weaver-tty'], tty, env, 180)
elf = t/'debug/rag3weaver-tty'
assert digest(elf) != c['historical_elf_sha'], 'historical ELF cannot qualify new source'
(p/'tty-elf-new.json').write_text(json.dumps(dict(path=str(elf), sha256=digest(elf), source=c['source'])))
(p/'target-before-harness.json').write_text(json.dumps(snapshot(t), indent=2))
stage('harness', [sys.executable, '-B', str(q/'qualification_harness.py')], q, env, 240)
assert json.loads((p/'harness-stage/harness-result.json').read_text())['verdict'] == 'VERT'
require_source(w, c['source']); require_hashes(c['sdk']); require_hashes(c['controllers']); require_hashes(c['runtime'])
(p/'target-after.json').write_text(json.dumps(snapshot(t), indent=2))
quiet = scan('/proc', [w, Path(c['old_worktree']), t])
(p/'quiet-driver-final.json').write_text(json.dumps(quiet, indent=2)); assert not quiet['found']
subprocess.run([sys.executable, str(q/'confiner_validation.py'), 'verifier'], check=True)
assert time.monotonic() < a['end'] - 15
(p/'driver-result.json').write_text(json.dumps(dict(verdict='VERT', source=c['source'],
    scope='canonical plus tty; pending outer Common termination and publication', finished=time.monotonic())))
