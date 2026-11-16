#!/usr/bin/env python3
"""Filesystem-only cold snapshot copy. No native engine or host calls."""
from pathlib import Path
import importlib.util,json,subprocess
HERE=Path(__file__).resolve().parent
spec=importlib.util.spec_from_file_location('inspect_fts',HERE/'inspect-fts.py');fs=importlib.util.module_from_spec(spec);spec.loader.exec_module(fs)
SOURCE=Path.home()/'.local/share/rag3weaver/instantanes/avant-59ac9c86b-20261116-927/code-rag3db-depot'
ROOT=Path.home()/'.cache/rag3weaver-build/bascule-live-20261116/cold-files-01'
def save(name,value):
 with (ROOT/name).open('x') as stream:json.dump(value,stream,ensure_ascii=False,indent=2)
def main():
 ROOT.mkdir(mode=0o700,parents=True,exist_ok=False)
 before=fs.file_inventory(SOURCE);save('source-before.json',before)
 copy=ROOT/'copy';subprocess.run(['cp','-a','--reflink=always','--',str(SOURCE),str(copy)],check=True,timeout=90)
 copied=fs.file_inventory(copy);save('copy-before.json',copied)
 if before!=copied:raise ValueError('copy mismatch')
 save('fts-inventory.json',fs.inspect(copy/'code-rag3db-depot.rag3db.fts'))
 after=fs.file_inventory(SOURCE);copy_after=fs.file_inventory(copy)
 save('source-after.json',after);save('copy-after.json',copy_after)
 if before!=after or before!=copy_after:raise ValueError('input or copy changed')
 save('verdict.json',dict(verdict='FILESYSTEM_INVENTORY_ONLY',native_open=False,lucivy_decode_qualified=False,files=len(before),source=str(SOURCE),copy=str(copy)))
if __name__=='__main__':
 try:main()
 except Exception as error:
  if ROOT.is_dir() and not (ROOT/'verdict.json').exists():save('verdict.json',dict(verdict='RED',error=str(error)))
  raise
