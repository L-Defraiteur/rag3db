#!/bin/bash
# **Instantanés reflink des bases partagées**, avant la bascule de format (41, 42) :
# une base qu'une lib neuve a touchée par un point de reprise est refusée par toute lib
# d'avant ; le seul retour est cet instantané.
#
# Usage : instantanes.sh <étiquette> <base>…   (une base = son chemin `…/nom.rag3db`)
#   ici      : instantanes.sh bascule-42 ~/.local/share/rag3weaver/memoire-longue.rag3db ~/.local/share/rag3weaver/code-rag3db-depot.rag3db
#   luciepc  : instantanes.sh bascule-42 ~/.local/share/rag3weaver/code-principal.rag3db ~/.local/share/rag3weaver/code-lourd.rag3db
#
# Refuse sans rien copier si un processus tient un fichier d'une base (l'hôte doit être
# arrêté proprement, `kill -TERM` ou l'arrêt de l'unité, jamais un arrêt brutal) ; copie
# chaque base et tous ses voisins (`nom.rag3db.*`, `nom.checkpoints`) par
# `cp -a --reflink=always` (btrfs : immédiat, sans place) dans
# ~/.local/share/rag3weaver/instantanes/<étiquette>/ ; vérifie octet pour octet les tailles ;
# écrit la recette de retour à côté.
set -u
[ $# -ge 2 ] || { echo "usage : $0 <étiquette> <base.rag3db>…" >&2; exit 2; }
ETIQUETTE=$1; shift
D=~/.local/share/rag3weaver/instantanes/$ETIQUETTE
[ -e "$D" ] && { echo "REFUS : $D existe déjà ; une autre étiquette" >&2; exit 3; }

# Les fichiers d'une base : elle-même, ses voisins `nom.rag3db.*`, et `nom.checkpoints`.
fichiers_de() {
  local b=$1 dir nom
  dir=$(dirname "$b"); nom=$(basename "$b" .rag3db)
  for f in "$b" "$b".* "$dir/$nom.checkpoints"; do [ -e "$f" ] && echo "$f"; done
}

# 1. Personne ne tient une base : chaque descripteur de chaque processus à moi.
tenus=""
for b in "$@"; do
  [ -e "$b" ] || { echo "REFUS : $b n'existe pas" >&2; exit 4; }
  for p in /proc/[0-9]*; do
    for fd in "$p"/fd/*; do
      cible=$(readlink "$fd" 2>/dev/null) || continue
      case "$cible" in
        "$b"|"$b".*|"$(dirname "$b")/$(basename "$b" .rag3db).checkpoints"*)
          tenus+="  pid ${p#/proc/} ($(tr '\0' ' ' < "$p/cmdline" | cut -c1-100)) tient $cible"$'\n' ;;
      esac
    done
  done
done
if [ -n "$tenus" ]; then
  echo "REFUS : des bases sont ouvertes — arrêter l'hôte proprement d'abord :" >&2
  printf '%s' "$tenus" | sort -u >&2
  exit 5
fi

# 2. La copie, base par base.
mkdir -p "$D"
for b in "$@"; do
  sous="$D/$(basename "$b" .rag3db)"
  mkdir -p "$sous"
  while read -r f; do
    cp -a --reflink=always "$f" "$sous/" || { echo "ÉCHEC : copie de $f (reflink refusé ? pas de repli en copie pleine)" >&2; exit 6; }
  done < <(fichiers_de "$b")
  # 3. La vérification : même taille apparente, octet pour octet, fichier par fichier.
  while read -r f; do
    a=$(du -s --apparent-size -b "$f" | cut -f1); c=$(du -s --apparent-size -b "$sous/$(basename "$f")" | cut -f1)
    [ "$a" = "$c" ] || { echo "ÉCHEC : $f fait $a octets, sa copie $c" >&2; exit 7; }
    echo "  $(basename "$f") : $a octets"
  done < <(fichiers_de "$b")
  echo "instantané de $b → $sous"
done

# 4. La recette de retour, écrite à côté (à jouer hôte arrêté, avec la lib d'avant en courant).
{
  echo "# Retour à l'instantané $ETIQUETTE ($(date '+%d/%m/%Y %H:%M'), $(hostname))"
  echo "# Hôte arrêté proprement, courant ramené à la lib d'avant, puis pour chaque base :"
  for b in "$@"; do
    sous="$D/$(basename "$b" .rag3db)"
    # L'état d'après, mis de côté hors du motif `nom.rag3db.*` (il serait pris pour un voisin).
    apres="$(dirname "$b")/apres-$ETIQUETTE-$(basename "$b" .rag3db)"
    echo "mkdir -p '$apres' && mv $(fichiers_de "$b" | sed "s/.*/'&'/" | tr '\n' ' ')'$apres/'"
    echo "cp -a --reflink=always '$sous'/* '$(dirname "$b")/'"
  done
} > "$D/RETOUR.sh"
echo "fini ; recette de retour : $D/RETOUR.sh"
