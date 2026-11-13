#!/usr/bin/env python3
"""Admit only the remaining original time AFTER the actual poste."""
from pathlib import Path
import json,math,os,sys,time
INITIAL=918138.385510088
ORIGINAL_END=922938.385510088
def remaining(now):return math.floor(ORIGINAL_END-now)-3
def main():
 q=Path(__file__).parent;s=Path('/home/lucied/.cache/rag3weaver-build/cx-confinement-couches-2b-union-v3')
 now=time.monotonic();duration=remaining(now);assert duration>60,'original deadline exhausted, refuse before receipt'
 (q/'remaining-after-poste.json').write_text(json.dumps(dict(initial=INITIAL,absolute=ORIGINAL_END,observed=now,duration=duration)))
 os.execv(sys.executable,[sys.executable,str(q/'confiner_validation.py'),'lancer','--delai',str(duration),'--preuves',str(s),'--','python3',str(q/'driver.py')])
if __name__=='__main__':main()
