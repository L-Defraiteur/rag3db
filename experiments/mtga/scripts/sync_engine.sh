#!/usr/bin/env bash
# Ingest the existing local snapshot. Run refresh.sh first to recapture Arena.
set -euo pipefail
cd "$(dirname "$0")/../../.."
python3 experiments/mtga/scripts/prepare_engine_collection.py
python3 experiments/mtga/scripts/engine_collection.py --ingest --ingest-relations --ingest-catalog
