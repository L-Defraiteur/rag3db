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
# Le target garde dans `.arbre-du-target` la racine canonique de l'arbre qui
# l'a bâti. Un autre arbre est refusé, nommément. `RAG3WEAVER_TARGET_REPRENDRE=1`
# efface les empreintes des paquets locaux (`.fingerprint/<paquet>-<hachage>`,
# dans chaque profil et chaque triplet), ce qui oblige cargo à les rebâtir,
# puis **vérifie** qu'il n'en reste aucune avant de reprendre le target.
# `cargo clean -p` n'est pas employé : chez G il a rendu « Removed 0 files »
# sans dire pourquoi, et la reprise passait pour faite.
# Le témoin : scripts/test_garde_target.sh.
garder_le_target() {
  local target="$1" arbre="$2" marque note paquets restes p
  mkdir -p "$target" || return 2
  target="$(cd "$target" && pwd -P)" || return 2
  arbre="$(cd "$arbre" && pwd -P)" || return 2
  marque="$target/.arbre-du-target"
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
    echo "    - reprendre ce target (efface les empreintes des paquets locaux, garde les"
    echo "      dépendances) : RAG3WEAVER_TARGET_REPRENDRE=1"
    return 2
  fi

  # --all-features : sans lui, une dépendance locale optionnelle (codeparsers,
  # sous la feature code) n'est pas listée, et ses empreintes resteraient.
  local meta err
  err="$(mktemp)"
  if ! meta="$(cd "$arbre" && cargo metadata --format-version 1 --all-features 2>"$err")"; then
    echo "✗ reprise du target : cargo metadata a échoué dans $arbre"
    grep -E '^(error|Caused by|  )' "$err" | head -4 | sed 's/^/    /'
    echo "  (un sous-module non initialisé ? git submodule update --init)"
    rm -f "$err"
    return 2
  fi
  rm -f "$err"
  paquets="$(echo "$meta" | python3 -c '
import json, sys
m = json.load(sys.stdin)
print(" ".join(sorted({p["name"] for p in m["packages"] if p["source"] is None})))
' 2>/dev/null)"
  if [ -z "$paquets" ]; then
    echo "✗ reprise du target : la liste des paquets locaux est vide"
    return 2
  fi
  echo "▸ target repris de $note : empreintes effacées pour $paquets"
  for p in $paquets; do
    find "$target" -path '*/.fingerprint/*' -prune -regextype posix-extended \
      -regex ".*/\.fingerprint/${p}-[0-9a-f]{16}" -exec rm -rf {} + 2>/dev/null
  done
  restes=""
  for p in $paquets; do
    restes="$restes$(find "$target" -path '*/.fingerprint/*' -prune -regextype posix-extended \
      -regex ".*/\.fingerprint/${p}-[0-9a-f]{16}" -print 2>/dev/null)"
  done
  if [ -n "$restes" ]; then
    echo "✗ reprise du target : des empreintes de paquets locaux restent :"
    echo "$restes" | sed 's/^/    /'
    return 2
  fi
  echo "$arbre" > "$marque"
}
