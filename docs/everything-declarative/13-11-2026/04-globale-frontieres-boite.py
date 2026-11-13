# Recette composée adaptée du handover Archi6786 ; aucun lancement autonome.
from pathlib import Path
import os,sys,json,time,subprocess,threading,hashlib,re,traceback,shlex,shutil
source=sys.argv[1]
B=Path.home()/'.cache/rag3weaver-build';O=B/'I-luciepc/frontieres-boite-20261113'/f'globale-{source[:9]}';O.mkdir(exist_ok=True)
W=Path.home()/'git_workspaces/cx-ed-frontieres-boite';C=W/'extension/rag3weaver';T=B/'target-cx-ed-frontieres-boite'
I=B/'I-luciepc/frontieres-boite-20261113/globale-budget';I.mkdir(exist_ok=True)
receipt=Path(os.environ['RAG3WEAVER_VALIDATION_RECU']);r=json.loads(receipt.read_text());deadline=r['echeance'];cg=Path('/proc/self/cgroup').read_text().strip().split(':',2)[2]
assert r['etat']=='actif' and r['cgroup']==cg and deadline>time.monotonic()
assert (Path('/sys/fs/cgroup'+cg)/'memory.max').read_text().strip()=='34359738368'
assert (Path('/sys/fs/cgroup'+cg)/'memory.high').read_text().strip()=='max'
assert os.environ['RAG3WEAVER_BUILD_MEMORY_HIGH']=='0'
assert subprocess.check_output(['git','-C',str(W),'rev-parse','HEAD'],text=True).strip()==source
assert not subprocess.check_output(['git','-C',str(W),'status','--porcelain'],text=True).strip()
assert not (O/'debut.json').exists(), 'preuves déjà utilisées'
if (I/'initial.json').exists():
 initial=json.loads((I/'initial.json').read_text())
else:
 initial=dict(source=source,premieres_preuves=str(O),deadline=deadline,debut_budget=deadline-6000)
 (I/'initial.json').write_text(json.dumps(initial,indent=2))
initial_deadline=initial['deadline'];initial_begin=initial['debut_budget']
assert deadline<=initial_deadline and time.monotonic()<initial_deadline
(O/'debut.json').write_text(json.dumps(dict(source=source,superviseur=r['pid'],debut_budget=initial_begin,deadline=deadline,deadline_initiale=initial_deadline,cgroup=cg,receipt=str(receipt)),indent=2))
F='rag3db-native,burn-embedder,burn-ocr,code,daemon,postgres,openai-llm,anthropic-llm'
env=os.environ.copy();env['PATH']=str(Path.home()/'.cargo/bin')+':'+env['PATH'];env.update(CARGO_MANIFEST_DIR=str(C),RAG3DB_LIB='rag3db-build-29a7bc52aaf11e6204f9beafaffb154105417e13-Release-p12-v11-g17-s18-rcOFF-stOFF-asanOFF-tsanOFF-ubsanOFF',CARGO_TARGET_DIR=str(T),CARGO_BUILD_JOBS='8',CARGO_INCREMENTAL='0',TMPDIR=str(Path.home()/'.cache/cx-ed-fb'),XDG_CONFIG_HOME=str(O/'config'),XDG_DATA_HOME=str(O/'data'),XDG_CACHE_HOME=str(O/'cache'),RAG3WEAVER_BUILTIN_TEMPLATES_DIR=str(O/'builtin'),RAG3WEAVER_STATE_DIR=str(O/'etat'),RAG3WEAVER_CACHE_DIR=str(O/'cache'),RAG3WEAVER_RUNS_DIR=str(O/'runs'),RAG3WEAVER_CHATS_DIR=str(O/'chats'),RAG3WEAVER_TACHES_DIR=str(O/'taches'),RAG3WEAVER_JOURNAUX_DIR=str(O/'racine de journaux'),RAG3WEAVER_PAQUETS_REACT=str(Path.home()/'.cache/rag3weaver-build/I-paquets-react'),RAG3WEAVER_NAVIGATEUR='/usr/lib/chromium/chromium',RAG3WEAVER_BURN_DEVICE='cpu',RAG3WEAVER_BURN_DEVICE_EMBEDDER='cpu',RAG3WEAVER_BURN_DEVICE_RERANKER='cpu',RAG3WEAVER_BURN_DEVICE_OCR='cpu',RAG3WEAVER_TARGET_MAX_GO='32')
for k in list(env):
 if k.endswith('_API_KEY') or k.startswith('RAG3WEAVER_SERVICE_') or k in ['RAG3WEAVER_EMBED_SERVICE','GOOGLE_APPLICATION_CREDENTIALS','GOOGLE_CLOUD_PROJECT','GCLOUD_PROJECT','RAG3WEAVER_EXIGER_GEMINI','RAG3WEAVER_LLM','RAG3WEAVER_MOTEUR_ANCIEN','RAG3WEAVER_MESURE','RAG3WEAVER_PG','RAG3WEAVER_GARDER']:
  env.pop(k,None)
lib=Path.home()/'.cache/rag3db-libs'/env['RAG3DB_LIB'];env['RAG3DB_LIBS_DIR']=str(lib.parent);env['LD_LIBRARY_PATH']=str(lib)+(':'+env['LD_LIBRARY_PATH'] if env.get('LD_LIBRARY_PATH') else '')
Path(env['TMPDIR']).mkdir(exist_ok=True)
KEYS={'TMPDIR','PATH','LD_LIBRARY_PATH','CUDA_ROOT','CARGO_MANIFEST_DIR','CARGO_INCREMENTAL','RUST_BACKTRACE','XDG_CONFIG_HOME','XDG_DATA_HOME','XDG_CACHE_HOME','RAG3DB_LIB','RAG3DB_LIBS_DIR','RAG3DB_ROOT','RAG3DB_SHARED','RAG3DB_LIBRARY_DIR','RAG3DB_INCLUDE_DIR','RAG3WEAVER_REGIME','RAG3WEAVER_MOTEUR_ANCIEN','RAG3WEAVER_GARDER','RAG3WEAVER_PASSE_E2E','RAG3WEAVER_E2E_JOURNAUX_DEMONS','RAG3WEAVER_E2E_DOSSIER','RAG3WEAVER_CHECKPOINT_DIR','RAG3WEAVER_STATE_DIR','RAG3WEAVER_CACHE_DIR','RAG3WEAVER_BUILTIN_TEMPLATES_DIR','RAG3WEAVER_RUNS_DIR','RAG3WEAVER_CHATS_DIR','RAG3WEAVER_TACHES_DIR','RAG3WEAVER_JOURNAUX_DIR','RAG3WEAVER_SERVICE_SPARSE','RAG3WEAVER_BURN_DEVICE','RAG3WEAVER_BURN_DEVICE_EMBEDDER','RAG3WEAVER_BURN_DEVICE_RERANKER','RAG3WEAVER_BURN_DEVICE_OCR'}
stop=threading.Event();observes={};erreurs=[];etapes=[]
def observer():
 while not stop.is_set():
  try:
   for pid in (Path('/sys/fs/cgroup'+cg)/'cgroup.procs').read_text().split():
    p=Path('/proc')/pid
    try:
     exe=os.readlink(p/'exe')
     if not exe.startswith(str(T/'debug/deps')):continue
     maps=(p/'maps').read_text();rows=[x for x in maps.splitlines() if 'librag3db.so' in x]
     if not rows:continue
     assert all('/'+env['RAG3DB_LIB']+'/librag3db.so' in x for x in rows),rows
     st=(p/'stat').read_text().rsplit(') ',1)[1].split()[19];phase=(O/'phase').read_text();key=(pid,st,exe,phase)
     if phase in ['lectures-ancien','create-natif']:
      raw=dict(v.split(b'=',1) for v in (p/'environ').read_bytes().split(b'\0') if b'=' in v)
      profile={k:os.fsdecode(raw[os.fsencode(k)]) for k in KEYS if os.fsencode(k) in raw}
      if phase=='lectures-ancien' and profile.get('RAG3WEAVER_E2E_DOSSIER','').endswith('/e2e_lectures') and not (O/'profil-lectures.json').exists():
       (O/'profil-lectures.json').write_text(json.dumps(dict(env=profile,pid=pid,starttime=st,cgroup=cg,exe=exe),indent=2))
      if phase=='create-natif' and not (O/'profil-create.json').exists():
       (O/'profil-create.json').write_text(json.dumps(dict(env=profile,pid=pid,starttime=st,cgroup=cg,exe=exe),indent=2))
     if key not in observes:
      q=O/'provenances';q.mkdir(exist_ok=True)
      row=dict(pid=pid,starttime=st,exe=exe,cgroup=cg,memory_max='34359738368',native=rows,phase=phase)
      observes[key]=row;(q/(pid+'-'+phase+'.maps')).write_text(maps);(q/(pid+'-'+phase+'.json')).write_text(json.dumps(row,indent=2))
    except (OSError,ProcessLookupError):continue
  except BaseException as e:erreurs.append(repr(e));return
  stop.wait(.01)
thread=threading.Thread(target=observer);thread.start()
def run(name,seconds,argv,extra=None,borne=None):
 limit=min(deadline-2,time.monotonic()+seconds,borne or deadline);assert time.monotonic()<limit,('budget',name)
 (O/'phase').write_text(name);e=env.copy();e.update(extra or {})
 with (O/(name+'.log')).open('w') as f:
  p=subprocess.Popen(argv,cwd=C,env=e,stdout=f,stderr=subprocess.STDOUT)
  prochain_disque=time.monotonic()+5
  while p.poll() is None:
   assert not erreurs,erreurs
   if time.monotonic()>=prochain_disque:
    assert int(subprocess.check_output(['du','-sb',str(T)],text=True).split()[0])<=34359738368
    prochain_disque=time.monotonic()+5
   assert time.monotonic()<limit,('timeout',name)
   text=(O/(name+'.log')).read_text(errors='replace')
   assert not re.search(r'^test .* \.\.\. FAILED$',text,re.M),('premier rouge',name)
   if name.startswith('features'):assert not re.search(r'^ROUGE\b',text,re.M),('premier rouge features',name)
   time.sleep(.05)
  assert p.returncode==0,(name,p.returncode)
 etapes.append(dict(phase=name,rc=0));(O/'etapes.json').write_text(json.dumps(etapes,indent=2))
 assert int(subprocess.check_output(['du','-sb',str(T)],text=True).split()[0])<=34359738368
 return (O/(name+'.log')).read_text(errors='replace')
def cargo(*args):return ['bash','scripts/cargo_magasin.sh',*args]
def binaire(log,nom):
 artifacts=[]
 for l in log.splitlines():
  try:j=json.loads(l)
  except ValueError:continue
  if j.get('reason')=='compiler-artifact' and j.get('executable') and j.get('target',{}).get('name')==nom:artifacts.append(j['executable'])
 assert len(artifacts)==1,(nom,artifacts)
 return Path(artifacts[0])
def noms(log):return {l[:-6] for l in log.splitlines() if l.endswith(': test')}
def resultat(log,passed,ignored):
 rs=re.findall(r'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;',log);assert rs and tuple(map(int,rs[-1]))==(passed,0,ignored),rs
 return dict(passed=passed,failed=0,ignored=ignored)
resultats={}
CASE='e2e_lectures::la_recherche_rend_les_mots_pendant_un_create_de_l_index_vectoriel'
def sante(name):
 h=json.loads(run(name,10,['curl','--max-time','8','-fsS','http://127.0.0.1:7879/sante']))
 assert h.get('modele')=='bge-m3' and h.get('dim')==1024 and h.get('factice') is False and h.get('sparse') is True,h
 return h
try:
 run('target-guard',10,['bash','-c','. scripts/garde_target.sh; garder_le_target "$CARGO_TARGET_DIR" "$PWD"'])
 run('tickets',30,['python3','scripts/check_tickets.py'])
 run('tickets-cpu',30,['python3','scripts/test_check_tickets.py'])
 run('etat-cpu',60,['bash','scripts/test_etat_suite_e2e.sh'])
 matrice=re.search(r'(?ms)^JEUX=\(\n(.*?)^\)',(C/'scripts/check_features.sh').read_text()).group(1)
 jeux=shlex.split(matrice,comments=True)
 assert len(jeux)==8 and jeux[:3]==['','daemon','rag3db-native,daemon'],jeux
 (O/'jeux.json').write_text(json.dumps(jeux,indent=2))
 fin_features=min(deadline-2,time.monotonic()+2100)
 for i,jeu in enumerate(jeux,1):
  run('features-'+str(i),900,cargo('check','--tests','--no-default-features','--features',jeu,'-j8'),borne=fin_features)
 (O/'features-verdict').write_text('CHECK_FEATURES VERT 8/8\n')
 run('minimal',120,cargo('check','--locked','--lib','--no-default-features','-j8'))
 fin_compile=min(deadline-2,time.monotonic()+1500)
 bins={}
 for key,kind,features in [('large','--lib',F),('nav','--lib',F+',navigateur'),('e2e','--test',F)]:
  args=['test','-j8','--no-run',kind]+(['e2e'] if kind=='--test' else [])+(['--no-default-features'] if key=='large' else [])+['--features',features,'--message-format=json']
  bins[key]=binaire(run('compile-'+key,1500,cargo(*args),borne=fin_compile),'e2e' if key=='e2e' else 'rag3weaver')
  shutil.copy2(bins[key],O/('ELF-'+key))
 inventaire={};bin_infos={}
 for key,b in bins.items():
  allnames=noms(run('liste-'+key,30,[str(b),'--list']));ignores=noms(run('ignores-'+key,30,[str(b),'--ignored','--list']))
  assert allnames and ignores<=allnames
  raw=b.read_bytes();files=[p for p in (C/'templates').rglob('*') if p.is_file()]
  assert source[:9].encode() in raw and len(files)==223 and all(p.read_bytes() in raw for p in files)
  bin_infos[key]=dict(executable=str(b),archive=str(O/('ELF-'+key)),sha256=hashlib.sha256(raw).hexdigest(),source=source,templates=len(files))
  (O/('dynamic-'+key+'.txt')).write_text(subprocess.check_output(['readelf','-d',str(b)],text=True))
  inventaire[key]=dict(tous=sorted(allnames),ignores=sorted(ignores))
 navfilters=set()
 for p in (C/'src').rglob('*.rs'):
  ls=p.read_text().splitlines()
  for i,l in enumerate(ls):
   if 'feature = "navigateur"), ignore' in l:
    for suite in ls[i:i+5]:
     match=re.search(r'fn ([a-z0-9_]+)\(',suite)
     if match:navfilters.add(match[1])
 navnames={n for n in inventaire['nav']['tous'] if any(f in n for f in navfilters)}
 assert len(navnames)==40 and not navnames.intersection(inventaire['nav']['ignores'])
 expected=dict(large=dict(passed=len(inventaire['large']['tous'])-len(inventaire['large']['ignores']),ignored=len(inventaire['large']['ignores'])),nav=dict(passed=len(navnames),ignored=0))
 assert expected['large']['passed']>2200
 for suite,n in [('e2e_lectures',27),('e2e_demon_hote',13)]:
  expected[suite]=dict(passed=len([x for x in inventaire['e2e']['tous'] if x.startswith(suite+'::')]),ignored=0)
  assert expected[suite]['passed']==n,(suite,expected[suite])
 assert CASE in inventaire['e2e']['tous']
 plan=dict(source=source,binaires=bin_infos,attendus=expected,listes=inventaire,nav_selection=sorted(navnames),deadline=deadline)
 (O/'inventaire.json').write_text(json.dumps(plan,indent=2));digest=hashlib.sha256((O/'inventaire.json').read_bytes()).hexdigest();(O/'inventaire.sha256').write_text(digest+'\n')
 (O/'phase').write_text('attente-publication-inventaire')
 while not (O/'inventaire-publie').exists():
  assert time.monotonic()<deadline-2,'budget pendant publication inventaire';assert not erreurs,erreurs;time.sleep(.1)
 assert (O/'inventaire-publie').read_text().strip()==digest
 resultats['large']=resultat(run('lib-large',900,cargo('test','-j8','--lib','--no-default-features','--features',F)),**expected['large'])
 resultats['nav']=resultat(run('navigateur',600,['bash','scripts/temoins_navigateur.sh',F],dict(RAG3WEAVER_EXIGER_NAVIGATEUR='1')),**expected['nav'])
 (O/'phase').write_text('attente-creneau-7879')
 while not (O/'creneau-7879').exists():
  assert time.monotonic()<deadline-2,'budget pendant coordination7879';time.sleep(.1)
 health=sante('sante-7879-avant')
 log=run('lectures-ancien',120,['bash','run_e2e.sh','--test','e2e_lectures'],dict(RAG3WEAVER_E2E_FEATURES=F,RAG3WEAVER_SERVICE_SPARSE='127.0.0.1:7879',RAG3WEAVER_GARDER='1',RAG3WEAVER_MOTEUR_ANCIEN='1'))
 assert 'SAUTÉ' not in log and re.search(r'test result: ok\. 2 passed; 0 failed; 0 ignored;',log)
 assert log.count('non jugé')==1 and "non jugé : la lib n'a pas la colonne state (format 43 requis)" in log
 resultats['lectures_ancien']=dict(**resultat(log,27,0),juges=26,non_juges=[CASE])
 profile=json.loads((O/'profil-lectures.json').read_text())['env']
 assert profile['RAG3WEAVER_MOTEUR_ANCIEN']=='1' and profile['RAG3WEAVER_GARDER']=='1'
 for k,b in bins.items():assert hashlib.file_digest(b.open('rb'),'sha256').hexdigest()==bin_infos[k]['sha256'],'ELF modifié'
 log=run('create-natif',120,['env','-u','RAG3WEAVER_MOTEUR_ANCIEN','bash',str(W/'docs/everything-declarative/13-11-2026/05-create-exact-meme-profil.sh'),str(bins['e2e']),bin_infos['e2e']['sha256'],str(O)],profile)
 assert 'non jugé' not in log and 'SAUTÉ' not in log
 resultats['create_natif']=resultat(log,1,0)
 assert json.loads((O/'gardes-complement.json').read_text())['verdict']=='VERT'
 after_profile=json.loads((O/'profil-create.json').read_text())['env']
 assert {k:v for k,v in profile.items() if k!='RAG3WEAVER_MOTEUR_ANCIEN'}==after_profile,'profil changé au-delà du seul drapeau'
 after=sante('sante-7879-apres');assert all(health.get(k)==after.get(k) for k in ['modele','dim','factice','sparse'])
 (O/'7879-rendu').write_text('lectures27 distincts/28brut et CREATE exact terminés, BGE rendu\n')
 log=run('e2e_demon_hote',120,['bash','run_e2e.sh','--test','e2e_demon_hote'],dict(RAG3WEAVER_E2E_FEATURES=F))
 assert 'SAUTÉ' not in log and 'non jugé' not in log
 assert re.search(r'test result: ok\. 2 passed; 0 failed; 0 ignored;',log)
 resultats['e2e_demon_hote']=resultat(log,13,0)
 for k,phase in [('large','lib-large'),('nav','navigateur'),('e2e','lectures-ancien')]:
  assert any(x['exe']==str(bins[k]) and x['phase']==phase for x in observes.values()),('ELF non observé',k)
 verdict=dict(verdict='VERT',source=source,resultats=resultats,binaires=bin_infos,lectures=dict(juges_distincts=27,invocations_brutes=28,filets_separes=True),deadline=deadline,fin=time.monotonic(),duree_cumulee=time.monotonic()-initial_begin,deadline_initiale=initial_deadline,target_octets=int(subprocess.check_output(['du','-sb',str(T)],text=True).split()[0]))
 (O/'verdict-driver.json').write_text(json.dumps(verdict,indent=2));(O/'verdict-driver').write_text('VERT\n')
except BaseException as e:
 (O/'erreur').write_text(repr(e)+'\n');(O/'verdict-driver').write_text('ROUGE\n');traceback.print_exc();raise
finally:
 stop.set();thread.join();(O/'observations-elf.json').write_text(json.dumps(list(observes.values()),indent=2))
