#!/usr/bin/env python3
"""Union figée Pub/ED/activation : CPU, fixtures neuves, portée commune externe."""
import ast, json, os, pathlib, re, shlex, subprocess, sys, time
W=pathlib.Path('/home/lucied/git_workspaces/cx-activation-modeles')
C=W/'extension/rag3weaver'
R=pathlib.Path.home()/'.cache/rag3weaver-build'
suffix=os.environ.get('ACTIVATION_UNION_SUFFIX','')
assert re.fullmatch(r'(?:-[a-z0-9-]{1,32})?',suffix)
P=pathlib.Path.home()/('.cache/embarquement-notes/activation/union-pub-ed'+suffix)
P.mkdir(exist_ok=False)
A=json.loads(pathlib.Path(os.environ['RAG3WEAVER_VALIDATION_RECU']).read_text())
assert A['echeance']>time.monotonic()+10
E=os.environ.copy()
for k in list(E):
    if k.startswith(('RAG3WEAVER_SERVICE_','RAG3WEAVER_EMBED_')) or k in ('RAG3WEAVER_PG','GOOGLE_APPLICATION_CREDENTIALS','RAG3WEAVER_STATE_DIR'): E.pop(k,None)
F='rag3db-native,burn-embedder,burn-ocr,code,daemon,postgres,openai-llm,anthropic-llm'
E.update(CARGO_TARGET_DIR=str(R/'target-spans'),CARGO_BUILD_JOBS='8',CARGO_INCREMENTAL='0',RUST_TEST_THREADS='8',LUCIVY_SCHEDULER_THREADS='8',RAG3DB_LIB='rag3db-build-29a7bc52aaf11e6204f9beafaffb154105417e13-Release-p12-v11-g17-s18-rcOFF-stOFF-asanOFF-tsanOFF-ubsanOFF',RAG3WEAVER_MOTEUR_ANCIEN='1',RAG3WEAVER_TARGET_MAX_GO='32',FEATURES=F,RAG3WEAVER_E2E_FEATURES=F,RAG3WEAVER_PAQUETS_REACT=str(R/'I-paquets-react'),RAG3WEAVER_EXIGER_NAVIGATEUR='1',RAG3WEAVER_NAVIGATEUR='/usr/lib/chromium/chromium',CHECK_FEATURES_LOGS=str(P/'features'),RAG3WEAVER_JOURNAUX_DIR=str(P/'commandes'))
for suffix in ('','_EMBEDDER','_RERANKER','_OCR'): E['RAG3WEAVER_BURN_DEVICE'+suffix]='cpu'
# Uniquement la connexion de fixture publiée, jamais un fichier de secrets.
old=ast.parse((R/'cx-passe-apercu-transfert-v1.py').read_text())
v=[ast.literal_eval(n.value) for n in old.body if isinstance(n,ast.Assign) and any(isinstance(t,ast.Name) and t.id=='PG_CONNECTION' for t in n.targets)]
assert len(v)==1
fields=dict(s.split('=',1) for s in shlex.split(v[0]))
assert fields['host'] in ('127.0.0.1','localhost') and fields['port']=='5433' and fields['user']=='rag3weaver'
PG='cx_embar_union_20261113'+suffix.replace('-','_')
fields['dbname']=PG
E['RAG3WEAVER_PG']=' '.join(k+'='+shlex.quote(v) for k,v in fields.items())
pg=['docker','exec','rag3weaver-pg']
assert not subprocess.check_output(pg+['psql','-U','rag3weaver','-d','postgres','-Atc',f"SELECT 1 FROM pg_database WHERE datname='{PG}'"],text=True).strip()
subprocess.run(pg+['createdb','-U','rag3weaver','-O','rag3weaver',PG],check=True)
source=subprocess.check_output(['git','rev-parse','HEAD'],cwd=W,text=True).strip()
(P/'source.json').write_text(json.dumps(dict(source=source,deadline=A['echeance'],features=F,pg=PG,cpu_roles=4)))
def run(name,cmd):
    assert subprocess.check_output(['git','rev-parse','HEAD'],cwd=W,text=True).strip()==source
    assert not subprocess.check_output(['git','status','--porcelain'],cwd=W,text=True).strip()
    remaining=A['echeance']-time.monotonic()-15
    assert remaining>0
    print('DEBUT',name,round(remaining),flush=True)
    with (P/(name+'.log')).open('w') as out:
        subprocess.run(['bash','-c','. scripts/lib_du_magasin.sh; exporter_la_lib || exit 2; exec "$@"','union',*cmd],cwd=C,env=E,stdout=out,stderr=subprocess.STDOUT,check=True,timeout=remaining)
    assert int(subprocess.check_output(['du','-sB1',str(R/'target-spans')],text=True).split()[0])<=34359738368
    print('VERT',name,flush=True)
    return (P/(name+'.log')).read_text(errors='replace')
try:
    run('features',['bash','scripts/check_features.sh'])
    run('lib',['cargo','test','--lib','--features',F])
    run('navigateur',['bash','scripts/temoins_navigateur.sh',F])
    for suite,n in (('e2e_tx_ligne_a_ligne',3),('e2e_tx_merge_puis_copy',1),('e2e_tx_par_paquet_arret',18),('e2e_tx_vecteurs_apres_rollback',3)):
        log=run(suite,['bash','run_e2e.sh','--test',suite])
        assert re.findall(r'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;',log)[-1]==(str(n),'0','0')
        assert 'Suites jouées : 1 sur 1 demandées.' in log and not re.search(r'\b(?:SAUTÉ|SKIPPED)\b',log)
    (P/'VERT').write_text('VERT\n')
finally:
    subprocess.run(pg+['dropdb','-U','rag3weaver',PG],check=True)
