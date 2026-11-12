#!/usr/bin/env bash
# Même ticket annulation. Aucun worktree/configuration créé par ce wrapper.
set -euo pipefail
if [ "$#" -ne 5 ]; then
  echo 'usage: cx-lancer-archive-privee.sh SOURCE BUILD_ANCRE RESULTATS REFERENCE UNITE' >&2
  exit 2
fi
source_dir=$1
build_anchor=$2
output_dir=$3
reference_dir=$4
unit_name=$5
case "$unit_name" in cx-coeur-archive-*) ;; *) echo 'unité hors périmètre' >&2; exit 2 ;; esac
for task_dir in "$source_dir" "$build_anchor" "$output_dir" "$reference_dir"; do
  test -d "$task_dir"
done
test -f "$output_dir/cx-build-archive-privee.py"
test ! -e "$output_dir/autoriser-build.json"
test ! -e "$output_dir/revue.json"
# La durée inclut configuration + attente de revue + build + sauvegarde.
# Le verrou se prend avant le démarrage de cette horloge ; il reste tenu jusqu'à fin.
exec "$HOME/.cache/rag3weaver-build/poste" lourd \
  systemd-run --user --wait --pipe --unit="$unit_name" \
  --property=MemoryMax=8589934592 --property=MemorySwapMax=0 \
  --property=RuntimeMaxSec=1740 --property=TimeoutStopSec=5 \
  --property=KillMode=control-group --property=Nice=19 --property=IOSchedulingClass=idle \
  bwrap --unshare-user --unshare-net --ro-bind / / --proc /proc --dev /dev \
  --size 2147483648 --tmpfs "$build_anchor" \
  --bind "$output_dir" "$output_dir" --chdir "$source_dir" \
  /usr/bin/python3 "$output_dir/cx-build-archive-privee.py" \
  --source "$source_dir" --build "$build_anchor" --output "$output_dir" \
  --reference "$reference_dir" --unit "$unit_name"
