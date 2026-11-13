#!/usr/bin/env python3
"""Exec the common authority AFTER actual poste; one fresh 4800s receipt."""
from pathlib import Path
import os,sys
q=Path(__file__).parent
s=Path('/home/lucied/.cache/rag3weaver-build/cx-confinement-couches-2b-union-v1')
os.execv(sys.executable,[sys.executable,str(q/'confiner_validation.py'),'lancer','--delai','4800','--preuves',str(s),'--','python3',str(q/'driver.py')])
