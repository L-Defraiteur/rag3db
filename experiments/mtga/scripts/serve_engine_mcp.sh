#!/usr/bin/env bash
set -euo pipefail
root="$(cd "$(dirname "$0")/../../.." && pwd)"
export LD_LIBRARY_PATH="$root/build/lecteurs-csv/src${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
export RAG3DB_BUFFER_POOL_SIZE="${RAG3DB_BUFFER_POOL_SIZE:-16106127360}"
export RAG3DB_MAX_DB_SIZE=68719476736
export RAG3WEAVER_RENDER_TEMPLATES="$root/experiments/mtga/backend/render"
exec "$root/experiments/mtga/.venv/bin/python" "$root/extension/rag3weaver/scripts/serve_backend_mcp.py" "$root/experiments/mtga/backend/backend.json" --response-format text
