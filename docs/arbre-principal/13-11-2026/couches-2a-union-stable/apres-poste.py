from pathlib import Path
import importlib.util,json,math,os,sys,time
sys.dont_write_bytecode=True;Q=Path(__file__).parent;R=Q.parent
spec=importlib.util.spec_from_file_location('probe_confinement',Q/'confiner_validation.py');c=importlib.util.module_from_spec(spec);sys.modules[spec.name]=c;spec.loader.exec_module(c)
assert c.poste_tenu(c.processus())
original=json.loads((Q/'provenance.json').read_text())['original'];assert original['deadline']==911024.174319861
remaining=math.floor(original['deadline']-time.monotonic())-3;assert remaining>60,'original deadline exhausted'
args=['python3',str(Q/'confiner_validation.py'),'lancer','--delai',str(remaining),'--preuves',str(R/'cx-confinement-couches-2a-union-v3'),'--','python3',str(Q/'driver.py')]
os.execvpe(args[0],args,os.environ.copy())
