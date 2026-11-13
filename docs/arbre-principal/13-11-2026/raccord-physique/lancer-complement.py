#!/usr/bin/env python3
"""Start a DISTINCT corrective 3600-second scope after actual poste admission."""
from pathlib import Path
import importlib.util,json,math,os,sys,time
sys.dont_write_bytecode=True
root=Path('/home/lucied/.cache/rag3weaver-build')
source=Path('/home/lucied/.cache/rag3weaver-build/cx-wt-raccord-physique-union')
controller=Path(__file__).with_name('complement-global.py')
spec=importlib.util.spec_from_file_location('owned_confinement',source/'extension/rag3weaver/scripts/confiner_validation.py')
c=importlib.util.module_from_spec(spec);sys.modules[spec.name]=c;spec.loader.exec_module(c)
assert c.poste_tenu(c.processus()),'actual poste required before computing the remaining budget'
initial=json.loads((root/'cx-passe-raccord-physique-union-v1/source.json').read_text())
assert initial['deadline_monotonic']==896572.526453239
assert time.monotonic()>initial['deadline_monotonic'],'previous scope must be expired'
os.environ['POSTE_MEM_MAX']='32G';os.environ['RAG3WEAVER_BUILD_MEMORY_HIGH']='0'
args=['python3',str(source/'extension/rag3weaver/scripts/confiner_validation.py'),'lancer','--delai','3600','--preuves',str(root/'cx-raccord-physique-global-complement-confinement-v1'),'--','python3',str(controller),sys.argv[1]]
os.execvpe(args[0],args,os.environ.copy())
