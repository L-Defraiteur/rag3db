"""Compose completed runtime v3 with the autonomous backend-maps attestation v8.

C7 explicitly authorized this composition instead of repeating passed suites.
The original instrumentation-red verdict is retained; no test is rerun here.
"""
from pathlib import Path
import hashlib,json,mmap,os,shutil,signal,subprocess,sys,time
signal.alarm(30)
R=Path('/home/lucied/.cache/rag3weaver-build');W=R/'cx-wt-publication-composite';C=W/'extension/rag3weaver';T=R/'cx-target-contenu-derive'
OLD=R/'cx-passe-publication-composite-v3';PROBE=R/'cx-publication-composite-probe-maps-v8';P=R/'cx-publication-composite-compose-v1'
SOURCE='626d050d3dcd9316bb8a1fc923bf71fcd0e36baf';RECIPE=sys.argv[1]
def git(*args):return subprocess.check_output(['git',*args],cwd=W,text=True).strip()
def digest(path):
 with Path(path).open('rb') as f:return hashlib.file_digest(f,'sha256').hexdigest()
def load(path):return json.loads(Path(path).read_text())
def save(name,value):(P/name).write_text(json.dumps(value,indent=2)+'\n')
assert git('rev-parse','HEAD')==RECIPE and not git('status','--porcelain')
assert not git('diff','--name-only',SOURCE,RECIPE,'--','extension/rag3weaver','tools/rust_api')
info=load(OLD/'source.json');assert info['source']==SOURCE
assert time.monotonic()<info['deadline_monotonic'],'original 5400s deadline expired'
state=load(OLD/'etat.json');assert state['phase']=='attestation' and state['verdict']=='ROUGE'
assert state['detail']=="('maps runtime manquantes', 'backend')"
assert state['phases'] and all(x['rc']==0 for x in state['phases'])
for name in ['features','minimal','lib','navigateur','cli','postgres','e2e_tx_ligne_a_ligne','e2e_tx_merge_puis_copy','e2e_tx_par_paquet_arret','e2e_tx_vecteurs_apres_rollback']:
 assert any(x['phase']==name for x in state['phases']),name
runtime=load(Path(info['supervision_receipt']).parent/'verdict.json')
assert runtime['verdict']=='ROUGE' and runtime['code']==1 and runtime['erreur'] is None and runtime['restes']==[] and runtime['nettoye']
assert not Path(runtime['prive']).exists()
attestation=load(R/'cx-publication-composite-probe-confinement-v8/validation-kj3l5tap/verdict.json')
assert attestation['verdict']=='VERT' and attestation['code']==0 and attestation['restes']==[] and attestation['nettoye']
assert not Path(attestation['prive']).exists()
annulation=load(R/'cx-publication-composite-global-confinement-v4/validation-a755jpea/verdict.json')
assert annulation['verdict']=='ROUGE' and annulation['restes']==[] and annulation['nettoye']
assert not Path(annulation['prive']).exists()
assert not P.exists();shutil.copytree(OLD,P)
save('historique-verdict-compose-v3.json',load(OLD/'verdict-compose.json'))
artifacts=load(OLD/'artefacts.json');assert set(artifacts)=={'lib','navigateur','cli','backend'}
native=info['native'];files=sorted(f for f in (C/'templates').rglob('*') if f.is_file());assert len(files)==220
maps=load(OLD/'maps.json');probe=load(PROBE/'backend-main.json')
assert probe['sha256']==digest(OLD/'binaire-backend') and probe['exe']==artifacts['backend']['actual_path']
probe_maps=(PROBE/probe['maps']).read_text();assert probe['native'] and all('/'+native+'/librag3db.so' in line for line in probe['native'])
assert not (PROBE/'backend-maps-absent.rag3db').exists()
assert 'exited normally' in (PROBE/'backend-main.log').read_text() and '\nnull\n' in (PROBE/'backend-main.log').read_text()
shutil.copy2(PROBE/probe['maps'],P/probe['maps']);save('backend-main.json',probe)
maps.append(probe)
for label,a in artifacts.items():
 original=OLD/('binaire-'+label);copy=P/('binaire-'+label);before=digest(original)
 assert digest(copy)==before
 with copy.open('rb') as f,mmap.mmap(f.fileno(),0,access=mmap.ACCESS_READ) as data:
  assert data.find(SOURCE[:9].encode())>=0,(label,'source')
  assert all(data.find(f.read_bytes())>=0 for f in files),(label,'templates')
 evidence=[m for m in maps if m['exe']==a['actual_path']];assert evidence,(label,'runtime maps')
 assert all(m['native'] and all('/'+native+'/librag3db.so' in line for line in m['native']) for m in evidence)
 a.update(original_runtime_path=a['actual_path'],actual_path=str(copy),sha256=before,size=copy.stat().st_size,source=SOURCE,templates=220)
save('artefacts.json',artifacts);save('maps.json',maps)
save('templates-220.json',[dict(path=str(f.relative_to(C)),sha256=digest(f),size=f.stat().st_size) for f in files])
quiet=load(OLD/'nav-quiescence.json');assert quiet['live']==[] and quiet['context']==[]
tmp=Path(load(OLD/'tmp-navigateur.json')['path']);assert tmp.parent==Path('/var/tmp') and tmp.name.startswith('cx-pub.') and not tmp.is_symlink()
assert (tmp/'.possede-publication-composite').read_text().strip()==SOURCE
left=[]
for proc in Path('/proc').iterdir():
 if not proc.name.isdigit() or int(proc.name)==os.getpid():continue
 try:
  if proc.stat().st_uid!=os.getuid() or (proc/'stat').read_text().rsplit(') ',1)[1].split()[0]=='Z':continue
  exe=os.readlink(proc/'exe');cwd=os.readlink(proc/'cwd');args=(proc/'cmdline').read_bytes()
  if exe.startswith(str(T)+'/') or cwd==str(W) or cwd.startswith(str(W)+'/') or str(tmp).encode() in args:left.append(dict(pid=proc.name,exe=exe,cwd=cwd))
 except OSError:continue
assert not left,left
save('quiescence.json',dict(live=[],context=[],checked_monotonic=time.monotonic()))
save('tmp-avant-retrait.json',dict(path=str(tmp),entries=sorted(str(p.relative_to(tmp)) for p in tmp.rglob('*'))))
shutil.rmtree(tmp);assert not tmp.exists()
save('tmp-retire.json',dict(path=str(tmp),absent=True,source=SOURCE))
assert time.monotonic()<info['deadline_monotonic']
record=dict(source=SOURCE,recipe_source=RECIPE,verdict='VERT',client_rc=0,composition='runtime-v3 + autonomous-attestation-v8; C7 explicit authorization',original_instrumentation_red_preserved=True,runtime_supervisor=runtime,attestation_supervisor=attestation,redundant_v4_cancellation=annulation,code_diff_paths=[],quiescence_scan=[],elf_rehash=4,tmp_removed=True,elapsed_s=time.monotonic()-info['initial_monotonic'],global_budget_s=5400,deadline_monotonic=info['deadline_monotonic'])
save('verdict-compose.json',record);(P/'VERDICT_COMPOSE').write_text('VERT\n')
print(json.dumps(record),flush=True)
