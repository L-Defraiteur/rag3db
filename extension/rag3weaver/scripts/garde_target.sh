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
# **Les empreintes d'un paquet, dans les deux dispositions de cargo** (13 novembre 2026) :
# jusqu'à cargo 1.99, `<profil>/.fingerprint/<paquet>-<hachage>` ; depuis cargo 1.100,
# `<profil>/build/<paquet>/<hachage>/fingerprint`. La reprise ne connaissait que la première :
# sous le cargo de la CI, elle n'effaçait rien, et son contrôle « aucune empreinte ne reste »
# cherchait au même endroit, donc passait à vide (run 43450165154). Une disposition de plus,
# demain, se verra au témoin (scripts/test_garde_target.sh), qui rebâtit après la reprise.
empreintes_du_paquet() {
  local target="$1" p="$2"
  find "$target" \( -path '*/.fingerprint/*' -prune -regextype posix-extended \
      -regex ".*/\.fingerprint/${p}-[0-9a-f]{16}" -print \) -o \
    \( -regextype posix-extended -regex ".*/build/${p}/[0-9a-f]{16}/fingerprint" -print -prune \) 2>/dev/null
}

garder_le_target() {
  local target="$1" arbre="$2" marque note paquets restes p
  shift 2
  local restrictions=("$@") option
  for option in "${restrictions[@]}"; do
    case "$option" in
      --offline|--locked|--frozen) ;;
      *) echo "✗ restriction Cargo inconnue pour la garde : $option" >&2; return 2 ;;
    esac
  done
  mkdir -p "$target" || return 2
  target="$(cd "$target" && pwd -P)" || return 2
  arbre="$(cd "$arbre" && pwd -P)" || return 2
  marque="$target/.arbre-du-target"
  if [ -L "$marque" ] || { [ -e "$marque" ] && [ ! -f "$marque" ]; }; then
    echo "✗ marqueur de target non régulier : $marque" >&2
    return 2
  fi
  if [ ! -f "$marque" ]; then
    local contenu
    contenu="$(find "$target" -mindepth 1 -maxdepth 1 -print -quit)" || return 2
    if [ -n "$contenu" ]; then
      echo "✗ target non vide sans provenance : $target ; conserver et diagnostiquer, ou choisir un target neuf" >&2
      return 2
    fi
    echo "$arbre" > "$marque"
    return 0
  fi
  note="$(cat "$marque")"
  if [ -z "$note" ] || [[ "$note" != /* ]]; then
    echo "✗ provenance de target invalide : $marque" >&2
    return 2
  fi
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
  if ! meta="$(cd "$arbre" && cargo metadata --format-version 1 --all-features "${restrictions[@]}" 2>"$err")"; then
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
    empreintes_du_paquet "$target" "$p" | while IFS= read -r e; do rm -rf -- "$e"; done
  done
  restes=""
  for p in $paquets; do
    restes="$restes$(empreintes_du_paquet "$target" "$p")"
  done
  if [ -n "$restes" ]; then
    echo "✗ reprise du target : des empreintes de paquets locaux restent :"
    echo "$restes" | sed 's/^/    /'
    return 2
  fi
  echo "$arbre" > "$marque"
}
