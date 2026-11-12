#!/usr/bin/env bash
set -euo pipefail
if [ "$#" -ne 3 ]; then echo 'usage: PHASE ESPACE_NEUF UNITE' >&2; exit 2; fi
phase=$1
pilot_dir=$2
unit_name=$3
case "$phase" in
 compile) lock=lourd; mem=8589934592; runtime=170 ;;
 measure) lock=mesure; mem=1073741824; runtime=890 ;;
 *) exit 2 ;;
esac
case "$unit_name" in cx-coeur-pilote-*) ;; *) exit 2 ;; esac
test -f "$pilot_dir/cx-controler-pilote.py"
exec "$HOME/.cache/rag3weaver-build/poste" "$lock" \
 systemd-run --user --wait --pipe --unit="$unit_name" \
 --property=MemoryMax="$mem" --property=MemorySwapMax=0 \
 --property=RuntimeMaxSec="$runtime" --property=TimeoutStopSec=2 \
 --property=KillMode=control-group --property=Nice=19 --property=IOSchedulingClass=idle \
 bwrap --unshare-user --unshare-net --ro-bind / / --proc /proc --dev /dev \
 --bind "$pilot_dir" "$pilot_dir" --chdir "$pilot_dir" \
 /usr/bin/python3 "$pilot_dir/cx-controler-pilote.py" "$phase" --unit "$unit_name"
