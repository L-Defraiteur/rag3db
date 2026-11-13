"""Preparation: invoke ONLY inside the separately authorized common confinement."""
import argparse, json, os, re, signal, subprocess, time
from pathlib import Path

p = argparse.ArgumentParser()
p.add_argument('--target', required=True)
p.add_argument('--logs', required=True)
a = p.parse_args()
source = '3b1661446c573ee794b918ec016c0cbff748e4ce'
assert subprocess.check_output(['git', 'rev-parse', 'HEAD'], text=True).strip() == source
assert not subprocess.check_output(['git', 'status', '--porcelain'], text=True).strip()
cg = Path('/proc/self/cgroup').read_text().split('0::', 1)[1].strip()
mem = {n: (Path('/sys/fs/cgroup') / cg.lstrip('/') / n).read_text().strip()
       for n in ['memory.max', 'memory.high']}
assert mem == {'memory.max': '8589934592', 'memory.high': 'max'}, mem
out = Path(a.logs); out.mkdir(parents=True, exist_ok=False)
os.environ['CARGO_TARGET_DIR'] = str(Path(a.target).resolve())
os.environ['CARGO_BUILD_JOBS'] = '4'
os.environ['CARGO_TERM_COLOR'] = 'never'
(out / 'entree.json').write_text(json.dumps(dict(source=source, cgroup=cg, memoire=mem), indent=2))

# The common supervisor owns this receipt. Never mint/extend an authority here.
receipt_path = Path(os.environ['RAG3WEAVER_VALIDATION_RECU'])
receipt = json.loads(receipt_path.read_text())
end = receipt['echeance']
assert receipt['etat'] == 'actif' and 0 < end - time.monotonic() <= 900
(out / 'budget.json').write_text(json.dumps(dict(echeance=end, restant=end-time.monotonic())))

def first_red(reason, **details):
    # Exclusive creation: subsequent cleanup errors cannot repaint the cause.
    marker = out / 'premier-rouge.json'
    if not marker.exists():
        with marker.open('x') as f:
            json.dump(dict(raison=reason, monotonic=time.monotonic(), **details), f, indent=2)
            f.flush(); os.fsync(f.fileno())
        (out / 'resultat.json').write_text(json.dumps(dict(verdict='ROUGE', raison=reason)))

def budget():
    live = json.loads(receipt_path.read_text())
    assert live['etat'] == 'actif' and live['echeance'] == end
    if end - time.monotonic() <= 5:
        first_red('budget global épuisé avant commande')
        raise SystemExit(2)

def pid_start(pid):
    try: return Path(f'/proc/{pid}/stat').read_text().rsplit(') ', 1)[1].split()[19]
    except FileNotFoundError: return None

def stop_owned(child, start):
    # The new session/group belongs solely to this invocation. Keep root
    # unreaped until the identity check; common supervisor owns any escapees.
    if pid_start(child.pid) == start:
        try: os.killpg(child.pid, signal.SIGTERM)
        except ProcessLookupError: pass
    try: child.wait(timeout=2)
    except subprocess.TimeoutExpired:
        if pid_start(child.pid) == start:
            try: os.killpg(child.pid, signal.SIGKILL)
            except ProcessLookupError: pass
        child.wait(timeout=2)

# Same libtest FAILED detector as qualified driver65ad, plus final summary.
# Standalone FAILED covers nocapture splitting the status from the test prefix.
failed = re.compile(r'^test .* \.\.\. FAILED$|^\s*FAILED\s*$|^test result: FAILED\.', re.M)

def run(args, name):
    budget()
    with (out / name).open('w') as f:
        child = subprocess.Popen(args, stdout=subprocess.PIPE, stderr=subprocess.STDOUT,
                                 text=True, start_new_session=True)
        start = pid_start(child.pid)
        assert start is not None
        (out / (name + '.pid.json')).write_text(json.dumps(dict(pid=child.pid, start=start, argv=args)))
        red = False
        for line in child.stdout:
            f.write(line); f.flush()
            if failed.match(line.rstrip('\n')):
                first_red('FAILED libtest', commande=args, log=name, ligne=line.rstrip(),
                          pid=child.pid, start=start)
                red = True
                stop_owned(child, start)
                break
        child.stdout.close()
        code = child.wait()
    print(name, 'rc', code, flush=True)
    if red or code:
        first_red('code commande non nul', commande=args, log=name, code=code,
                  pid=child.pid, start=start)
        raise SystemExit(code if code > 0 else 1)
    budget()
    return (out / name).read_text()

def inventory(package):
    base = ['cargo', 'test', '-j4', '-p', package, '--lib']
    run(base + ['--no-run'], package + '-compilation.log')
    listing = run(base + ['--', '--list'], package + '-inventaire.log')
    names = [line[:-6] for line in listing.splitlines() if line.endswith(': test')]
    assert names and len(names) == len(set(names)), 'inventory missing/duplicated'
    return base, names

base, names = inventory('luciole')
log = run(base + ['--', '--nocapture'], 'luciole-lib-entiere.log')
summary = re.search(r'test result: ok\. (\d+) passed; 0 failed; 0 ignored;', log)
assert summary and int(summary[1]) == len(names), 'lib inventory/count mismatch'
base, names = inventory('lucivy-core')
selected = ['a_failed_close_still_stops_its_pools', 'test_drop_commits_the_writes_still_queued',
            'test_close_and_reopen', 'test_writes_to_a_shard_without_writer_are_reported_by_commit',
            'successful_close_releases_router_and_shards_at_return',
            'failed_close_releases_router_and_shards_at_return']
for short in selected:
    name = 'sharded_handle::tests::' + short
    assert names.count(name) == 1, name
    log = run(base + [name, '--', '--exact', '--nocapture'], short + '.log')
    assert 'running 1 test' in log
    assert re.search(r'test result: ok\. 1 passed; 0 failed; 0 ignored;', log)
# Hash evidence before checking the same global deadline one last time.
import hashlib
manifest = {f.name: hashlib.sha256(f.read_bytes()).hexdigest() for f in out.iterdir() if f.is_file()}
(out / 'empreintes.json').write_text(json.dumps(manifest, indent=2))
budget()
(out / 'resultat.json').write_text(json.dumps(dict(source=source, verdict='VERT', core_exacts=6,
    echeance=end, monotonic=time.monotonic(), quiescence='à confirmer par verdict du confineur commun')))
