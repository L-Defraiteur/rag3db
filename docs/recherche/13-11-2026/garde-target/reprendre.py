#!/usr/bin/env python3
"""Missing builtin module only, fixed siblings profile, no Cargo and no reset."""
from pathlib import Path
import hashlib,importlib.util,json,math,mmap,os,re,shutil,signal,subprocess,sys,time
R=Path('/home/lucied/.cache/rag3weaver-build');W=R/'cx-wt-recherche-garde-target';C=W/'extension/rag3weaver';T=R/'cx-target-garde-target-qualification-v1'
OLD=R/'cx-validation-garde-target-v1';P=R/'cx-validation-garde-target-v2';Q=R/'cx-pilote-garde-target-v2';S=R/'cx-confinement-garde-target-v2'
APP='698b952ab89fd48ac112645650000ebae34e6742';PREVIOUS='bdffcef070d78f5a97cc7d2a1edbf60baef7b58e';START=909825.832473217;END=911625.832473217
LEDGER_SHA='6da389b1c6f8294e6d8d306c33b71db82e78483c0e6a27cd5c29fb5975bcd070'
def sha(path):
 with Path(path).open('rb') as f:return hashlib.file_digest(f,'sha256').hexdigest()
def save(name,value):(P/name).write_text(json.dumps(value,indent=2)+'\n')
def git(root,*args):return subprocess.check_output(['git','-C',str(root),*args],env=dict(os.environ,GIT_OPTIONAL_LOCKS='0'))
def guard(meta):
 for root,h in [(W,APP),(Path(meta['control']),meta['pilot'])]:
  assert git(root,'rev-parse','HEAD').decode().strip()==h and not git(root,'status','--porcelain').strip()
  for n in ['rebase-merge','rebase-apply']:assert not Path(git(root,'rev-parse','--path-format=absolute','--git-path',n).decode().strip()).exists()
 for name,h in meta['files'].items():assert sha(Q/name)==h
 assert sha(OLD/'ledger.json')==LEDGER_SHA
 assert (T/'.arbre-du-target').read_text().strip()==str(C)
def helper():
 spec=importlib.util.spec_from_file_location('reviewed_attestation',Q/'attester-original.py');a=importlib.util.module_from_spec(spec);spec.loader.exec_module(a);a.P=P;return a
def main():
 if sys.argv[1:]==['--poste']:
  meta=json.loads((Q/'provenance.json').read_text());guard(meta)
  spec=importlib.util.spec_from_file_location('resume_confinement',Q/'confiner_validation.py');c=importlib.util.module_from_spec(spec);sys.modules[spec.name]=c;spec.loader.exec_module(c);assert c.poste_tenu(c.processus())
  remaining=math.floor(END-time.monotonic())-3;assert remaining>60
  os.execvpe('python3',['python3',str(Q/'confiner_validation.py'),'lancer','--delai',str(remaining),'--preuves',str(S),'--','python3',str(Q/'reprendre.py'),'--driver'],os.environ.copy())
 if sys.argv[1:]==['--driver']:
  meta=json.loads((Q/'provenance.json').read_text());guard(meta);auth=Path(os.environ['RAG3WEAVER_VALIDATION_RECU']);authority=json.loads(auth.read_text());end=min(END,authority['echeance']);assert authority['echeance']<=END
  P.mkdir();source=dict(app=APP,pilot=meta['pilot'],control=meta['control'],receipt=str(auth),start=START,end=end,original_end=END,previous_pilot=PREVIOUS,phase='runtime-manquant-seulement');save('source.json',source)
  for name in ['tmp','cache','data','config','xdg-state','state-fixture']:(P/name).mkdir()
  env={k:os.environ[k] for k in ['HOME','USER','LOGNAME','PATH','LANG','LC_ALL','TZ','DBUS_SESSION_BUS_ADDRESS','RAG3WEAVER_VALIDATION_RECU','RAG3WEAVER_VALIDATION_JETON'] if k in os.environ}
  env.update(CARGO_TARGET_DIR=str(T),CARGO_MANIFEST_DIR=str(C),CARGO_BUILD_JOBS='8',CARGO_INCREMENTAL='0',RUST_TEST_THREADS='8',TMPDIR=str(P/'tmp'),XDG_CACHE_HOME=str(P/'cache'),XDG_DATA_HOME=str(P/'data'),XDG_CONFIG_HOME=str(P/'config'),XDG_STATE_HOME=str(P/'xdg-state'),RAG3WEAVER_STATE_DIR=str(P/'state-fixture'),RAG3WEAVER_CACHE_DIR=str(P/'cache'))
  for suffix in ['','_EMBEDDER','_RERANKER','_OCR']:env['RAG3WEAVER_BURN_DEVICE'+suffix]='cpu'
  assert not Path(env['RAG3WEAVER_STATE_DIR']).is_relative_to(env['XDG_STATE_HOME']);save('profil-demande.json',env|{'RAG3WEAVER_VALIDATION_JETON':'<non publié>'})
  cg=Path('/proc/self/cgroup').read_text().strip().split('::',1)[1];g=Path('/sys/fs/cgroup'+cg);assert (g/'memory.max').read_text().strip()=='34359738368' and (g/'memory.high').read_text().strip()=='max'
  events=lambda:dict(line.split() for line in (g/'memory.events').read_text().splitlines());before=events()
  try:
   ledger=json.loads((OLD/'ledger.json').read_text());assert ledger['app']==APP and ledger['pilot']==PREVIOUS and ledger['features']==[] and len(ledger['names'])==9
   binary=Path(ledger['binary']);assert binary.is_relative_to(T) and sha(binary)==sha(OLD/'binaire-cpu')==ledger['sha256'];shutil.copy2(binary,P/'binaire-cpu')
   artifacts=[];builds={}
   for line in (OLD/'compile.log').read_text().splitlines():
    try:v=json.loads(line)
    except ValueError:continue
    if v.get('reason')=='build-script-executed':builds[v['package_id']]=v['out_dir']
    if v.get('reason')=='compiler-artifact' and v.get('target',{}).get('name')=='rag3weaver' and v['profile']['test'] and v.get('executable'):artifacts.append(v)
   assert len(artifacts)==1 and artifacts[0]['executable']==str(binary) and artifacts[0]['features']==[]
   a=helper();a.attester(binary,artifacts[0],builds)
   save('ledger.json',dict(ledger,resume_pilot=meta['pilot'],carried_sha256=LEDGER_SHA));h=sha(P/'ledger.json');print('INVENTAIRE_A_PUBLIER',h,flush=True)
   while not (P/'inventaire-publie').exists():assert time.monotonic()<end-15;time.sleep(.1)
   assert (P/'inventaire-publie').read_text().strip()==h
   limit=min(end-15,time.monotonic()+120)
   with (P/'templates.log').open('w') as out:
    child=subprocess.Popen([str(binary),'builtin_templates::tests::','--show-output','--test-threads=8'],cwd=C,env=env,stdout=out,stderr=subprocess.STDOUT,start_new_session=True)
    save('runtime-pid.json',dict(pid=child.pid,starttime=(Path('/proc')/str(child.pid)/'stat').read_text().rsplit(') ',1)[1].split()[19],exe=str(binary),cgroup=cg))
    try:
     while True:
      text=(P/'templates.log').read_text(errors='replace');assert not re.search(r'^test .* \.\.\. FAILED$|^FAILED\b',text,re.M),'premier rouge';assert time.monotonic()<limit
      if child.poll() is not None:break
      time.sleep(.025)
     assert child.wait()==0
    except BaseException:
     if child.poll() is None:
      os.killpg(child.pid,signal.SIGTERM)
      try:child.wait(timeout=2)
      except subprocess.TimeoutExpired:os.killpg(child.pid,signal.SIGKILL);child.wait()
     raise
   text=(P/'templates.log').read_text();assert re.findall(r'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;',text)[-1]==('9','0','0')
   successes=set(re.findall(r'^test (.*?) \.\.\. ok$',text,re.M));assert set(ledger['names'])<=successes
   a.attester(binary,artifacts[0],builds);assert sha(binary)==sha(P/'binaire-cpu')==ledger['sha256'];after=events();save('memory.json',dict(before=before,after=after,peak=int((g/'memory.peak').read_text())));assert all(before[k]==after[k] for k in ['high','max','oom','oom_kill']);guard(meta);assert time.monotonic()<end-10
   save('driver-verdict.json',dict(verdict='VERT',meaning='module9 et attestations seulement, ancien rouge conservé'))
  except BaseException as e:save('driver-verdict.json',dict(verdict='ROUGE',error=repr(e)));raise
  return
 pilot=sys.argv[1];assert re.fullmatch('[0-9a-f]{40}',pilot);control=Path(__file__).resolve().parents[4];assert os.uname().nodename.lower()=='luciepc'
 assert not any(x.exists() for x in [P,Q,S]);assert sha(OLD/'ledger.json')==LEDGER_SHA
 old_source=json.loads((OLD/'source.json').read_text());assert old_source['app']==APP and old_source['pilot']==PREVIOUS and old_source['start']==START and old_source['end']==END
 assert json.loads(Path(old_source['receipt']).read_text())['etat']=='termine'
 assert json.loads((OLD/'driver-verdict.json').read_text())['verdict']=='ROUGE'
 Q.mkdir();shutil.copy2(__file__,Q/'reprendre.py');shutil.copy2(C/'scripts/confiner_validation.py',Q/'confiner_validation.py')
 original=Path(__file__).with_name('qualifier.py');assert original.read_bytes()==git(W,'show',PREVIOUS+':docs/recherche/13-11-2026/garde-target/qualifier.py');shutil.copy2(original,Q/'attester-original.py')
 meta=dict(app=APP,pilot=pilot,control=str(control),files={n:sha(Q/n) for n in ['reprendre.py','confiner_validation.py','attester-original.py']});(Q/'provenance.json').write_text(json.dumps(meta));guard(meta)
 a=helper();scan=a.quiet(control);assert not scan['found'];(Q/'scan-avant.json').write_text(json.dumps(scan))
 os.chdir('/var/tmp');env=dict(os.environ,POSTE_MEM_MAX='32G',RAG3WEAVER_BUILD_MEMORY_HIGH='0')
 assert env.get('DBUS_SESSION_BUS_ADDRESS')=='unix:path=/run/user/1000/bus';subprocess.run(['busctl','--user','--timeout=5','call','org.freedesktop.DBus','/org/freedesktop/DBus','org.freedesktop.DBus.Peer','Ping'],env=env,check=True)
 rc=subprocess.run([str(C/'scripts/poste'),'lourd','python3',str(Q/'reprendre.py'),'--poste'],env=env).returncode;record=dict(verdict='ROUGE',code=rc)
 try:
  source=json.loads((P/'source.json').read_text());scope=json.loads((Path(source['receipt']).parent/'verdict.json').read_text());assert rc==0 and scope['verdict']=='VERT' and scope['code']==0 and scope['restes']==[] and scope['nettoye'] and not Path(scope['prive']).exists()
  assert json.loads((P/'driver-verdict.json').read_text())['verdict']=='VERT';scan=a.quiet(control);save('scan-final.json',scan);assert not scan['found']
  ledger=json.loads((OLD/'ledger.json').read_text());assert sha(ledger['binary'])==sha(OLD/'binaire-cpu')==sha(P/'binaire-cpu')==ledger['sha256'];guard(meta);now=time.monotonic();assert now<source['end']<=END and now-START<1800;record.update(verdict='VERT',scope=scope,elapsed=now-START,end=source['end'],original_end=END,meaning='qualification composite ciblée, pas F8/liblarge/NAV')
 except BaseException as e:record['error']=repr(e)
 if record['verdict']=='VERT' and time.monotonic()>=record['end']:record.update(verdict='ROUGE',error='deadline before publication')
 P.mkdir(exist_ok=True);save('verdict-compose.json',record);print(json.dumps(record));sys.exit(0 if record['verdict']=='VERT' else 1)
if __name__=='__main__':main()
