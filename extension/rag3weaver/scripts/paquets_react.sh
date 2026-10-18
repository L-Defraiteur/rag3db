#!/bin/bash
# Les paquets npm de React que le témoin de l'aperçu à chaud range dans un backend
# (src/daemon_host/apercu_a_chaud.rs), déballés dans <dossier>/<paquet>/ :
#
#   scripts/paquets_react.sh <dossier>
#   RAG3WEAVER_PAQUETS_REACT=<dossier> cargo test --lib ... apercu
#
# Les versions sont fixées : react-dom 19.2.0 dépend de scheduler 0.27.0, et
# `libs add` vérifie les versions des dépendances contre libs.json.
set -euo pipefail
dossier=${1:?usage : paquets_react.sh <dossier>}
mkdir -p "$dossier"
cd "$dossier"
for spec in react@19.2.0 react-dom@19.2.0 scheduler@0.27.0 react-refresh@0.18.0; do
    nom=${spec%@*}
    [ -f "$nom/package.json" ] && continue
    archive=$(npm pack "$spec" --silent | tail -1)
    rm -rf "$nom.tmp" && mkdir "$nom.tmp"
    tar xzf "$archive" -C "$nom.tmp"
    mv "$nom.tmp/package" "$nom"
    rm -rf "$nom.tmp" "$archive"
done
ls "$dossier"
