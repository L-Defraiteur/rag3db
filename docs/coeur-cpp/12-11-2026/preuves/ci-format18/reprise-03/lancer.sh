#!/usr/bin/env bash
# Proposition pour revue C7 : ne pas invoquer avant son GO et libération BancMESURE.
set -euo pipefail
out="${1:?Répertoire de preuves neuf préparé avec controleur.py relu}"
[[ "$out" = "$HOME/.cache/rag3db-moteur-notes/ci-format18-74ad-03" ]]
[[ -f "$out/controleur.py" && ! -e "$out/limites.json" ]]
previous="$HOME/.cache/rag3db-moteur-notes/ci-format18-74ad-02"
scratch="$HOME/.cache/rag3weaver-build/cx-ci-format18-74ad-03"
# Le répertoire point de montage doit être vide et préparé lors du staging autorisé.
[[ -d "$scratch" && -z "$(ls -A "$scratch")" ]]
exec "$HOME/.cache/rag3weaver-build/poste" lourd systemd-run --user --wait --pipe \
 --unit=cx-coeur-ci-format18-74ad-03 \
 --property=MemoryMax=34359738368 --property=MemorySwapMax=0 \
 --property=RuntimeMaxSec=1480 --property=TimeoutStopSec=10 --property=KillMode=control-group \
 --property=Nice=19 --property=IOSchedulingClass=idle \
 bwrap --unshare-user --unshare-net --ro-bind / / --proc /proc --dev /dev \
 --size 34359738368 --tmpfs "$scratch" --bind "$out" "$out" \
 /usr/bin/env -i PATH=/usr/bin:/bin LANG=C LC_ALL=C TZ=UTC \
 GIT_OPTIONAL_LOCKS=0 GIT_TERMINAL_PROMPT=0 GIT_ALLOW_PROTOCOL=file \
 /usr/bin/python3 "$out/controleur.py" "$scratch" "$out" "$previous" \
 "$HOME/git_workspaces/rag3db-colonnes"
