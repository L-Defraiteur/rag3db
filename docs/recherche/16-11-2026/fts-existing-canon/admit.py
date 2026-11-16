#!/usr/bin/env python3
"""Mint one fresh bounded admission only after actual poste acquisition."""
import json,math,os,sys,time
from pathlib import Path
Q=Path(__file__).parent
config=json.loads((Q/'prepared.json').read_text())
start=time.monotonic();end=start+config['budget'];duration=math.floor(end-time.monotonic())-3
assert duration>60
with (Q/'remaining-after-poste.json').open('x') as f:
 json.dump(dict(observed=start,duration=duration,original_start=start,original_end=end),f)
os.execv(sys.executable,[sys.executable,str(Q/'confiner_validation.py'),'lancer','--delai',str(duration),'--preuves',config['scope'],'--',sys.executable,str(Q/'driver.py')])
