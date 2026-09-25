#!/usr/bin/env bash
set -euo pipefail
root="$(cd "$(dirname "$0")/../../.." && pwd)"
export LD_LIBRARY_PATH="$root/build/lecteurs/src${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
export RAG3DB_BUFFER_POOL_SIZE=536870912
export RAG3DB_MAX_DB_SIZE=17179869184
exec "$root/extension/rag3weaver/target/debug/rag3daemon" \
  --adresse 127.0.0.1:8732 --base "$root/experiments/mtga/data/rag3db"
