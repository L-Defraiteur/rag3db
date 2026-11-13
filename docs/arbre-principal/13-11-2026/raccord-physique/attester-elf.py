#!/usr/bin/env python3
"""Attestation RO de l'ELF v3 : aucun Cargo ni exécution du binaire."""
from pathlib import Path
import hashlib,json,mmap,subprocess,time,os
W=Path('/home/lucied/.cache/rag3weaver-build/cx-wt-raccord-physique')
C=W/'extension/rag3weaver'
P=Path('/home/lucied/.cache/rag3weaver-build/cx-raccord-physique-cible-v3')
OUT=Path('/home/lucied/.cache/rag3weaver-build/cx-raccord-physique-attestation-v3.json')
SOURCE='10e6c75270a93c4db7a5d42c69c9dbd5d658f46e'
EXPECTED='8387de3dd95ef2c3d290166d6d2049c0a4cf9f4cb29ace565da362b9b8a9426d'
NATIVE='rag3db-build-29a7bc52aaf11e6204f9beafaffb154105417e13-Release-p12-v11-g17-s18-rcOFF-stOFF-asanOFF-tsanOFF-ubsanOFF'
sha=lambda p:hashlib.file_digest(p.open('rb'),'sha256').hexdigest()
authority=json.loads(Path(os.environ['RAG3WEAVER_VALIDATION_RECU']).read_text())
assert time.monotonic()<authority['echeance']
assert not OUT.exists()
assert not subprocess.check_output(['git','status','--porcelain'],cwd=W,text=True).strip()
assert not subprocess.check_output(['git','diff',SOURCE,'HEAD','--','extension/rag3weaver'],cwd=W)
artifacts=[]
for line in (P/'build.log').read_text().splitlines():
 try:m=json.loads(line)
 except ValueError:continue
 if m.get('reason')=='compiler-artifact' and m.get('executable') and m.get('profile',{}).get('test') and m['target']['name']=='rag3weaver':artifacts.append(m)
assert len({m['executable'] for m in artifacts})==1
binary=Path(artifacts[0]['executable']);copy=P/'elf'/binary.name
assert sha(binary)==sha(copy)==EXPECTED
manifest=[]
for p in sorted((C/'templates').rglob('*')):
 if p.is_file():
  rel=p.relative_to(C/'templates');assert sha(p)==sha(P/'builtin_templates'/rel)
  manifest.append(dict(path=str(rel),sha256=sha(p),size=p.stat().st_size))
assert manifest==json.loads((P/'templates.json').read_text())
with binary.open('rb') as f,mmap.mmap(f.fileno(),0,access=mmap.ACCESS_READ) as data:
 offset=data.find(SOURCE[:9].encode());assert offset>=0
 embedded={}
 for name in ['tools/search_base.mmd','search_expansion.mmd']:
  payload=(C/'templates'/name).read_bytes();assert payload
  found=data.find(payload);assert found>=0
  embedded[name]=dict(offset=found,sha256=hashlib.sha256(payload).hexdigest(),size=len(payload))
native=Path('/home/lucied/.cache/rag3db-libs')/NATIVE/'librag3db.so'
assert native.is_file()
old=json.loads((P/'result.json').read_text());assert old['verdict']=='ROUGE' and old['source']==SOURCE
assert len(json.loads((P/'phases.json').read_text()))==1
result=dict(verdict='VERT_ATTESTATION_RO',source=SOURCE,recipe_source=subprocess.check_output(['git','rev-parse','HEAD'],cwd=W,text=True).strip(),binary=str(binary),copy=str(copy),sha256=EXPECTED,source_offset=offset,embedded=embedded,templates=manifest,native=str(native),native_sha256=sha(native),build_log_sha256=sha(P/'build.log'),compiler_artifact=artifacts[0],prior_result=old,prior_deadline=old['deadline'],attestation_deadline=authority['echeance'],finished=time.monotonic(),tests_executed=0)
assert time.monotonic()<authority['echeance']
OUT.write_text(json.dumps(result,ensure_ascii=False,indent=2))
print(json.dumps({k:v for k,v in result.items() if k not in ['templates','compiler_artifact']}))
