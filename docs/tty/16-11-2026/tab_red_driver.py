"""One Rust test through actual guarded Cargo, spy catalogue callback, no network server."""
import json,os,re,subprocess,sys,time
from pathlib import Path
from strict_target_guard import digest,snapshot,scan,require_source,require_hashes,require_provenance
q=Path(__file__).parent;c=json.loads((q/'prepared.json').read_text());a=json.loads((q/'allocation.json').read_text())
w,t,p=Path(c['worktree']),Path(c['target']),Path(c['proofs']);old_w=Path(c['old_worktree'])
p.mkdir(mode=0o700)
subprocess.run([sys.executable,str(q/'confiner_validation.py'),'verifier'],check=True)
receipt=Path(os.environ['RAG3WEAVER_VALIDATION_RECU']);(p/'authority-path.json').write_text(json.dumps(dict(path=str(receipt),end=a['end'])))
require_source(w,c['source']);require_hashes(c['controllers']);assert digest(q/'target-ledger.json')==c['ledger_sha']
require_provenance(t,json.loads((q/'target-ledger.json').read_text()))
assert digest(q/'historical-tty-elf')==c['old_elf_sha'] and digest(q/'old-local-fingerprints.tar')==c['old_fingerprints_sha']
previous=json.loads((q/'previous-autorite.json').read_text());verdict=json.loads((q/'previous-verdict.json').read_text())
assert previous['etat']=='termine' and verdict['restes']==[] and verdict['nettoye']
quiet=scan('/proc',[w,old_w,t]);(p/'quiet-before.json').write_text(json.dumps(quiet,indent=2));assert quiet['found']==[]
for name in ['cache','state','fixture','data','config']:(p/name).mkdir(mode=0o700)
tmp=Path.home()/'.cache/ttytab1';tmp.mkdir(mode=0o700)
env={key:os.environ[key] for key in ['HOME','USER','LOGNAME','PATH','SHELL','LANG','LC_ALL','TZ','CARGO_HOME','RUSTUP_HOME',
     'DBUS_SESSION_BUS_ADDRESS','RAG3WEAVER_VALIDATION_RECU','RAG3WEAVER_VALIDATION_JETON'] if key in os.environ}
env.update(CARGO_TARGET_DIR=str(t),CARGO_BUILD_JOBS='4',CARGO_INCREMENTAL='0',CARGO_NET_OFFLINE='true',CARGO_TERM_COLOR='never',
           RUST_TEST_THREADS='1',RAG3WEAVER_TARGET_REPRENDRE='1',RAG3WEAVER_TARGET_MAX_GO='32',TMPDIR=str(tmp),
           XDG_CACHE_HOME=str(p/'cache'),XDG_STATE_HOME=str(p/'state'),XDG_DATA_HOME=str(p/'data'),XDG_CONFIG_HOME=str(p/'config'),
           RAG3WEAVER_STATE_DIR=str(p/'fixture'))
command=[sys.executable,'-B',str(q/'cargo_garde.py'),'test','--offline','--locked','-j4','--lib',c['filter'],'--','--exact','--nocapture']
(p/'command.json').write_text(json.dumps(command))
with (p/'cargo.log').open('x') as output:
    result=subprocess.run(command,cwd=w/'extension/rag3weaver/tty',env=env,stdout=output,stderr=subprocess.STDOUT)
(p/'cargo.rc').write_text(str(result.returncode))
log=(p/'cargo.log').read_text()
compiled=f'Compiling rag3weaver-tty v0.1.0 ({w}/extension/rag3weaver/tty)' in log
expected=result.returncode==101 and compiled and 'running 1 test' in log and 'catalogue request for /task mes' in log and 'test result: FAILED. 0 passed; 1 failed; 0 ignored;' in log
record=dict(verdict='ROUGE_ASSERTION_ATTENDUE' if expected else 'REFUS_OU_RESULTAT_INATTENDU',rc=result.returncode,
            compiled_from_new_worktree=compiled,filter=c['filter'],no_live_http_server_required=True)
match=re.search(r'Running unittests src/lib.rs \(([^\n]+)\)',log)
if match:
    elf=Path(match.group(1));assert elf.is_relative_to(t)
    record.update(test_elf=str(elf),test_elf_sha256=digest(elf))
(p/'test-result.json').write_text(json.dumps(record,indent=2))
(p/'target-after.json').write_text(json.dumps(snapshot(t),indent=2))
quiet=scan('/proc',[w,old_w,t]);(p/'quiet-driver-after.json').write_text(json.dumps(quiet,indent=2));assert quiet['found']==[]
require_source(w,c['source']);subprocess.run([sys.executable,str(q/'confiner_validation.py'),'verifier'],check=True)
assert time.monotonic()<a['end']
raise SystemExit(1 if expected else 2)
