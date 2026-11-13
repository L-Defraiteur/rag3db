"""Prepared CPU A/B orchestration. No execution without separate MESURE GO."""
import argparse, hashlib, itertools, json, os, shutil, statistics, subprocess, time
from pathlib import Path

p = argparse.ArgumentParser()
p.add_argument('--ancien', required=True)
p.add_argument('--nouveau', required=True)
p.add_argument('--zone', required=True)
p.add_argument('--target', required=True)
p.add_argument('--cpus', required=True, help='Exactly four coordinated available CPU IDs')
a = p.parse_args()
b = Path(a.zone).resolve(); b.mkdir(parents=True, exist_ok=False)
receipt_path = Path(os.environ['RAG3WEAVER_VALIDATION_RECU'])
receipt = json.loads(receipt_path.read_text()); end = receipt['echeance']
assert receipt['etat'] == 'actif' and 0 < end - time.monotonic() <= 600
cg = Path('/proc/self/cgroup').read_text().split('0::', 1)[1].strip()
mem = {n: (Path('/sys/fs/cgroup') / cg.lstrip('/') / n).read_text().strip()
       for n in ['memory.max', 'memory.high']}
assert mem == {'memory.max': '8589934592', 'memory.high': 'max'}
cpus = {int(n) for n in a.cpus.split(',')}
assert len(cpus) == 4 and cpus <= os.sched_getaffinity(0)
os.sched_setaffinity(0, cpus)
os.environ['CARGO_BUILD_JOBS'] = '4'
os.environ['CARGO_TARGET_DIR'] = str(Path(a.target).resolve())
os.environ['CARGO_TERM_COLOR'] = 'never'
source = {'A': '663975adf11fc4a9d6cadf90e70ee68f6c9db075',
          'B': 'fbd3f0a4179917e3674fa52a56ffde027831a354'}
roots = {'A': Path(a.ancien).resolve(), 'B': Path(a.nouveau).resolve()}
for variant, root in roots.items():
    assert subprocess.check_output(['git', '-C', str(root), 'rev-parse', 'HEAD'], text=True).strip() == source[variant]
    assert not subprocess.check_output(['git', '-C', str(root), 'status', '--porcelain'], text=True).strip()
harness = Path(__file__).with_name('cout-envoi')
rust_bytes = (harness / 'src/main.rs').read_bytes()
template = (harness / 'Cargo.toml.template').read_text()

def budget():
    live = json.loads(receipt_path.read_text())
    assert live['etat'] == 'actif' and live['echeance'] == end
    assert end - time.monotonic() > 5, 'global MESURE deadline, no reset'

def run(argv, log):
    budget()
    with (b / log).open('w') as f:
        result = subprocess.run(argv, stdout=f, stderr=subprocess.STDOUT)
    if result.returncode:
        with (b / 'premier-rouge.json').open('x') as f:
            json.dump(dict(argv=argv, code=result.returncode, log=log), f)
        raise SystemExit(result.returncode)
    budget()
    return (b / log).read_text()

meta = dict(sources=source, cpus=sorted(cpus), cgroup=cg, memory=mem, echeance=end,
            harness_sha256=hashlib.sha256(rust_bytes).hexdigest(), stderr_samples='/dev/null',
            limitation='global delta incl HandlerGuard; pinned diagnostics still cost CPU')
meta['toolchain'] = run(['rustc', '--version', '--verbose'], 'toolchain.log')
(b / 'metadata.json').write_text(json.dumps(meta, indent=2))
executables = {}
for variant in ['A', 'B']:
    app = b / ('harnais-' + variant); (app / 'src').mkdir(parents=True)
    (app / 'src/main.rs').write_bytes(rust_bytes)
    path = json.dumps(str(roots[variant] / 'luciole'))[1:-1]
    (app / 'Cargo.toml').write_text(template.replace('@LUCIOLE_PATH@', path))
    argv = ['cargo', 'build', '-j4', '--release', '--manifest-path', str(app / 'Cargo.toml')]
    if variant == 'B':
        shutil.copy2(b / 'harnais-A/Cargo.lock', app / 'Cargo.lock')
        argv.append('--locked')
    run(argv, 'build-' + variant + '.log')
    exe = Path(a.target).resolve() / 'release/cout-envoi-luciole'
    frozen = b / ('exe-' + variant); shutil.copy2(exe, frozen)
    executables[variant] = frozen
assert (b / 'harnais-A/Cargo.lock').read_bytes() == (b / 'harnais-B/Cargo.lock').read_bytes()
meta['lock_sha256'] = hashlib.sha256((b / 'harnais-A/Cargo.lock').read_bytes()).hexdigest()
meta['exe_sha256'] = {v: hashlib.sha256(exe.read_bytes()).hexdigest() for v, exe in executables.items()}
(b / 'metadata.json').write_text(json.dumps(meta, indent=2)); budget()
rows = []
with (b / 'samples.jsonl').open('w') as output:
    for dispatch, producers, capacity in itertools.product(['worker', 'pinned'], [1, 4], [0, 1024]):
        for block in range(3):
            for variant in ['A', 'B', 'B', 'A']:
                budget()
                load = Path('/proc/loadavg').read_text().strip()
                argv = [str(executables[variant]), dispatch, str(producers), str(capacity), '100000']
                case = dict(index=len(rows)+1, variant=variant, source=source[variant],
                            dispatch=dispatch, producers=producers, capacity=capacity,
                            block=block, argv=argv, exe_sha256=meta['exe_sha256'][variant],
                            cpus=sorted(cpus), load=load, stderr='/dev/null')
                prefix = f"cas-{len(rows)+1:03d}"
                raw_path = b / (prefix + '.stdout')
                # Direct file output survives a worker panic followed by a
                # blocked Fence and an eventual common-supervisor timeout.
                # Persist identity BEFORE waiting; no PIPE buffer can hide it.
                with raw_path.open('w') as raw:
                    child = subprocess.Popen(argv, stdout=raw, stderr=subprocess.DEVNULL, text=True)
                    case['pid'] = child.pid
                    try:
                        case['start'] = Path(f'/proc/{child.pid}/stat').read_text().rsplit(') ', 1)[1].split()[19]
                    except OSError as error:
                        case['start'] = None; case['start_error'] = repr(error)
                    case['etat'] = 'en cours'; case['code'] = None
                    with (b / (prefix + '.json')).open('w') as identity:
                        json.dump(case, identity, indent=2)
                        identity.flush(); os.fsync(identity.fileno())
                    case['code'] = child.wait()
                    case['etat'] = 'termine'
                    raw.flush(); os.fsync(raw.fileno())
                stdout = raw_path.read_text()
                case['stdout_sha256'] = hashlib.sha256(stdout.encode()).hexdigest()
                (b / (prefix + '.json')).write_text(json.dumps(case, indent=2))
                def case_red(reason):
                    marker = b / 'premier-rouge.json'
                    if not marker.exists():
                        with marker.open('x') as f:
                            json.dump(dict(raison=reason, raw=prefix+'.stdout', identite=case), f, indent=2)
                            f.flush(); os.fsync(f.fileno())
                    raise SystemExit(child.returncode if child.returncode > 0 else 2)
                if case['start'] is None:
                    case_red('identité PID/start non attestée ; stdout conservé')
                if child.returncode:
                    case_red('code cas non nul ; diagnostic panic éventuel dans raw stdout')
                try:
                    row = json.loads(stdout)
                    assert row['count'] == row['messages'] == producers * 100000
                    assert row['status'] == 'ok'
                    assert row['dispatch'] == dispatch and row['producers'] == producers and row['capacity'] == capacity
                    assert row['seconds'] > 0 and row['messages_per_second'] > 0
                except (ValueError, KeyError, TypeError, AssertionError) as error:
                    case_red('JSON/schema/compte invalide: ' + repr(error))
                row.update(variant=variant, source=source[variant], block=block, load=load)
                rows.append(row); output.write(json.dumps(row) + '\n'); output.flush()
                budget()
assert len(rows) == 96
summary = []
for dispatch, producers, capacity in itertools.product(['worker', 'pinned'], [1, 4], [0, 1024]):
    subset = [r for r in rows if (r['dispatch'], r['producers'], r['capacity']) == (dispatch, producers, capacity)]
    med = {}
    item = dict(dispatch=dispatch, producers=producers, capacity=capacity)
    for variant in ['A', 'B']:
        values = [r['messages_per_second'] for r in subset if r['variant'] == variant]
        med[variant] = statistics.median(values)
        item[variant] = dict(median=med[variant], minimum=min(values), maximum=max(values),
                             quartiles=statistics.quantiles(values, n=4))
    item['ratio_throughput_B_over_A'] = med['B'] / med['A']; summary.append(item)
(b / 'comparaison.json').write_text(json.dumps(summary, indent=2)); budget()
(b / 'resultat.json').write_text(json.dumps(dict(samples=96, verdict='mesure complète sans seuil',
    quiescence='à confirmer par confineur', echeance=end, monotonic=time.monotonic())))
