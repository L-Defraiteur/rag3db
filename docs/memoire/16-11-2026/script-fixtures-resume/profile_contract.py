"""Fresh short disk TMP and strict same-owner carry; no adoption or Cargo here."""
from pathlib import Path
import hashlib,json,os,secrets,stat,subprocess,time

INITIAL=1179575.678574093
ORIGINAL_END=1186775.678574093
OLD_PILOT='38304227d75e878cceb8ce4f08e2d246e7835bf9'
MANIFEST_SHA='904832a0385f0e3b75c8e588f564094dc33a003844a35707e0d4e3499b955029'
OLD_ELF_SHA='6f12273abeb4a957dd5abdcd0f518ee673607146eefbe2c2938769d9bfdf8e74'

def require(value,message):
    if not value:raise RuntimeError(message)

def remaining(now):
    import math
    value=math.floor(ORIGINAL_END-now)-3
    require(value>60,'original allocation exhausted')
    return value

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
        oldp=r/'cx-memoire-script-fixtures-qualification-v1';oldq=r/'cx-memoire-script-fixtures-controller-v1';archive=r/'cx-proofs-memoire-script-fixtures-red-v1'
        require(digest(archive/'manifest.json')==MANIFEST_SHA,'prior qualified manifest changed')
        entries={v['path']:v for v in json.loads((archive/'manifest.json').read_text())}
        pairs={'first-red':(oldp/'first-red.json','client/first-red.json'), 'watcher-match':(oldp/'watcher-first-match.json','client/watcher-first-match.json'), 'profile':(oldp/'profile.json','client/profile.json'),'configuration':(oldq/'configuration.json','controller/configuration.json')}
        record['prior_files']={}
        for name,(actual,stored) in pairs.items():
            value=digest(actual);require(value==digest(archive/stored)==entries[stored]['sha256'],'prior evidence changed: '+name)
            record['prior_files'][name]=dict(identity=identity(actual),sha256=value)
        previous=json.loads((oldq/'configuration.json').read_text());require(previous['app']==cfg['app'] and previous['pilot']==OLD_PILOT,'wrong prior source/pilot')
        receipt=Path(json.loads((oldp/'authority-path.json').read_text())['path']);a=json.loads(receipt.read_text());v=json.loads((receipt.parent/'verdict.json').read_text())
        require(a['etat']=='termine' and a['echeance']==ORIGINAL_END,'prior authority active or different END')
        require(v['verdict']=='ROUGE' and v['code']==1 and not v['restes'] and v['nettoye'] and not Path(v['prive']).exists(),'prior scope not quiet red')
        record['authority']=dict(identity=identity(receipt),sha256=digest(receipt),state=a['etat'],deadline=a['echeance'])
        record['verdict']=dict(identity=identity(receipt.parent/'verdict.json'),sha256=digest(receipt.parent/'verdict.json'))
        require(record['verdict']['sha256']==digest(archive/'supervisor/verdict.json')==entries['supervisor/verdict.json']['sha256'],'prior scope archive differs')
        record['target']=identity(t);marker=t/'.arbre-du-target';record['marker']=dict(identity=identity(marker),sha256=digest(marker),owner=marker.read_text().strip())
        require(stat.S_ISDIR(record['target']['mode']) and record['target']['uid']==os.getuid(),'target ownership/type')
        require(stat.S_ISREG(record['marker']['identity']['mode']) and record['marker']['owner']==str(c),'wrong target marker')
        ledger=json.loads((archive/'client/red-elf-ledger.json').read_text());binary=Path(ledger['binary'])
        record['elf']=dict(identity=identity(binary),sha256=digest(binary));require(record['elf']['sha256']==ledger['sha256']==digest(archive/'client/large-unit-tests-red')==OLD_ELF_SHA,'prior real ELF differs')
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
