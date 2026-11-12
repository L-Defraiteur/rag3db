from pathlib import Path
import hashlib,json,os,sys
R=Path('/home/lucied/.cache/rag3weaver-build');P=R/'cx-publication-composite-cible-v1';W=R/'cx-wt-publication-composite';T=R/'cx-target-contenu-derive'
source=sys.argv[1];rc=int(sys.argv[2]);record=dict(source=source,client_rc=rc,verdict='ROUGE')
try:
 assert rc==0,'superviseur non réussi'
 result=json.loads((P/'resultat.json').read_text());assert result['verdict']=='VERT' and result['source']==source
 info=json.loads((P/'source.json').read_text());receipt=Path(info['supervision_receipt'])
 assert receipt.parent.is_relative_to(R/'cx-publication-composite-confinement-v1')
 supervisor=json.loads((receipt.parent/'verdict.json').read_text())
 assert supervisor['verdict']=='VERT' and supervisor['code']==0 and supervisor['restes']==[] and supervisor['nettoye']
 assert not Path(supervisor['prive']).exists(),'bus commun restant'
 artifact=json.loads((P/'binaire.json').read_text())
 with (P/'binaire-tests').open('rb') as f: assert hashlib.file_digest(f,'sha256').hexdigest()==artifact['sha256']
 rest=[]
 for p in Path('/proc').iterdir():
  if not p.name.isdigit() or int(p.name)==os.getpid():continue
  try:
   if p.stat().st_uid!=os.getuid() or (p/'stat').read_text().rsplit(') ',1)[1].split()[0]=='Z':continue
   exe=os.readlink(p/'exe');cwd=os.readlink(p/'cwd')
   if exe.startswith(str(T)+'/') or cwd==str(W) or cwd.startswith(str(W)+'/'):rest.append(dict(pid=p.name,exe=exe,cwd=cwd))
  except OSError:continue
 assert not rest,rest
 record.update(verdict='VERT',tests=result['tests'],supervisor=supervisor,own_processes=rest,elf_rehash=True)
except BaseException as error:record['error']=repr(error)
finally:
 P.mkdir(exist_ok=True)
 (P/'VERDICT_COMPOSE').write_text(record['verdict']+'\n')
 (P/'verdict-compose.json').write_text(json.dumps(record,indent=2)+'\n')
 print(json.dumps(record),flush=True)
sys.exit(0 if record['verdict']=='VERT' else 1)
