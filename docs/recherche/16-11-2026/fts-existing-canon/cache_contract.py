"""Read and compare the exact prior cache provenance; never adopt or mutate it."""
import json,os,stat,time
from pathlib import Path

def identity(path):
    value=Path(path).lstat()
    return dict(device=value.st_dev,inode=value.st_ino,mode=value.st_mode,uid=value.st_uid)

def capture_cache_contract(config,git,digest):
    record=dict(observed=time.monotonic())
    try:
        target=Path(config['target']);previous=Path(config['previous_archive']);old=Path(config['previous_worktree'])
        record['target']=identity(target)
        marker=target/'.arbre-du-target'
        record['marker']=dict(identity=identity(marker),owner=marker.read_text().strip())
        record['previous_files']={name:digest(previous/name) for name in
            ['supervisor/autorite.json','supervisor/verdict.json','driver/ledger.json','manifest.json']}
        manifest=json.loads((previous/'manifest.json').read_text())
        record['manifest_entries']={name:manifest[name] for name in ['supervisor/autorite.json','supervisor/verdict.json','driver/ledger.json']}
        authority=Path(config['previous_authority']);verdict=authority.parent/'verdict.json'
        record['live_previous_files']={name:dict(identity=identity(path),sha256=digest(path)) for name,path in [('authority',authority),('verdict',verdict)]}
        auth=json.loads(authority.read_text())
        record['authority']={key:auth.get(key) for key in ['etat','pid','debut','echeance']}
        record['scope']=json.loads(verdict.read_text())
        ledger=json.loads((previous/'driver/ledger.json').read_text());elf=Path(ledger['artifact']['executable'])
        record['elf']=dict(path=str(elf),identity=identity(elf),sha256=digest(elf),ledger_sha256=ledger['elf_sha256'])
        record['old_worktree']=dict(head=git(old,'rev-parse','HEAD'),status=git(old,'status','--porcelain'))
    except BaseException as error:
        record['capture_error']=repr(error)
    return record

def verify_cache_contract(record,expected):
    assert 'capture_error' not in record,record.get('capture_error')
    for key in ['target','marker','previous_files','manifest_entries','live_previous_files','authority','scope','elf','old_worktree']:
        assert record[key]==expected[key],'changed cache contract: '+key
    assert stat.S_ISDIR(record['target']['mode']) and record['target']['uid']==os.getuid()
    assert stat.S_ISREG(record['marker']['identity']['mode']) and record['marker']['identity']['uid']==os.getuid()
    assert stat.S_ISREG(record['elf']['identity']['mode']) and record['elf']['sha256']==record['elf']['ledger_sha256']
    for name,value in record['manifest_entries'].items():
        assert record['previous_files'][name]==value,'qualified manifest mismatch: '+name
    for name,archived in [('authority','supervisor/autorite.json'),('verdict','supervisor/verdict.json')]:
        live=record['live_previous_files'][name]
        assert stat.S_ISREG(live['identity']['mode']) and live['identity']['uid']==os.getuid()
        assert live['sha256']==record['previous_files'][archived]
    assert record['authority']['etat']=='termine'
    scope=record['scope'];assert scope['verdict']=='VERT' and scope['code']==0 and not scope['restes'] and scope['nettoye']
    assert record['old_worktree']['head']=='04634cfed667711192efd313ff1418ef8dfd5c26' and not record['old_worktree']['status']

def publish_composed(path,record,deadline,now=time.monotonic):
    with Path(path).open('x') as stream:
        if record['verdict']=='VERT' and now()>=deadline:
            record=dict(verdict='ROUGE',error='deadline reached immediately before publication')
        json.dump(record,stream,indent=2)
    return record
