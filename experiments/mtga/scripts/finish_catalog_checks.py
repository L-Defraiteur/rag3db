"""Sequential checks, reopening and durably closing the same database each time."""
import subprocess,sys
from pathlib import Path
ROOT=Path(__file__).resolve().parents[3];S=Path(__file__).parent;P=ROOT/'experiments/mtga/data'
checks=[('render',[str(S/'test_engine_render.py')]),('wildcards',[str(S/'plan_wildcards.py'),str(ROOT/'experiments/mtga/backend/queries/catalog/wildcard-targets.json'),str(P/'catalog-research/wildcard-plan.json')]),('memory',[str(S/'measure_search_memory.py')])]
for label,args in checks:
 log=Path('/tmp')/f'mtga-final-{label}.log'
 with log.open('w') as output:subprocess.run([sys.executable,*args],cwd=ROOT,stdout=output,stderr=subprocess.STDOUT,check=True)
 print(label,'passed',flush=True)
