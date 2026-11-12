#!/usr/bin/env bash
set -euo pipefail
pilot_dir="$HOME/.cache/rag3db-moteur-notes/annulation/cx-pilote-retrait-0511dece7"
exec "$HOME/.cache/rag3weaver-build/poste" mesure \
 systemd-run --user --wait --pipe --unit=cx-coeur-pilote-mesure-12nov-01 \
 --property=MemoryMax=1073741824 --property=MemorySwapMax=0 \
 --property=RuntimeMaxSec=890 --property=TimeoutStopSec=2 \
 --property=KillMode=control-group --property=Nice=19 --property=IOSchedulingClass=idle \
 bwrap --unshare-user --unshare-net --ro-bind / / --proc /proc --dev /dev \
 --bind "$pilot_dir" "$pilot_dir" --chdir "$pilot_dir" \
 /usr/bin/python3 "$pilot_dir/cx-controler-mesure-validee.py" --unit cx-coeur-pilote-mesure-12nov-01
