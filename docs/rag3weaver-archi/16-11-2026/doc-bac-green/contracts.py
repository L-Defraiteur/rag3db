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
    'e2e':'e2e_equipier::documentation_fixture_uses_the_declared_private_sandbox_before_add',
}
PREVIOUS_APP='223b7f55cb502deffe85584898d8fd340fd48518'
PREVIOUS_PILOT='72338e04d7b48549e092bf740f26cdf608c2e4b4'
PREVIOUS_P=R/'cx-archi-doc-bac-red-r2'
PREVIOUS_ARCHIVE=R/'cx-proofs-doc-bac-red-expected-r2'
REPRISE=R/'cx-doc-bac-green-transition-r1'
ARCHIVE049=Path.home()/'git_workspaces/rag3db-cx-archi-doc-049-proof-r1'
ARCHIVE223=Path.home()/'git_workspaces/rag3db-cx-archi-doc-223-proof-r1'
MIDDLE_TRANSITION=R/'cx-doc-bac-registration-reprise-r1'
SIX=['Cargo.toml','Cargo.lock','build.rs','src/lib.rs','src/daemon_host/teammate_tests.rs','tests/common/documentation_fixture.rs']
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




def previous_expected_red(require_original=True):
    manifest=PREVIOUS_ARCHIVE/'manifest.json'
    assert sha(manifest)=='9d82d08f9adc693e1fb4b1c1d762bbe229461f0b1f4451f95bff5101b5c5418f'
    rows=json.loads(manifest.read_text());assert len(rows)==2111
    indexed={row['path']:row for row in rows}
    for row in rows:
        file=PREVIOUS_ARCHIVE/row['path']
        assert file.parent.resolve().is_relative_to(PREVIOUS_ARCHIVE)
        if row['type']=='symlink':
            target=os.readlink(file);assert target==row['link']
            assert hashlib.sha256(os.fsencode(target)).hexdigest()==row['sha256']
        else:assert file.stat().st_size==row['size'] and sha(file)==row['sha256']
    for name in ['result.json','driver-result.json','first-red.json','authority-path.json',
                 'ledger-before-witnesses.json','ledger-published.json','runtime-permitted.json',
                 'expected-red-lib.json','expected-red-e2e.json']:
        assert sha(PREVIOUS_P/name)==indexed['client/'+name]['sha256']
    authority=Path(json.loads((PREVIOUS_P/'authority-path.json').read_text())['path'])
    auth=json.loads(authority.read_text());scope=json.loads((authority.parent/'verdict.json').read_text())
    assert auth['etat']=='termine' and scope['verdict']=='ROUGE' and scope['code']==101
    assert not scope['erreur'] and not scope['restes'] and scope['nettoye'] and not Path(scope['prive']).exists()
    for name in ['verdict.json','enfants.json']:assert sha(authority.parent/name)==indexed['supervisor/'+name]['sha256']
    result=json.loads((PREVIOUS_P/'result.json').read_text())
    assert result['status']=='EXPECTED_SETUP_REDS_OBSERVED' and result['source']==PREVIOUS_APP and result['pilot']==PREVIOUS_PILOT
    assert result['distinct_judges']==2 and result['raw_test_invocations']==4
    assert json.loads((PREVIOUS_P/'first-red.json').read_text())['expected'] is True
    ledger=json.loads((PREVIOUS_P/'ledger-before-witnesses.json').read_text())
    assert ledger['source']==PREVIOUS_APP and ledger['pilot']==PREVIOUS_PILOT
    hashes=dict(lib='0fc11292f774ea870c578dabe52116322a1c33970569a3107fd9d232c66ad7b7',
                e2e='887ee4ff6cf1a540574bba180dd33e79586c9a32d4f716b3baace3941bd2abf2')
    for label,item in ledger['artifacts'].items():
        assert item['sha256']==hashes[label] and sorted(item['features'])==['code','daemon','rag3db-native']
        assert sha(Path(item['copy']))==sha(PREVIOUS_ARCHIVE/('client/'+label+'-elf'))==hashes[label]
        if require_original:assert sha(Path(item['binary']))==hashes[label]
    published=json.loads((PREVIOUS_P/'ledger-published.json').read_text())['observed']
    marker=json.loads((PREVIOUS_P/'runtime-permitted.json').read_text())['observed']
    times=[json.loads((PREVIOUS_P/('expected-red-'+label+'.json')).read_text())['observed'] for label in NAMES]
    assert ledger['created']<published<marker<min(times)<result['final_started']<result['deadline']
    config_path=PREVIOUS_ARCHIVE/'controller/configuration.json'
    config=json.loads(config_path.read_text())
    assert config['app']==PREVIOUS_APP and config['pilot']==PREVIOUS_PILOT
    assert source.git(Path(config['control']),'rev-parse','HEAD')==PREVIOUS_PILOT
    assert not source.git(Path(config['control']),'status','--porcelain')
    for root,key in [(source.TRANSITION,'transition_sha256'),(MIDDLE_TRANSITION,'reprise_sha256')]:
        assert sha(root/'transition.json')==config[key]
    return dict(authority_path=str(authority),authority_sha256=sha(authority),old_cgroup=auth['cgroup'],
                manifest_sha256=sha(manifest),ledger_sha256=sha(PREVIOUS_P/'ledger-before-witnesses.json'),
                elf_sha256=hashes,children_sha256=sha(authority.parent/'enfants.json'))

def expected_green(name,rc,text):
    assert name in NAMES.values() and rc==0,'wrong witness or nonzero result'
    assert 'FAILED' not in text and 'panicked at' not in text,'failure in parent or child'
    summaries=re.findall(r'^test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;',text,re.M)
    assert summaries==[('1','0','0'),('1','0','0')],'parent and child must both judge exactly one test'
    assert re.findall(r'^test (.+?) \.\.\. ',text,re.M)==[name,name],'wrong actual test headers'
    assert len(re.findall(r'(?:^|\s)DOCUMENTATION_SETUP_CHILD_OK '+re.escape(name)+r'$',text,re.M))==1
    rows=re.findall(r'(?:^|\s)DOCUMENTATION_PRIVATE_SANDBOX_COMMITTED ([^\n]+)',text,re.M)
    assert len(rows)==1,'one committed HEAD attestation required'
    row=json.loads(rows[0])
    assert row['name']==name and row['commands']=='approval'
    assert row['manifest_equals_declaration'] is True and row['head_equals_declaration'] is True
    assert re.fullmatch('[0-9a-f]{40}|[0-9a-f]{64}',row['head'])
    assert row['target_token']=='{repo_target}' and row['tmp_token']=='{task_dir}/tmp'
    assert row['cargo_jobs']=='1' and row['offline']=='true'
    targets=Path(row['targets_root']);assert targets.is_absolute() and targets.name=='private-session-targets'
    assert str(targets.parent/'private-controller') in row['extra_read']
    assert Path(row['manifest']).is_relative_to(targets.parent/'setup')
    # Success deletes its TempDir: validate the emitted assertions, never resurrect the fixture.
    return dict(name=name,exit_code=rc,parent=dict(passed=1,failed=0,ignored=0),
                child=dict(passed=1,failed=0,ignored=0),committed=row,distinct_judges=1,raw_test_invocations=2)


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
