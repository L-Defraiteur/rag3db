#!/usr/bin/env bash
set -euo pipefail
pilot_dir="$HOME/.cache/rag3db-moteur-notes/annulation/cx-pilote-retrait-0511dece7"
point_out="$pilot_dir/quatre-points-12nov-01"
exec "$HOME/.cache/rag3weaver-build/poste" mesure \
 systemd-run --user --wait --pipe --unit=cx-coeur-quatre-points-12nov-01 \
 --property=MemoryMax=1073741824 --property=MemorySwapMax=0 \
 --property=RuntimeMaxSec=890 --property=TimeoutStopSec=2 \
 --property=KillMode=control-group --property=Nice=19 --property=IOSchedulingClass=idle \
 bwrap --unshare-user --unshare-net --ro-bind / / --proc /proc --dev /dev \
 --bind "$point_out" "$point_out" --chdir "$point_out" \
 /usr/bin/python3 "$pilot_dir/cx-controler-quatre-points.py" --unit cx-coeur-quatre-points-12nov-01
