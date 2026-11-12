#!/usr/bin/env bash
# Meme ticket annulation. Preflight sans test, puis une tentative runtime autorisee.
set -euo pipefail
mode=${1:?preflight ou run}
out="$HOME/.cache/rag3db-moteur-notes/annulation/cx-runtime-saturation-12nov-01"
package="$HOME/.cache/rag3db-moteur-notes/annulation/cx-build-saturation-12nov-02/package"
build_evidence="$HOME/.cache/rag3db-moteur-notes/annulation/cx-build-saturation-12nov-02"
anchor="$HOME/.cache/rag3weaver-build/cx-build-saturation-12nov-02"
case "$mode" in
 preflight) unit=cx-coeur-runtime-saturation-preflight-12nov-01; poste=lourd; duration=30; stop=5 ;;
 run)
  unit=cx-coeur-runtime-saturation-12nov-01; poste=mesure; duration=1200; stop=10
  test -f "$out/preflight-ok.json"
  test -f "$out/publication.json"
  test ! -e "$out/attempt.json"
  ;;
 *) exit 2 ;;
esac
exec "$HOME/.cache/rag3weaver-build/poste" "$poste" systemd-run --user --wait --pipe \
 --unit="$unit" --property=MemoryMax=8589934592 --property=MemorySwapMax=0 \
 --property=RuntimeMaxSec="$duration" --property=TimeoutStopSec="$stop" \
 --property=KillMode=control-group --property=Restart=no \
 bwrap --unshare-user --unshare-net --ro-bind / / --proc /proc --dev /dev \
 --size 2147483648 --tmpfs "$anchor" --dir "$anchor/source" \
 --ro-bind "$package" "$anchor/source" --dir "$anchor/runtime" \
 --bind "$out" "$out" --chdir "$anchor/runtime" \
 /usr/bin/env -i PATH=/usr/bin:/bin LANG=C LC_ALL=C PYTHONDONTWRITEBYTECODE=1 \
 /usr/bin/python3 "$out/cx-runtime-saturation.py" "$mode" "$package" "$out" "$anchor" \
 "$build_evidence/manifest.json" "$build_evidence/revue-manifeste-elf.json"
