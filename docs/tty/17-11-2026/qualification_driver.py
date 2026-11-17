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
    end = min(a['end'] - 20, time.monotonic() + cap, deadline or a['end'])
    remaining = math.floor(end - time.monotonic())
    assert remaining > 0
    row = dict(name=name, argv=command, cwd=str(cwd), start=time.monotonic(), end=end)
    (p / (name + '-command.json')).write_text(json.dumps(row, indent=2))
    try:
        with (p / (name + '.log')).open('x') as out:
            completed = subprocess.run(command, cwd=cwd, env=environment, stdout=out,
                                       stderr=subprocess.STDOUT, timeout=remaining)
        row['rc'] = completed.returncode
    finally:
        row['finished'] = time.monotonic()
        (p / (name + '-result.json')).write_text(json.dumps(row, indent=2))
    assert row['rc'] == 0 and row['finished'] < end, row
    return (p / (name + '.log')).read_text()


def results(log, passed=None, allow_ignored=False):
    rows = re.findall(r'test result: ok\. (\d+) passed; 0 failed; (\d+) ignored;', log)
    assert rows, 'no actual Rust result'
    assert any(int(n) > 0 and (passed is None or int(n) == passed) for n, ignored in rows)
    assert allow_ignored or all(int(ignored) == 0 for n, ignored in rows)
    assert not re.search(r'^test result: FAILED|^error(?:\[|:)|^SAUTÉ|NON JOUÉES', log, re.M)


canonical_end = min(a['end'] - 650, time.monotonic() + 2700)
assert canonical_end > time.monotonic() + 60
grant = contract(q, c, a, a['grant_sha256'], canonical_end)
canonical = Path(c['canonical_proofs'])
assert not canonical.exists() and not canonical.is_symlink()
canon_env = dict(env, CIBLE=t.name, SUITE_DU_TEMOIN='e2e_tty', SUITES='e2e_lectures,e2e_code',
                 RAG3WEAVER_PG=next(row['url'] for row in grant['leases'] if row['role'] == 'postgres'))
(p / 'contract-admission.json').write_text(json.dumps(dict(nonce=grant['nonce'], allocation=a['reference'],
                 admitted=time.monotonic(), phase='canonical', phase_end=canonical_end), indent=2))
stage('canonical', ['bash', str(q/'passe_epinglee.sh'), w.name, canonical.name, '', '-'], q, canon_env, 2700, canonical_end)
assert (canonical/'base').read_text().strip() == c['base']
summary = (canonical/'resume.log').read_text()
assert summary.splitlines()[-1] == 'VERT' and c['source'][:7] in summary.splitlines()[0]
for name, count in [('temoins', 4), ('e2e_lectures', None), ('e2e_code', None), ('navigateur', None)]:
    results((canonical/(name+'.log')).read_text(), passed=count)
results((canonical/'lib.log').read_text(), allow_ignored=True)
assert (canonical/'check_features.log').read_text().splitlines()[-1] == 'CHECK_FEATURES VERT 8/8'
proof = (canonical/'preuve.log').read_text()
assert re.search(r'(Checking|Compiling) rag3weaver ', proof)
assert not re.search(r'^error(?:\[|:)', proof, re.M)
assert c['sdk_id'] in ''.join(path.read_text(errors='replace') for path in canonical.glob('*.log'))
assert (t/'.arbre-du-target').read_text().strip() == str(w/'extension/rag3weaver')
tty = w/'extension/rag3weaver/tty'
results(stage('tty-lib', [sys.executable, '-B', str(q/'cargo_garde.py'), 'test', '--offline', '--locked',
                        '-j4', '--lib'], tty, env, 240))
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
