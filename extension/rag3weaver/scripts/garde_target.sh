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

# **Effacer les empreintes des paquets locaux** d'un target : tous (`<sous>` vide), ou ceux dont le manifeste est sous
# le dossier `<sous>` (un sous-module). Usage : effacer_empreintes_locales <target> <arbre> <raison> <sous> [restrictions…]
effacer_empreintes_locales() {
  local target="$1" arbre="$2" raison="$3" sous="$4"
  shift 4
  local restrictions=("$@")
  # --all-features : sans lui, une dépendance locale optionnelle (codeparsers,
  # sous la feature code) n'est pas listée, et ses empreintes resteraient.
  local meta err paquets restes p
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
racine = sys.argv[1]
print(" ".join(sorted({p["name"] for p in m["packages"] if p["source"] is None
                        and (not racine or p["manifest_path"].startswith(racine.rstrip("/") + "/"))})))
' "$sous" 2>/dev/null)"
  if [ -z "$paquets" ]; then
    echo "✗ reprise du target : la liste des paquets locaux est vide"
    return 2
  fi
  echo "▸ target $raison : empreintes effacées pour $paquets"
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
}

# **Les commits des sous-modules** (14 novembre 2026, faux rouge de codeparsers) : un sous-module ramené à un commit plus
# vieux a des fichiers datés d'avant les artefacts, que cargo juge frais. Le target note le commit de chaque sous-module
# de l'arbre dans `.sous-modules-du-target` ; un commit changé, ou une liste encore jamais notée, efface les empreintes
# des paquets de ce sous-module.
sous_modules_de() { # <arbre> : « <chemin absolu> <commit> » par sous-module initialisé
  local haut
  haut="$(git -C "$1" rev-parse --show-toplevel 2>/dev/null)" || return 0
  git -C "$haut" submodule status --recursive 2>/dev/null | while read -r sha chemin _; do
    case "$sha" in -*) continue ;; esac
    sha="${sha#[+U ]}"
    echo "$(cd "$haut/$chemin" && pwd -P) $sha"
  done
}
noter_les_sous_modules() { # <target> <arbre>
  sous_modules_de "$2" > "$1/.sous-modules-du-target"
}
suivre_les_sous_modules() { # <target> <arbre> [restrictions…] : efface ce qui a changé, puis note
  local target="$1" arbre="$2" fichier="$1/.sous-modules-du-target" ligne chemin sha
  shift 2
  local actuels
  actuels="$(sous_modules_de "$arbre")"
  [ -n "$actuels" ] || { : > "$fichier"; return 0; }
  while read -r chemin sha; do
    [ -n "$chemin" ] || continue
    if [ ! -f "$fichier" ]; then
      effacer_empreintes_locales "$target" "$arbre" "sans commit noté pour le sous-module $chemin" "$chemin" "$@" || return 2
    elif ! grep -qxF "$chemin $sha" "$fichier"; then
      effacer_empreintes_locales "$target" "$arbre" "à un autre commit du sous-module $chemin ($(grep -F "$chemin " "$fichier" | cut -d' ' -f2 | cut -c1-9) → ${sha:0:9})" "$chemin" "$@" || return 2
    fi
  done <<< "$actuels"
  echo "$actuels" > "$fichier"
}

# **Le ménage au plafond** (15 novembre 2026, ticket d'archi) : au-delà du plafond, run_e2e.sh effaçait tout le target, et
# rebâtissait les dépendances tierces avec. Au plafond, ce sont les paquets locaux qui pèsent (archi, delais-c, 36 Go :
# 24 Go d'incrémental de rag3weaver, 7,3 Go d'artefacts locaux, 4,9 Go de tiers ; `cargo clean -p rag3weaver` en rend 32,9).
# `menager_au_plafond <target> <arbre> <plafond en Mio>` fait `cargo clean -p` des paquets locaux pour chaque profil
# (debug, release) et chaque triplet présents, le dit avec le poids avant et après, et rend 1 si le target reste au-dessus.
menager_au_plafond() {
  local target="$1" arbre="$2" plafond="$3" avant apres paquets d nom prof args
  [ -d "$target" ] || return 0
  avant=$(du -xsm "$target" 2>/dev/null | cut -f1)
  [ "${avant:-0}" -gt "$plafond" ] || return 0
  paquets="$(cd "$arbre" && cargo metadata --format-version 1 --all-features 2>/dev/null | python3 -c '
import json, sys
m = json.load(sys.stdin)
print(" ".join(sorted({p["name"] for p in m["packages"] if p["source"] is None})))
' 2>/dev/null)"
  [ -n "$paquets" ] || { echo "✗ ménage au plafond : la liste des paquets locaux est vide (cargo metadata dans $arbre)"; return 1; }
  echo "▸ le target $target pèse ${avant} Mio, au-delà du plafond de ${plafond} Mio : ménage des paquets locaux ($paquets)"
  args=""; for p in $paquets; do args="$args -p $p"; done
  # **L'étiquette du cache** (15 novembre 2026, relevé d'archi) : `cargo clean` refuse un target sans `CACHEDIR.TAG`
  # (« cleaning has been aborted to prevent accidental deletion of unrelated files »). Nos targets naissent de la garde
  # (`mkdir -p`, puis le marqueur) avant que cargo ne les voie, et cargo n'y écrit alors jamais son étiquette. Le target
  # est à nous (il porte notre marqueur) : la garde la pose, au contenu de cargo.
  if [ ! -f "$target/CACHEDIR.TAG" ]; then
    printf 'Signature: 8a477f597d28d172789f06886806bc55\n# This file is a cache directory tag created by cargo.\n# For information about cache directory tags see https://bford.info/cachedir/\n' > "$target/CACHEDIR.TAG"
  fi
  for d in "$target" "$target"/*/; do
    d="${d%/}"; nom=$(basename "$d")
    if [ "$d" != "$target" ]; then
      case "$nom" in debug|release|doc|tmp|package|build|incremental|deps|.*) continue ;; esac
      [ -d "$d/debug" ] || [ -d "$d/release" ] || continue
    fi
    for prof in debug release; do
      [ -d "$d/$prof" ] || continue
      if [ "$d" = "$target" ]; then
        (cd "$arbre" && cargo clean -q --target-dir "$target" $([ $prof = release ] && echo --release) $args) \
          || { echo "✗ ménage au plafond : cargo clean a échoué dans $d ($prof), voir ci-dessus"; return 1; }
      else
        (cd "$arbre" && cargo clean -q --target-dir "$target" --target "$nom" $([ $prof = release ] && echo --release) $args) \
          || { echo "✗ ménage au plafond : cargo clean a échoué dans $d ($prof), voir ci-dessus"; return 1; }
      fi
    done
  done
  apres=$(du -xsm "$target" 2>/dev/null | cut -f1)
  echo "▸ ménage au plafond : ${avant} Mio → ${apres} Mio (les dépendances tierces gardées)"
  [ "${apres:-0}" -gt "$plafond" ] && return 1
  return 0
}

# **Un build.rs qui vise un autre arbre** (15 novembre 2026, ticket d'archi) : `build.rs` embarque des fichiers par
# chemins absolus (les gabarits, par `include_bytes!`) et dit `rerun-if-changed` sur ces chemins. Cargo ne met pas le
# chemin d'un paquet local dans son hachage : deux worktrees partagent le même `OUT_DIR`. Un bâti hors garde depuis un
# autre worktree rejoue build.rs pour lui, et le marqueur garde l'arbre d'avant ; au retour, cargo juge build.rs à jour,
# puisque les chemins qu'il surveille sont ceux de l'autre arbre, qui n'ont pas bougé. `build_rs_d_un_autre_arbre
# <target> <arbre>` lit les sorties des build scripts du target (`<profil>/build/…/output`, à l'hôte et sous chaque
# triplet). Pour chaque sortie dont un chemin `rerun-if-changed` est sous un autre worktree du dépôt, ou sous le dossier
# git d'un autre worktree, elle écrit « <dossier de la sortie><tab><chemin> » ; elle rend 0 si elle en trouve une, 1
# sinon (et 1 hors d'un dépôt git). La garde retire ces dossiers : une sortie périmée d'un jeu de features que personne
# ne rebâtit ferait sinon effacer les empreintes à chaque passe.
build_rs_d_un_autre_arbre() {
  local target="$1" arbre="$2" haut propre commun
  haut="$(git -C "$arbre" rev-parse --show-toplevel 2>/dev/null)" || return 1
  propre="$(git -C "$arbre" rev-parse --absolute-git-dir 2>/dev/null)" || return 1
  commun="$(cd "$arbre" && cd "$(git rev-parse --git-common-dir)" && pwd -P)" || return 1
  python3 - "$target" "$haut" "$propre" "$commun" "$(git -C "$arbre" worktree list --porcelain | sed -n 's/^worktree //p')" <<'PY'
import os, re, sys
target, haut, propre, commun, liste = sys.argv[1:6]
haut, propre = os.path.realpath(haut), os.path.realpath(propre)
autres = [os.path.realpath(w) for w in liste.splitlines() if w and os.path.realpath(w) != haut]
sous = lambda p, r: p == r or p.startswith(r.rstrip('/') + '/')
racines = []
for base in [target] + [os.path.join(target, d) for d in os.listdir(target) if os.path.isdir(os.path.join(target, d))]:
    for prof in ('debug', 'release'):
        b = os.path.join(base, prof, 'build')
        if os.path.isdir(b):
            racines.append(b)
trouve = False
for b in racines:
    for dossier, _, fichiers in os.walk(b):
        if 'output' not in fichiers:
            continue
        try:
            lignes = open(os.path.join(dossier, 'output'), errors='replace').read().splitlines()
        except OSError:
            continue
        for l in lignes:
            m = re.match(r'cargo::?rerun-if-changed=(/.*)', l)
            if not m:
                continue
            p = os.path.normpath(m.group(1))
            # Le dossier git d'abord : il est sous l'arbre principal, qui est aussi « un autre worktree ».
            if sous(p, propre):
                continue
            if sous(p, commun):
                if sous(p, os.path.join(commun, 'worktrees')):
                    print(dossier + '\t' + p); trouve = True; break
                continue
            if any(sous(p, a) for a in autres):
                print(dossier + '\t' + p); trouve = True; break
sys.exit(0 if trouve else 1)
PY
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
    noter_les_sous_modules "$target" "$arbre"
    return 0
  fi
  note="$(cat "$marque")"
  if [ -z "$note" ] || [[ "$note" != /* ]]; then
    echo "✗ provenance de target invalide : $marque" >&2
    return 2
  fi
  if [ "$note" = "$arbre" ]; then
    local perimes d
    if perimes="$(build_rs_d_un_autre_arbre "$target" "$arbre")"; then
      effacer_empreintes_locales "$target" "$arbre" "noté pour cet arbre, mais un build.rs vise un autre arbre ($(echo "$perimes" | head -1 | cut -f2) : un bâti hors garde ; $(echo "$perimes" | wc -l) sortie(s) retirée(s))" "" "${restrictions[@]}" || return 2
      echo "$perimes" | cut -f1 | while IFS= read -r d; do
        case "$d" in "$target"/*) rm -rf -- "$d" ;; esac
      done
    fi
    suivre_les_sous_modules "$target" "$arbre" "${restrictions[@]}"
    return
  fi

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

  effacer_empreintes_locales "$target" "$arbre" "repris de $note" "" "${restrictions[@]}" || return 2
  echo "$arbre" > "$marque"
  noter_les_sous_modules "$target" "$arbre"
}


# **`garde_target.sh --reprendre <target> <arbre>`** (14 novembre 2026, suggestion de codeparsers) : avant un `cargo`
# lancé à la main, hors des scripts de passe ; efface toujours les empreintes des paquets locaux, même pour l'arbre noté,
# puis note l'arbre et les commits de ses sous-modules.
if [ "${BASH_SOURCE[0]}" = "$0" ]; then
  if [ "${1:-}" = "--reprendre" ] && [ $# = 3 ]; then
    mkdir -p "$2" && t="$(cd "$2" && pwd -P)" && a="$(cd "$3" && pwd -P)" || exit 2
    effacer_empreintes_locales "$t" "$a" "repris à la main" "" || exit 2
    echo "$a" > "$t/.arbre-du-target"
    noter_les_sous_modules "$t" "$a"
    exit 0
  fi
  echo "usage : garde_target.sh --reprendre <target> <arbre>   (ou : . garde_target.sh ; garder_le_target <target> <arbre>)" >&2
  exit 2
fi
