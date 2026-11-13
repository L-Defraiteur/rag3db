# À lancer via poste lourd ; le budget est fixé après acquisition réelle.
from pathlib import Path
import importlib.util,json,math,os,sys,time
b=Path.home()/'.cache/rag3weaver-build';w=Path.home()/'git_workspaces/cx-ed-frontieres-boite'
spec=importlib.util.spec_from_file_location('confineur',w/'extension/rag3weaver/scripts/confiner_validation.py')
c=importlib.util.module_from_spec(spec);spec.loader.exec_module(c)
assert c.poste_tenu(c.processus()),'poste réel requis avant calcul du budget'
i=b/'I-luciepc/frontieres-boite-20261113/globale-budget/initial.json'
assert i.is_file(), 'budget initial requis'
reste=min(600,math.floor(json.loads(i.read_text())['deadline']-time.monotonic())-3)
assert reste>30,('budget global expiré',reste)
os.environ['POSTE_MEM_MAX']='32G'
os.environ['RAG3WEAVER_BUILD_MEMORY_HIGH']='0'
a=['python3',str(w/'extension/rag3weaver/scripts/confiner_validation.py'),'lancer','--delai',str(reste),'--preuves',str(b/'I-luciepc/frontieres-boite-20261113'/('complement-'+sys.argv[1][:9]+'-supervision')),'--','python3',str(b/'I-luciepc/frontieres-boite-20261113/07-complement-frontieres-boite.py'),sys.argv[1]]
os.execvpe(a[0],a,os.environ.copy())
