#!/bin/bash
# L'arbre de master que l'hôte de luciepc indexe (21 novembre 2026, c7 et Lucie : « faut que ça vous devienne vraiment
# utile ») : chaque minute, par index-master-ff.timer, `git fetch` puis `git merge --ff-only origin/master` sur la branche
# index-master de cet arbre, et rien d'autre. Personne n'y commite. Jamais de reset, jamais de checkout : un arbre
# modifié, une autre branche, ou un ff refusé (un commit local) est écrit au journal et signalé par le fichier
# index-master-ff.refused ; un fetch en échec est écrit et ne touche à rien. Le ff suivant qui passe efface le signal.
# Le journal ne dit que ce qui change (un ff, un refus, un échec) : rien les minutes où master n'a pas bougé.
# Usage : index_master_ff.sh <arbre>   (INDEX_MASTER_STATE : le dossier du journal et du signal, ~/.cache/rag3weaver-build)
set -u
A=${1:?usage: index_master_ff.sh <arbre>}
S=${INDEX_MASTER_STATE:-$HOME/.cache/rag3weaver-build}
mkdir -p "$S"
L=$S/index-master-ff.log
R=$S/index-master-ff.refused
dit() { printf '%s\t%s\n' "$(date +%FT%T)" "$1" >> "$L"; }
refus() { dit "$1"; printf '%s\t%s\n' "$(date +%FT%T)" "$1" > "$R"; echo "$1" >&2; exit 0; }
exec 9>> "$S/index-master-ff.lock"
flock -n 9 || exit 0
g() { git -C "$A" "$@"; }
[ "$(g branch --show-current 2>/dev/null)" = index-master ] || refus "refusé : l'arbre $A n'est pas sur index-master ($(g branch --show-current 2>/dev/null || echo illisible)) ; rien n'est touché"
[ -z "$(g status --porcelain --untracked-files=no 2>/dev/null)" ] || refus "refusé : l'arbre $A est modifié ; personne n'y commite ni n'y écrit, rien n'est touché"
if ! timeout 60 git -C "$A" fetch -q origin master 2> "$S/index-master-ff.fetch.err"; then
  dit "fetch en échec : $(head -1 "$S/index-master-ff.fetch.err") ; l'arbre reste à $(g rev-parse --short HEAD)"
  exit 0
fi
avant=$(g rev-parse HEAD); cible=$(g rev-parse origin/master)
if [ "$avant" = "$cible" ]; then rm -f -- "$R"; exit 0; fi
if g merge -q --ff-only origin/master > "$S/index-master-ff.merge.err" 2>&1; then
  dit "ff ${avant:0:10}..${cible:0:10} ($(g rev-list --count "$avant..$cible") commit(s))"
  rm -f -- "$R"
else
  refus "ff refusé : index-master (${avant:0:10}) n'est pas un ancêtre d'origin/master (${cible:0:10}) — $(head -1 "$S/index-master-ff.merge.err") ; rien n'est réinitialisé"
fi
