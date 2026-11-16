#!/usr/bin/env bash
set -euo pipefail
out="$HOME/.cache/rag3db-moteur-notes/c1-offsets-02"
scratch="$HOME/.cache/rag3weaver-build/cx-coeur-c1-offsets"
[[ -d "$out" && -f "$out/controller.py" && ! -e "$out/limits.json" ]]
[[ -d "$scratch" && -z "$(ls -A "$scratch")" ]]
# C7 distinct corrective admission1800:1790+10stop/controller1770; first failure stops.
POSTE_MEM_MAX=32G POSTE_SWAP_MAX=0 exec "$HOME/.cache/rag3weaver-build/poste" lourd \
 systemd-run --user --wait --pipe --unit=cx-coeur-c1-offsets-02 \
 --property=MemoryMax=34359738368 --property=MemorySwapMax=0 \
 --property=RuntimeMaxSec=1790 --property=TimeoutStopSec=10 --property=KillMode=control-group \
 --property=Nice=19 --property=IOSchedulingClass=idle \
 bwrap --unshare-user --unshare-net --ro-bind / / --proc /proc --dev /dev \
 --size 34359738368 --tmpfs "$scratch" --bind "$out" "$out" \
 /usr/bin/env -i PATH=/usr/bin:/bin LANG=C LC_ALL=C TZ=UTC \
 /usr/bin/python3 "$out/controller.py" "$scratch" "$out" \
 "$HOME/git_workspaces/rag3db-colonnes" "$out/source.bundle" "$out/recipe.json"
