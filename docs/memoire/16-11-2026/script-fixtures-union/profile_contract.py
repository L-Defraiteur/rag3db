"""Fresh short disk TMP and strict same-owner carry; no adoption or Cargo here."""
from pathlib import Path
import hashlib,json,os,secrets,stat,subprocess,time
import source_proof

PRIOR_END=1186775.678574093
OLD_PILOT='f812f1fd8024aac0ab1a70e2633d75e326c5263b'
CANONICAL_SHA='0dddc68c959407fc97c88f024b1911fa9dcd1c718a738175b8f9f3d2e99076ca'
MANIFEST_SHA='da809b576a3b5e53dde0255c8c0f50f8f62c0fd196fec374a6814caafebeb618'
OLD_ELF_SHA='6f12273abeb4a957dd5abdcd0f518ee673607146eefbe2c2938769d9bfdf8e74'

def require(value,message):
    if not value:raise RuntimeError(message)

def identity(path):
    path=Path(path);s=path.lstat()
    require(path.resolve()==path,'noncanonical or symbolic path')
    return dict(path=str(path),device=s.st_dev,inode=s.st_ino,mode=s.st_mode,uid=s.st_uid)

def check_short(path):
    path=Path(path);record=identity(path)
    require(stat.S_ISDIR(record['mode']) and record['uid']==os.getuid() and stat.S_IMODE(record['mode'])==0o700,'TMP not private owned directory')
    # Bound PID and counter textual widths rather than relying on today's PID.
    longest=path/'pont-9999999999-999999/ecoute/pont.sock'
    require(len(os.fsencode(longest))<=107,'TMP socket path exceeds UTF8 byte bound')
    record['longest_socket_bytes']=len(os.fsencode(longest));return record

def create_short(root):
    root=Path(root);before=identity(root)
    require(stat.S_ISDIR(before['mode']) and before['uid']==os.getuid(),'disk root ownership/type')
    fs=subprocess.check_output(['findmnt','-n','-o','FSTYPE','-T',str(root)],text=True).strip()
    require(bool(fs) and fs not in ['tmpfs','ramfs'],'TMP must be on disk')
    path=root/('m-'+secrets.token_hex(5));os.mkdir(path,0o700)
    require(identity(root)==before and not list(path.iterdir()),'root changed or new TMP not empty')
    record=check_short(path);require(record['device']==before['device'],'TMP changed filesystem')
    record['filesystem']=fs;return record

def check_same_short(record):
    require(check_short(record['path'])=={k:v for k,v in record.items() if k!='filesystem'},'TMP identity changed')

def capture(cfg,digest,git):
    record=dict(observed=time.monotonic())
    try:
        r=Path(cfg['root']);t=Path(cfg['target']);w=Path(cfg['worktree']);c=w/'extension/rag3weaver'
        oldp=r/'cx-memoire-script-fixtures-qualification-v3';oldq=r/'cx-memoire-script-fixtures-controller-v3';archive=r/'cx-proofs-memoire-script-fixtures-green-v3'
        require(digest(archive/'manifest.json')==MANIFEST_SHA,'prior qualified manifest changed')
        entries={v['path']:v for v in json.loads((archive/'manifest.json').read_text())}
        pairs={'result':(oldp/'composed-result.json','client/composed-result.json'), 'exec-profile':(oldp/'sdk-exec-profile.json','client/sdk-exec-profile.json'), 'profile':(oldp/'profile.json','client/profile.json'),'configuration':(oldq/'configuration.json','controller/configuration.json'), 'canonical':(r/'cx-memoire-script-fixtures-qualification-v2/official/resume.log','carried-canonical/official/resume.log'), 'ledger':(oldp/'ledger-before-second-lib.json','client/ledger-before-second-lib.json')}
        record['prior_files']={}
        for name,(actual,stored) in pairs.items():
            value=digest(actual);require(value==digest(archive/stored)==entries[stored]['sha256'],'prior evidence changed: '+name)
            record['prior_files'][name]=dict(identity=identity(actual),sha256=value)
        qualified=json.loads((oldp/'composed-result.json').read_text());require(qualified['status']=='GREEN' and qualified['source']==source_proof.OLD_APP and qualified['pilot']==OLD_PILOT,'old composition not qualified')
        previous=json.loads((oldq/'configuration.json').read_text());require(previous['app']==source_proof.OLD_APP and previous['pilot']==OLD_PILOT,'wrong prior source/pilot')
        receipt=Path(json.loads((oldp/'authority-path.json').read_text())['path']);a=json.loads(receipt.read_text());v=json.loads((receipt.parent/'verdict.json').read_text())
        require(a['etat']=='termine' and a['echeance']==1186772.020708913 and a['echeance']<=PRIOR_END,'prior authority active or different END')
        require(v['verdict']=='VERT' and v['code']==0 and not v['restes'] and v['nettoye'] and not Path(v['prive']).exists(),'prior scope not quiet red')
        record['authority']=dict(identity=identity(receipt),sha256=digest(receipt),state=a['etat'],deadline=a['echeance'])
        record['verdict']=dict(identity=identity(receipt.parent/'verdict.json'),sha256=digest(receipt.parent/'verdict.json'))
        require(record['verdict']['sha256']==digest(archive/'supervisor/verdict.json')==entries['supervisor/verdict.json']['sha256'],'prior scope archive differs')
        record['target']=identity(t);marker=t/'.arbre-du-target';record['marker']=dict(identity=identity(marker),sha256=digest(marker),owner=marker.read_text().strip())
        require(stat.S_ISDIR(record['target']['mode']) and record['target']['uid']==os.getuid(),'target ownership/type')
        require(stat.S_ISREG(record['marker']['identity']['mode']) and record['marker']['owner']==str(c),'wrong target marker')
        canonical=r/'cx-memoire-script-fixtures-qualification-v2/official/resume.log';require(digest(canonical)==CANONICAL_SHA and canonical.read_text().splitlines()[-1]=='VERT','canonical qualification changed')
        ledger=json.loads((archive/'client/ledger-before-second-lib.json').read_text());binary=Path(ledger['binary'])
        record['elf']=dict(identity=identity(binary),sha256=digest(binary));require(record['elf']['sha256']==ledger['sha256']==digest(archive/'client/large-unit-tests')==OLD_ELF_SHA,'prior real ELF differs')
        record['canonical']=dict(path=str(canonical),sha256=digest(canonical),ledger_first=ledger['first'])
        require(ledger['source']==source_proof.OLD_APP and ledger['pilot']==OLD_PILOT and ledger['canonical_summary_sha256']==record['canonical']['sha256'],'prior canonical ledger mismatch')
        source_proof.verify_archive(json.loads((source_proof.TRANSITION/'frozen-source.json').read_text()))
        record['transition']=dict(sha256=digest(source_proof.TRANSITION/'transition.json'),archive_sha256=digest(source_proof.TRANSITION/'frozen-source.json'),new_source_sha256=digest(source_proof.TRANSITION/'new-source.json'))
        record['source']=dict(head=git(w,'rev-parse','HEAD'),status=git(w,'status','--porcelain'));require(record['source']==dict(head=cfg['app'],status=''),'source differs')
        fingerprints=[]
        for directory in sorted(t.rglob('.fingerprint')):
            for file in sorted(directory.rglob('*')):
                if file.is_file():fingerprints.append([str(file.relative_to(t)),digest(file)])
        record['fingerprints']=fingerprints
        probe=subprocess.run(['bash','-c','. "$1"; build_rs_d_un_autre_arbre "$2" "$3"','read-only-buildrs',str(c/'scripts/garde_target.sh'),str(t),str(c)],capture_output=True,text=True,timeout=20)
        record['buildrs_probe']=dict(rc=probe.returncode,stdout=probe.stdout,stderr=probe.stderr)
        require(probe.returncode==1 and not probe.stdout,'foreign build.rs output present')
    except BaseException as error:record['error']=repr(error)
    return record

def verify(record,expected):
    require('error' not in record,record.get('error','capture failed'))
    require({k:v for k,v in record.items() if k!='observed'}=={k:v for k,v in expected.items() if k!='observed'},'carry changed while waiting for poste')
