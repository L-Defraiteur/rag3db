"""Pinned old evidence and exact expected failures; no Cargo or target mutation."""
from pathlib import Path
import hashlib, json, os, re, stat, subprocess
import cache_state
import source_proof as source

R=source.R
W=source.W
C=W/'extension/rag3weaver'
SESSION=R/'target-cx-archi-pas-va-au-bout-r2'
T=SESSION/'native-doc-ea670-v1'
GREEN=R/'cx-archi-doc-supervisor-green-r1'
GREEN_Q=R/'cx-archi-doc-supervisor-controller-green-r1'
RED=R/'cx-archi-doc-supervisor-empty-r1'
ARCHIVE=R/'cx-proofs-doc-supervisor-green-v1'
MANIFEST='fa7f619343b3d2c635e5b81a8412e30810dbee588279f852f26831343a01c584'
GREEN_ELF='734f6547e22bcdbef95bae0956cf68a7df20654ccc82092df63861bd1922bffa'
EA_TRANSITION=R/'cx-doc-green-transition-r1'
NAMES={
    'lib':'daemon_host::teammate_tests::task_fixture_uses_the_declared_private_sandbox_before_add',
    'e2e':'documentation_fixture_uses_the_declared_private_sandbox_before_add',
}
CHILD_MARKER='DOCUMENTATION_PRIVATE_SANDBOX_NOT_CONNECTED_BEFORE_ADD_FILES'
PARENT_MARKER='DOCUMENTATION_PRIVATE_SANDBOX_SETUP_REFUSED'

def sha(path):
    with Path(path).open('rb') as stream:return hashlib.file_digest(stream,'sha256').hexdigest()

def save(path, value):
    with Path(path).open('x') as stream:
        json.dump(value,stream,indent=2);stream.write('\n');stream.flush();os.fsync(stream.fileno())

def archived_green(require_original=True):
    assert sha(ARCHIVE/'manifest.json')==MANIFEST
    rows=json.loads((ARCHIVE/'manifest.json').read_text());assert len(rows)==36
    for row in rows:
        file=ARCHIVE/row['path'];assert file.resolve().is_relative_to(ARCHIVE)
        assert file.stat().st_size==row['size'] and sha(file)==row['sha256']
    authority_path=Path(json.loads((GREEN/'authority-path.json').read_text())['path'])
    auth=json.loads(authority_path.read_text());scope=json.loads((authority_path.parent/'verdict.json').read_text())
    assert auth['etat']=='termine' and scope['verdict']=='VERT' and scope['code']==0
    assert not scope['restes'] and scope['nettoye'] and not Path(scope['prive']).exists()
    result=json.loads((GREEN/'result.json').read_text())
    assert result['status']=='GREEN_TARGETED_ONLY' and result['not_a_full_qualification']
    ledger=json.loads((GREEN/'ledger-before-witness.json').read_text())
    assert ledger['source']==source.OLD_APP and ledger['sha256']==GREEN_ELF
    assert sha(GREEN/'native-fixture-elf')==GREEN_ELF
    if require_original:assert sha(Path(ledger['binary']))==GREEN_ELF
    # The original ELF under T may be rebuilt later. Its two archived copies must not change.
    indexed={row['path']:row for row in rows}
    for name in ['authority-path.json','result.json','ledger-before-witness.json','native-fixture-elf']:
        assert sha(GREEN/name)==indexed['client/'+name]['sha256']
    assert sha(authority_path.parent/'verdict.json')==indexed['supervisor/verdict.json']['sha256']
    assert sha(authority_path.parent/'enfants.json')==indexed['supervisor/enfants.json']['sha256']
    assert sha(GREEN_Q/'configuration.json')==indexed['controller/configuration.json']['sha256']
    old_cfg=json.loads((GREEN_Q/'configuration.json').read_text())
    assert old_cfg['app']==source.OLD_APP and old_cfg['pilot']=='b12ee2a65af9d60b6264b0ececa07993fdff196a'
    assert source.git(Path(old_cfg['control']),'rev-parse','HEAD')==old_cfg['pilot']
    assert not source.git(Path(old_cfg['control']),'status','--porcelain')
    assert sha(EA_TRANSITION/'frozen-source.json')==indexed['transition/frozen-source.json']['sha256']
    record=json.loads((EA_TRANSITION/'frozen-source.json').read_text())
    assert record['head']=='ea67054a11d73e06cf44c38d9ce941a754c0190d'
    source.verify_archive(record)
    return dict(authority_path=str(authority_path),authority_sha256=sha(authority_path),
                old_cgroup=auth['cgroup'],archive_manifest=MANIFEST,old_elf=GREEN_ELF,
                children_sha256=sha(authority_path.parent/'enfants.json'))

def owned_gone(authority_path):
    rows=json.loads((Path(authority_path).parent/'enfants.json').read_text());alive=[];unknown=[]
    for row in rows:
        try:
            fields=(Path('/proc')/str(row['pid'])/'stat').read_text().rsplit(') ',1)[1].split()
            if fields[19]==str(row['debut']) and fields[0]!='Z':alive.append(row)
        except (FileNotFoundError,ProcessLookupError):continue
        except OSError as error:unknown.append(dict(pid=row['pid'],error=repr(error)))
    assert not alive and not unknown,dict(alive=alive,unknown=unknown)
    return dict(observations=len(rows),alive=alive,unknown_owned=unknown)

def cache_contract():
    identity=json.loads((RED/'new-cache-identity.json').read_text())
    cache_state.require_identity(T,identity)
    assert (T/'.arbre-du-target').read_text().strip()==str(C)
    frozen=json.loads((RED/'frozen-old-cache.json').read_text())
    cache_state.verify_frozen(SESSION,T,frozen)
    cache_state.require_size(SESSION)
    return dict(identity=identity,owner=str(C),frozen_old_sha256=sha(RED/'frozen-old-cache.json'),
                active_inventory=cache_state.inventory(T,None))

def expected_red(name, rc, text):
    assert name in NAMES.values() and rc==101, 'wrong witness or exit status'
    assert CHILD_MARKER in text and PARENT_MARKER in text, 'not the missing setup raccord'
    summaries=re.findall(r'^test result: FAILED\. (\d+) passed; (\d+) failed; (\d+) ignored;',text,re.M)
    assert summaries==[('0','1','0'),('0','1','0')], 'parent AND same-ELF child must actually fail'
    assert len(re.findall(r'panicked at',text))==2, 'unexpected extra panic'
    # --nocapture can interleave stderr after the harness prefix and print a bare FAILED later.
    headers=re.findall(r'^test (.+?) \.\.\. ',text,re.M)
    assert headers==[name,name], 'another test ran or the exact did not run'
    failed=re.findall(r'^(?:test .* \.\.\. )?FAILED$',text,re.M)
    assert len(failed)==2, 'parent and child did not both report failure'
    fixtures=re.findall(r'(?:^|\s)DOCUMENTATION_FAILED_FIXTURE ([^\n]+)',text,re.M)
    assert len(fixtures)==1, 'missing preserved parent fixture'
    return dict(name=name,exit_code=rc,parent=dict(passed=0,failed=1,ignored=0),
                child=dict(passed=0,failed=1,ignored=0),fixture=fixtures[0],
                distinct_judges=1,raw_test_invocations=2)

def private_environment(inherited, proof, tmp, poste_fils):
    jobs=min(8,int(poste_fils));assert jobs>0
    names=['HOME','USER','LOGNAME','PATH','LANG','LC_ALL','CARGO_HOME','RUSTUP_HOME',
           'DBUS_SESSION_BUS_ADDRESS','RAG3WEAVER_VALIDATION_RECU','RAG3WEAVER_VALIDATION_JETON']
    env={k:inherited[k] for k in names if k in inherited}
    env.update(POSTE_FILS=str(poste_fils),CARGO_BUILD_JOBS=str(jobs),RAYON_NUM_THREADS=str(jobs),
        RUST_TEST_THREADS='1',CARGO_MANIFEST_DIR=str(C),CARGO_TARGET_DIR=str(T),CARGO_INCREMENTAL='0',
        CARGO_TERM_COLOR='never',RUST_BACKTRACE='1',TMPDIR=str(tmp),TMP=str(tmp),TEMP=str(tmp),
        XDG_CONFIG_HOME=str(proof/'config'),XDG_DATA_HOME=str(proof/'data'),XDG_CACHE_HOME=str(proof/'cache'),
        XDG_STATE_HOME=str(proof/'xdg-state'),RAG3WEAVER_STATE_DIR=str(proof/'state-fixture'),
        RAG3WEAVER_CONFIG_DIR=str(proof/'config'),RAG3WEAVER_DATA_DIR=str(proof/'data'),
        RAG3WEAVER_DAEMON_DIR=str(proof/'daemon'),RAG3WEAVER_JOURNAUX_DIR=str(proof/'journals'),
        RAG3WEAVER_BUILTIN_TEMPLATES_DIR=str(proof/'templates'),RAG3WEAVER_CACHE_DIR=str(proof/'cache'))
    for suffix in ['', '_EMBEDDER','_RERANKER','_OCR']:env['RAG3WEAVER_BURN_DEVICE'+suffix]='cpu'
    assert not any(k.startswith('GOOGLE_') or k.endswith('_API_KEY') for k in env)
    assert Path(env['XDG_STATE_HOME']).parent==Path(env['RAG3WEAVER_STATE_DIR']).parent
    return env
