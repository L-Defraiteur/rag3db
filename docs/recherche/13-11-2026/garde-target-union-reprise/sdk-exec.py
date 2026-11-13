#!/usr/bin/env python3
"""Record one actual SDK export then exec the reviewed Cargo/ELF invocation."""
from pathlib import Path
import hashlib,json,os,sys
NATIVE='rag3db-build-29a7bc52aaf11e6204f9beafaffb154105417e13-Release-p12-v11-g17-s18-rcOFF-stOFF-asanOFF-tsanOFF-ubsanOFF'
F='rag3db-native,burn-embedder,burn-ocr,code,daemon,postgres,openai-llm,anthropic-llm'
root=Path('/home/lucied/.cache/rag3db-libs')/NATIVE;target=Path('/home/lucied/.cache/rag3weaver-build/cx-target-garde-target-qualification-v1');proof=Path(sys.argv[1]);phase=sys.argv[2];command=sys.argv[3:]
assert phase in ['large-list','large-ignored','nav-list','nav-ignored','lib-large']
assert Path(command[0]).is_relative_to(target) and Path(command[0]).is_file()
if phase.startswith('nav-'):assert command[-(3 if phase=='nav-list' else 4):]==(['--list','--format','terse'] if phase=='nav-list' else ['--list','--ignored','--format','terse'])
else:
 suffixes={'large-list':['--list','--format','terse'],'large-ignored':['--list','--ignored','--format','terse'],'lib-large':['--show-output','--test-threads=8']}
 assert command[1:]==suffixes[phase]
expected=dict(RAG3DB_LIB=NATIVE,RAG3DB_SHARED='1',RAG3DB_LIBRARY_DIR=str(root),RAG3DB_INCLUDE_DIR=str(root),LD_LIBRARY_PATH=str(root),RAG3DB_BUFFER_POOL_SIZE='1073741824',RAG3WEAVER_MOTEUR_ANCIEN='1')
actual={key:os.environ[key] for key in expected};assert actual==expected
with (root/'librag3db.so').open('rb') as f:sha=hashlib.file_digest(f,'sha256').hexdigest()
assert sha=='b014febe51417ca535218d1c0b18308e7d301157abf45027a5cbba70dfc54929'
out=proof/('sdk-env-'+phase+'.json');assert not out.exists()
out.write_text(json.dumps(dict(pid=os.getpid(),starttime=Path('/proc/self/stat').read_text().rsplit(') ',1)[1].split()[19],cgroup=Path('/proc/self/cgroup').read_text(),env=actual,library_sha256=sha,command=command,meaning='actual exported environment; runtime loading must be observed separately'),indent=2))
os.execvpe(command[0],command,os.environ.copy())
