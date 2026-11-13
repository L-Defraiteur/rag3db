"""Preparation: invoke ONLY inside the separately authorized common confinement."""
import argparse, json, os, re, subprocess
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
(out / 'entree.json').write_text(json.dumps(dict(source=source, cgroup=cg, memoire=mem), indent=2))

def run(args, name):
    with (out / name).open('w') as f:
        result = subprocess.run(args, stdout=f, stderr=subprocess.STDOUT)
    print(name, 'rc', result.returncode, flush=True)
    if result.returncode: raise SystemExit(result.returncode)
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
(out / 'resultat.json').write_text(json.dumps(dict(source=source, verdict='VERT', core_exacts=6)))
