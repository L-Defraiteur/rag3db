from pathlib import Path
import hashlib,json,os,shutil,sys,time
R=Path('/home/lucied/.cache/rag3weaver-build');P=R/'cx-passe-publication-composite-v1';W=R/'cx-wt-publication-composite';T=R/'cx-target-contenu-derive'
SOURCE=sys.argv[1];client_rc=int(sys.argv[2]);P.mkdir(exist_ok=True)
record=dict(source=SOURCE,verdict='ROUGE',client_rc=client_rc);exit_code=1
try:
    assert client_rc==0,'superviseur non réussi'
    assert (P/'VERDICT').read_text().strip()=='VERT'
    source=json.loads((P/'source.json').read_text());assert source['source']==SOURCE
    receipt=Path(source['supervision_receipt']);assert receipt.parent.is_relative_to(R/'cx-publication-composite-global-confinement')
    supervisor=json.loads((receipt.parent/'verdict.json').read_text())
    assert supervisor['verdict']=='VERT' and supervisor['code']==0 and supervisor['restes']==[] and supervisor['nettoye']
    assert not Path(supervisor['prive']).exists(),'bus commun restant'
    quiet=json.loads((P/'quiescence.json').read_text());navquiet=json.loads((P/'nav-quiescence.json').read_text())
    assert not quiet['live'] and not quiet['context'] and not navquiet['live'] and not navquiet['context']
    tmpinfo=json.loads((P/'tmp-navigateur.json').read_text());tmp=Path(tmpinfo['path'])
    assert tmp.parent==Path('/var/tmp') and tmp.name.startswith('cx-pub.') and not tmp.is_symlink()
    assert (tmp/'.possede-publication-composite').read_text().strip()==SOURCE
    leftover=[]
    for q in Path('/proc').iterdir():
        if not q.name.isdigit() or int(q.name)==os.getpid():continue
        try:
            if q.stat().st_uid!=os.getuid():continue
            stat=(q/'stat').read_text().rsplit(') ',1)[1].split()
            if stat[0]=='Z':continue
            exe=os.readlink(q/'exe');cwd=os.readlink(q/'cwd');args=(q/'cmdline').read_bytes()
            if exe.startswith(str(T)+'/') or cwd==str(W) or cwd.startswith(str(W)+'/') or str(tmp).encode() in args:
                leftover.append(dict(pid=q.name,exe=exe,cwd=cwd))
        except OSError:continue
    assert not leftover,('processus propres restants',leftover)
    artifacts=json.loads((P/'artefacts.json').read_text());assert set(artifacts)=={'lib','navigateur','cli','backend'}
    for label,item in artifacts.items():
        with (P/('binaire-'+label)).open('rb') as f:sha=hashlib.file_digest(f,'sha256').hexdigest()
        assert sha==item['sha256'] and item['source']==SOURCE and item['templates']==220
    shutil.rmtree(tmp);assert not tmp.exists()
    (P/'tmp-retire.json').write_text(json.dumps(dict(path=str(tmp),absent=True,source=SOURCE))+'\n')
    elapsed=time.monotonic()-source['initial_monotonic'];assert elapsed<5400,'échéance initiale dépassée'
    record.update(verdict='VERT',supervisor=supervisor,quiescence_scan=leftover,elf_rehash=4,tmp_removed=True,elapsed_s=elapsed)
    exit_code=0
except BaseException as error:record['error']=repr(error)
finally:
    (P/'VERDICT_COMPOSE').write_text(record['verdict']+'\n')
    (P/'verdict-compose.json').write_text(json.dumps(record,indent=2)+'\n')
    print(json.dumps(record),flush=True)
sys.exit(exit_code)
