"""Read-only target, private harness; first red stops this Common authority only."""
import json
import os
from pathlib import Path
import signal
import subprocess
import sys
import time
from strict_target_guard import digest, scan, snapshot, require_source, require_provenance, require_hashes

q = Path(__file__).parent
c = json.loads((q / 'prepared.json').read_text())
a = json.loads((q / 'allocation.json').read_text())
w, target, p = Path(c['worktree']), Path(c['target']), Path(c['proofs'])
p.mkdir(mode=0o700)
subprocess.run([sys.executable, str(q / 'confiner_validation.py'), 'verifier'], check=True)
receipt = Path(os.environ['RAG3WEAVER_VALIDATION_RECU'])
(p / 'authority-path.json').write_text(json.dumps(dict(path=str(receipt), end=a['end'])))
require_source(w, c['source'])
require_hashes(c['controllers'])
require_hashes(c['carried'])
require_hashes(c['sdk'])
require_hashes(c['tmux'])
assert digest(q / 'target-ledger.json') == c['ledger_sha']
require_provenance(target, json.loads((q / 'target-ledger.json').read_text()))
assert time.monotonic() < a['end']
quiet = scan('/proc', [w,target])
(p / 'quiet-before.json').write_text(json.dumps(quiet, indent=2))
assert quiet['found'] == []
ports = subprocess.run(['ss','-ltn','sport = :17990'], capture_output=True, text=True)
(p / 'port-check.json').write_text(json.dumps(dict(rc=ports.returncode, stdout=ports.stdout, stderr=ports.stderr)))
assert ports.returncode == 0 and len(ports.stdout.splitlines()) == 1
old = json.loads((q / 'previous-authority.json').read_text())
old_verdict = json.loads((q / 'previous-verdict.json').read_text())
assert old['etat'] == 'termine' and old_verdict['restes'] == [] and old_verdict['nettoye']
for name in ['state','fixture','config','data','cache','harness']:
    (p / name).mkdir(mode=0o700)
tmp = Path.home() / '.cache/ttyhr2'
tmp.mkdir(mode=0o700)
env = {key:os.environ[key] for key in ['HOME','USER','LOGNAME','PATH','SHELL','LANG','LC_ALL','TZ',
       'DBUS_SESSION_BUS_ADDRESS','RAG3WEAVER_VALIDATION_RECU','RAG3WEAVER_VALIDATION_JETON'] if key in os.environ}
env.update(TMPDIR=str(tmp), XDG_STATE_HOME=str(p/'state'), XDG_CONFIG_HOME=str(p/'config'),
           XDG_DATA_HOME=str(p/'data'), XDG_CACHE_HOME=str(p/'cache'),
           RAG3WEAVER_STATE_DIR=str(p/'fixture'), CARGO_MANIFEST_DIR=str(w/'extension/rag3weaver/tty'))
(p / 'environment-keys.json').write_text(json.dumps(sorted(env)))
themes = {path.stem:json.loads(path.read_text()) for path in sorted((w/'extension/rag3weaver/templates/themes').glob('*.json'))}
(p / 'themes.json').write_text(json.dumps(themes))
command = [sys.executable,'-B',str(q/'run_private_harness.py'),str(q/'mesure.py'),c['elf'],
           str(q/'faux_hote.py'),str(p/'harness'),str(p/'themes.json'),str(p/'harness-result.json'),str(p/'state')]
(p / 'harness-command.json').write_text(json.dumps(command))
with (p/'harness.log').open('x') as output:
    child = subprocess.Popen(command, cwd=q, env=env, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
    for line in child.stdout:
        output.write(line)
        output.flush()
        if line.startswith('ROUGE '):
            os.fsync(output.fileno())
            authority = json.loads(receipt.read_text())
            pid = authority['pid']
            assert authority['etat'] == 'actif' and authority['script'] == str(q/'confiner_validation.py')
            fd = os.pidfd_open(pid)
            try:
                info = (Path('/proc')/str(pid)/'stat').read_text().rsplit(') ',1)[1].split()
                assert info[19] == str(authority['debut'])
                assert (Path('/proc')/str(pid)/'cgroup').read_text().strip().endswith(authority['cgroup'])
                with (p/'first-red-stop.json').open('x') as marker:
                    json.dump(dict(cause=line.strip(), observed=time.monotonic(), pid=pid,
                                   start=authority['debut'], action='TERM own Common via verified pidfd'),marker)
                    marker.flush(); os.fsync(marker.fileno())
                signal.pidfd_send_signal(fd,signal.SIGTERM)
            finally:
                os.close(fd)
    rc = child.wait()
(p/'harness.rc').write_text(str(rc))
assert rc == 0
result = json.loads((p/'harness-result.json').read_text())
assert result['verdict'] == 'VERT' and result['rouges'] == []
require_hashes(c['controllers'])
require_hashes(c['sdk'])
require_hashes({c['elf']:c['elf_sha']})
assert snapshot(target) == json.loads((q/'target-ledger.json').read_text())
quiet = scan('/proc',[w,target])
(p/'quiet-driver-final.json').write_text(json.dumps(quiet,indent=2))
assert quiet['found'] == []
subprocess.run([sys.executable,str(q/'confiner_validation.py'),'verifier'],check=True)
assert time.monotonic() < a['end']
(p/'driver-result.json').write_text(json.dumps(dict(verdict='VERT', finished=time.monotonic(),
    scope='HARNESS ONLY; pending outer Common termination and final publication')))
