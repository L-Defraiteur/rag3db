#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
node scripts/collect.cjs
.venv/bin/python scripts/build.py
