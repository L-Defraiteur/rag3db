#!/bin/bash
# **Les libs de la page de l'IDE** (l'éditeur CodeMirror, 19 octobre 2026), rangées
# dans templates/ide/ par `libs add` : libs.json et le magasin .libs/<blake3>. Le
# binaire les embarque (src/builtin_templates.rs) et les sert par
# /ide/editeur/modules.js : ni CDN, ni téléchargement à l'usage.
#
#   scripts/ide_libs.sh <rag3weaver-backend>
#
# Les versions sont fixées : chaque dépendance déclarée (^…) doit être tenue par la
# copie rangée, et `libs add` ne garde qu'une version par paquet. Pour monter de
# version : changer la liste, relancer, relire le diff de templates/ide/libs.json.
set -euo pipefail
binaire=${1:?usage : ide_libs.sh <rag3weaver-backend>}
ici=$(cd "$(dirname "$0")/.." && pwd)
dossier=$ici/templates/ide
mkdir -p "$dossier"
travail=$(mktemp -d -p "${TMPDIR:-$HOME/.cache}")
trap 'rm -rf "$travail"' EXIT
rm -rf "$dossier/.libs" "$dossier/libs.json"
for spec in \
    @codemirror/view@6.43.14 \
    @codemirror/state@6.7.6 \
    @codemirror/commands@6.11.1 \
    @codemirror/language@6.13.1 \
    @codemirror/search@6.7.2 \
    @codemirror/streamparser@6.0.0 \
    @lezer/common@1.5.3 \
    @lezer/highlight@1.2.5 \
    @lezer/lr@1.4.11 \
    @marijn/find-cluster-break@1.0.4 \
    crelt@1.0.7 \
    style-mod@4.1.4 \
    w3c-keyname@2.2.8; do
    nom=${spec%@*}
    ( cd "$travail" && npm pack "$spec" --silent > /dev/null )
    archive=$(ls "$travail"/*.tgz)
    mkdir -p "$travail/paquet"
    tar xzf "$archive" -C "$travail/paquet"
    "$binaire" libs add "$dossier/libs.json" "$travail/paquet/package"
    rm -rf "$archive" "$travail/paquet"
done
# Le magasin réduit à ce que le graphe de l'éditeur atteint (src/ide.rs, EDITEUR),
# plus les licences : ni l'ESM d'un paquet dont on prend le CommonJS, ni ce qu'aucun
# module n'importe.
"$binaire" libs prune "$dossier/libs.json" @codemirror/view @codemirror/state @codemirror/commands @codemirror/search @codemirror/language
echo "$(ls "$dossier/.libs" | wc -l) objets, $(du -sh "$dossier/.libs" | cut -f1) dans $dossier/.libs"
