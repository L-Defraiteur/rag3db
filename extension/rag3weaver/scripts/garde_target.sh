# À sourcer (run_e2e.sh, scripts/check_features.sh) : `garder_le_target <target> <arbre>`.
#
# **Un target ne sert qu'un arbre à la fois.** Cargo note les sources d'un
# paquet local par des chemins relatifs à l'espace de travail (`ir/src/lib.rs`)
# et juge un artefact frais par les dates : sous un target partagé entre deux
# worktrees, l'arbre aux fichiers plus anciens reprend l'artefact bâti par
# l'autre. Codeparsers l'a vu le 11 octobre 2026 (« Plan introuvable » alors
# que l'ir du worktree le définit) : un rouge de cette sorte est faux, un vert
# peut l'être aussi.
#
# Le target garde dans `.arbre-du-target` la racine de l'arbre qui l'a bâti.
# Un autre arbre est refusé, nommément ; `RAG3WEAVER_TARGET_REPRENDRE=1` efface
# les artefacts des paquets locaux (pas les dépendances) et le reprend.
garder_le_target() {
  local target="$1" arbre="$2" marque note paquets
  marque="$target/.arbre-du-target"
  mkdir -p "$target"
  arbre="$(cd "$arbre" && pwd -P)"
  if [ ! -f "$marque" ]; then
    echo "$arbre" > "$marque"
    return 0
  fi
  note="$(cat "$marque")"
  [ "$note" = "$arbre" ] && return 0

  if [ "${RAG3WEAVER_TARGET_REPRENDRE:-0}" != "1" ]; then
    echo "✗ le target $target a été bâti par un autre arbre :"
    echo "    noté  : $note"
    echo "    ici   : $arbre"
    echo "  Cargo y reprendrait les artefacts de l'autre arbre (paquets locaux jugés"
    echo "  frais par les dates). Deux remèdes :"
    echo "    - un sous-dossier par worktree :"
    echo "        CARGO_TARGET_DIR=~/.cache/rag3weaver-build/target-<session>/<branche>"
    echo "    - reprendre ce target (efface les paquets locaux, garde les dépendances) :"
    echo "        RAG3WEAVER_TARGET_REPRENDRE=1"
    return 2
  fi

  paquets="$(cd "$arbre" && cargo metadata --format-version 1 2>/dev/null | python3 -c '
import json, sys
m = json.load(sys.stdin)
print(" ".join(sorted({p["name"] for p in m["packages"] if p["source"] is None})))
')" || paquets=""
  if [ -z "$paquets" ]; then
    echo "✗ reprise du target : la liste des paquets locaux est vide (cargo metadata a échoué ?)"
    return 2
  fi
  echo "▸ target repris de $note : cargo clean des paquets locaux ($paquets)"
  local args=() p
  for p in $paquets; do args+=(-p "$p"); done
  (cd "$arbre" && CARGO_TARGET_DIR="$target" cargo clean "${args[@]}") || return 2
  echo "$arbre" > "$marque"
}
