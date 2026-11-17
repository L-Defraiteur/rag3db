"""Admission after the true poste; no new budget when the slot is acquired."""
import json,math,os,sys,time
from pathlib import Path
q=Path(__file__).parent;c=json.loads((q/'prepared.json').read_text());a=json.loads((q/'allocation.json').read_text())
remaining=math.floor(a['end']-time.monotonic())-15
assert 30<remaining<=c['budget']
(q/'remaining-after-poste.json').write_text(json.dumps(dict(remaining=remaining,original_end=a['end'])))
os.chdir(q)
os.execve(sys.executable,[sys.executable,str(q/'confiner_validation.py'),'lancer','--delai',str(remaining),'--preuves',c['scope'],
    '--',sys.executable,str(q/'tab_green_driver.py')],dict(os.environ,TTY_CONTROLLER_DIR=str(q)))
