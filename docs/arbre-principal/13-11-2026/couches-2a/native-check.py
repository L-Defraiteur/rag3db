#!/usr/bin/env python3
"""Record the real exported build environment, then exec Cargo (no SDK runtime)."""
from pathlib import Path
import hashlib,json,os,sys
NATIVE='rag3db-build-29a7bc52aaf11e6204f9beafaffb154105417e13-Release-p12-v11-g17-s18-rcOFF-stOFF-asanOFF-tsanOFF-ubsanOFF'
root=Path('/home/lucied/.cache/rag3db-libs')/NATIVE;proof=Path(sys.argv[1]);out=proof/'native-check-env.json';assert not out.exists()
expected=dict(RAG3DB_LIB=NATIVE,RAG3DB_SHARED='1',RAG3DB_LIBRARY_DIR=str(root),RAG3DB_INCLUDE_DIR=str(root),LD_LIBRARY_PATH=str(root),RAG3DB_BUFFER_POOL_SIZE='1073741824')
actual={key:os.environ[key] for key in expected};assert actual==expected
with (root/'librag3db.so').open('rb') as f:sha=hashlib.file_digest(f,'sha256').hexdigest()
assert sha=='b014febe51417ca535218d1c0b18308e7d301157abf45027a5cbba70dfc54929'
out.write_text(json.dumps(dict(pid=os.getpid(),starttime=Path('/proc/self/stat').read_text().rsplit(') ',1)[1].split()[19],cgroup=Path('/proc/self/cgroup').read_text(),env=actual,library_sha256=sha,meaning='build environment only, not SDK execution or DB open'),indent=2))
assert sys.argv[2:]==['cargo','check','--lib','--no-default-features','--features','rag3db-native']
os.execvpe('cargo',sys.argv[2:],os.environ.copy())
