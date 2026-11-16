#!/usr/bin/env python3
"""Pinned read-only two-direction checker on another fresh private snapshot copy."""
from pathlib import Path
import argparse,hashlib,importlib.util,json,os,re,stat,subprocess,time
HOME=Path.home();HERE=Path(__file__).resolve().parent
TOOL=HOME/'.cache/optimiseur-notes/bascule-16nov-927/bin/check_rel_directions-luciepc'
LIB=HOME/'.cache/rag3db-libs/rag3db-build-59ac9c86bc2b3529b590a7462ab7051e3ad26b8a-Release-p12-v11-g17-s18-rcOFF-stOFF-asanOFF-tsanOFF-ubsanOFF'
def digest(path):
 h=hashlib.sha256()
 with path.open('rb') as stream:
  for chunk in iter(lambda:stream.read(1048576),b''):h.update(chunk)
 return h.hexdigest()
def parse_directions(text):
 tables=re.findall(r'^([A-Za-z_][A-Za-z0-9_]*) : [0-9]+ étiquettes sources, [0-9]+ destinations ; lectures avant ([0-9]+), arrière ([0-9]+) — égales$',text,re.M)
 counts=re.findall(r'^    comptes : avant ([0-9]+), arrière ([0-9]+), union ([0-9]+), perdues d.un seul sens ([0-9]+)$',text,re.M)
 if len(tables)!=18 or len({t[0] for t in tables})!=18 or len(counts)!=18:raise ValueError('incomplete 18-table observation')
 if any(a!=b or a!=c or d!='0' for a,b,c,d in counts):raise ValueError('direction counts disagree')
 if any(t[1]!=c[0] or t[2]!=c[1] for t,c in zip(tables,counts)):raise ValueError('raw versus distinct direction counts disagree')
 if not text.endswith('Aucun écart entre les deux rangements.\n'):raise ValueError('missing final checker verdict')
 return ([dict(table=t[0],forward=int(c[0]),backward=int(c[1]),union=int(c[2]),lost=int(c[3])) for t,c in zip(tables,counts)])

def main():
 ap=argparse.ArgumentParser();ap.add_argument('--base',choices=['code-principal','code-lourd'],required=True);ap.add_argument('--sets-verdict-sha',required=True);ap.add_argument('--source-inventory-sha',required=True);args=ap.parse_args()
 if digest(TOOL)!='554b08c031e26de725682cf335c436093248d62f96badab29097c0a86c4a57da':raise ValueError('checker pin mismatch')
 if digest(LIB/'librag3db.so')!='b7e9fc75a3d2de04c72680b0309e8bb1df9eaadb5211ebbacf39716c6d0a2101':raise ValueError('lib pin mismatch')
 if digest(HERE/'inspect-fts.py')!='b846b0991999040c04f298747eda27aa07a4aa9ad0c277f8b7dc485eb24465a8':raise ValueError('helper pin mismatch')
 previous=HOME/'.cache/rag3weaver-build/bascule-live-20261116/fresh-sets-01'/args.base
 if digest(previous/'verdict.json')!=args.sets_verdict_sha or digest(previous/'source-before.json')!=args.source_inventory_sha:raise ValueError('prior proof pin mismatch')
 verdict=json.loads((previous/'verdict.json').read_text())
 if verdict.get('verdict')!='FRESH_ID_SETS_EQUAL' or verdict.get('base')!=args.base:raise ValueError('fresh sets not green')
 provenance=json.loads((previous/'provenance.json').read_text());source=Path(provenance['source']).resolve(strict=True)
 allowed=(HOME/'.local/share/rag3weaver/instantanes').resolve(strict=True)
 if not source.is_relative_to(allowed) or source.name!=args.base:raise ValueError('source is not cold snapshot')
 if digest(Path(provenance['g_receipt']))!=provenance['g_receipt_sha256']:raise ValueError('G receipt changed')
 expected=json.loads((previous/'source-before.json').read_text())
 root=HOME/'.cache/rag3weaver-build/bascule-live-20261116/fresh-directions-01'/args.base;root.mkdir(mode=0o700,parents=True,exist_ok=False)
 def save(name,value):
  with (root/name).open('x') as stream:json.dump(value,stream,ensure_ascii=False,indent=2)
 start=time.monotonic();end=start+1780
 save('provenance.json',dict(base=args.base,source=str(source),sets_verdict_sha256=args.sets_verdict_sha,source_inventory_sha256=args.source_inventory_sha,tool=str(TOOL),tool_sha256=digest(TOOL)))
 try:
  spec=importlib.util.spec_from_file_location('inspect_fts',HERE/'inspect-fts.py');fs=importlib.util.module_from_spec(spec);spec.loader.exec_module(fs)
  before=fs.file_inventory(source);save('source-before.json',before)
  if before!=expected:raise ValueError('snapshot changed since ID comparison')
  copy=root/'copy';subprocess.run(['cp','-a','--reflink=always','--',str(source),str(copy)],check=True,timeout=90)
  if fs.file_inventory(copy)!=before:raise ValueError('copy mismatch')
  env=dict(os.environ,LD_LIBRARY_PATH=str(LIB));first_error=None
  try:
   remaining=end-time.monotonic()
   if remaining<=30:raise ValueError('deadline before checker')
   try:result=subprocess.run([str(TOOL),str(copy/(args.base+'.rag3db'))],capture_output=True,timeout=min(1700,remaining-30),env=env)
   except subprocess.TimeoutExpired as error:
    (root/'directions.stdout').write_bytes(error.stdout or b'');(root/'directions.stderr').write_bytes(error.stderr or b'');save('process.json',dict(rc=None,timeout=True));raise
   (root/'directions.stdout').write_bytes(result.stdout);(root/'directions.stderr').write_bytes(result.stderr);save('process.json',dict(rc=result.returncode))
   if result.returncode:raise ValueError('two-direction checker refused')
   save('tables.json',parse_directions(result.stdout.decode()))
  except Exception as error:first_error=error
  try:
   after=fs.file_inventory(copy);source_after=fs.file_inventory(source);save('copy-after.json',after);save('source-after.json',source_after)
   old={r['path']:r for r in before};new={r['path']:r for r in after};name=args.base+'.rag3db.readers';path=copy/name;created=name not in old and name in new
   if path.exists() or path.is_symlink():
    info=path.lstat()
    if not stat.S_ISREG(info.st_mode) or info.st_uid!=1000 or info.st_size!=0:raise ValueError('invalid readers sidecar')
   if created:
    row=new.pop(name)
    if row['sha256']!=hashlib.sha256(b'').hexdigest():raise ValueError('nonempty readers')
   save('sidecar.json',dict(path=name,created=created))
   if old!=new or before!=source_after:raise ValueError('snapshot or copy changed')
  except Exception as audit_error:
   save('audit-error.json',dict(error=str(audit_error)))
   if first_error is None:first_error=audit_error
  if first_error:raise first_error
  if digest(TOOL)!='554b08c031e26de725682cf335c436093248d62f96badab29097c0a86c4a57da' or time.monotonic()>=end:raise ValueError('tool changed or deadline')
  save('verdict.json',dict(verdict='FRESH_18_DIRECTIONS_EQUAL',base=args.base,elapsed=time.monotonic()-start,purge_qualified=False,cli_authorized=False))
 except Exception as error:save('verdict.json',dict(verdict='RED',error=str(error)));raise
if __name__=='__main__':main()
