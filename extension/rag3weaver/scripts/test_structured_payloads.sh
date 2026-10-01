#!/usr/bin/env bash
set -euo pipefail
root="$(cd "$(dirname "$0")/../../.." && pwd)"
cd "$root"
export LD_LIBRARY_PATH="$root/build/lecteurs-csv/src${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
export RAG3DB_BUFFER_POOL_SIZE=1073741824
export RAG3DB_MAX_DB_SIZE=4294967296
export RAG3WEAVER_TEST_DAEMON=127.0.0.1:8736
export RAG3WEAVER_TEST_REPORT="$root/experiments/mtga/data/composed-search-report.json"
export RAG3WEAVER_TEST_MTGA="$root/experiments/mtga/data"
export RAG3WEAVER_TEST_EMBEDDINGS=127.0.0.1:7878
export RAG3WEAVER_TEST_VECTOR_EXTENSION="$root/extension/vector/build/libvector.rag3db_extension"
log="$(mktemp /tmp/rag3weaver-structured.XXXXXX.log)"
extension/rag3weaver/target/debug/rag3daemon --adresse "$RAG3WEAVER_TEST_DAEMON" --base :memoire: >"$log" 2>&1 &
demon_pid=$!
trap 'kill "$demon_pid" 2>/dev/null || true; wait "$demon_pid" 2>/dev/null || true' EXIT
python3 - <<'PY'
import os,time,urllib.request
for attempt in range(100):
    try:
        with urllib.request.urlopen('http://'+os.environ['RAG3WEAVER_TEST_DAEMON']+'/sante', timeout=1): break
    except OSError: time.sleep(.1)
else: raise SystemExit('test daemon unavailable')
PY
cargo test --offline --manifest-path extension/rag3weaver/Cargo.toml --features daemon --test structured_payloads -- --ignored --nocapture --test-threads=1
