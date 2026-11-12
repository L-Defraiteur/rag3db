#!/bin/bash
set -euo pipefail
cd /home/lucied/.cache/rag3weaver-build/cx-wt-faits-lies
test -z "$(git status --porcelain)"
test "$(git rev-parse HEAD)" = d2cb7f0636d099823f63de2cef674b630a46bc6f
reste=$(python3 - <<'PY'
import json,time
p='/home/lucied/.cache/rag3weaver-build/cx-passe-faits-lies-v1/source.json'
print(max(0,int(json.load(open(p))['deadline_monotonic']-time.monotonic()-5)))
PY
)
[ "$reste" -gt 30 ]
cd /var/tmp
systemd-run --user --unit=cx-passe-faits-lies-v5.service --wait --pipe --collect \
 -p WorkingDirectory=/var/tmp -p KillMode=control-group -p MemoryMax=32G -p MemoryHigh=infinity -p RuntimeMaxSec="${reste}s" -p TimeoutStopSec=5s \
 /usr/bin/bash -lc 'POSTE_MEM_MAX=32G /home/lucied/.cache/rag3weaver-build/cx-wt-faits-lies/extension/rag3weaver/scripts/poste lourd bash /home/lucied/.cache/rag3weaver-build/cx-reprise-faits-lies-v5.sh d2cb7f0636d099823f63de2cef674b630a46bc6f'
