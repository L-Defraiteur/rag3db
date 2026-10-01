#!/usr/bin/env bash
# Hôte JSONL du backend MTG, pour rag3weaver-chat (backend_command). Même
# environnement que serve_engine_mcp.sh ; la base ne s'ouvre qu'une fois :
# pas en même temps que le serveur MCP de llama-server.
set -euo pipefail
root="$(cd "$(dirname "$0")/../../.." && pwd)"
export LD_LIBRARY_PATH="$root/build/lecteurs-csv/src${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
export RAG3DB_BUFFER_POOL_SIZE="${RAG3DB_BUFFER_POOL_SIZE:-16106127360}"
export RAG3DB_MAX_DB_SIZE=68719476736
export RAG3WEAVER_RENDER_TEMPLATES="$root/experiments/mtga/backend/render"
exec "${RAG3WEAVER_BACKEND_BIN:-$root/extension/rag3weaver/target/release/rag3weaver-backend}" "$root/experiments/mtga/backend/backend.json"
