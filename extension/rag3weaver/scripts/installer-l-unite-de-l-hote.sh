#!/bin/bash
# **L'hôte de rag3weaver en service systemd --user, activé par socket** (18 octobre 2026).
#
# `<nom>.socket` écoute l'adresse de l'hôte ; la première connexion lance `<nom>.service`,
# qui démarre LE BINAIRE INSTALLÉ (jamais celui d'un client). L'hôte prend la socket de
# systemd (LISTEN_FDS), s'arrête seul après l'inactivité (RAG3WEAVER_DAEMON_IDLE_MIN), et
# l'appel suivant le relance. Les clients voient l'unité et ne lancent plus d'hôte.
#
#   scripts/installer-l-unite-de-l-hote.sh [--binaire <chemin>] [--adresse 127.0.0.1:7970]
#       [--inactivite <minutes>] [--arenes <n>] [--env <fichier>] [--nom rag3weaver-daemon]
#       [--maintenant] [--montrer] [--retirer] [--filet-seul]
#
# - `--env` : le fichier d'environnement de l'hôte (service d'embarquement, RAG3DB_ROOT,
#   RAG3DB_LIBRARY_DIR…), repris tel quel s'il existe (~/.config/rag3weaver/<nom>.env).
# - Un hôte de cette unité déjà en service n'est arrêté qu'avec `--maintenant` : c'est un
#   redémarrage, à annoncer avant aux sessions que `daemon_status` liste.
# - `--arenes` : MALLOC_ARENA_MAX de l'hôte (glibc), 0 pour le défaut de glibc (8 par cœur).
#   Les arènes des fils ne sont jamais rendues au système : un plafond borne ce que l'hôte
#   garde après un gros index (ticket « l'hôte ne rend pas la mémoire d'un gros index »).
# - `--montrer` écrit les deux unités sur la sortie, sans rien poser.
# - `--nom` : une autre unité (les témoins : rag3weaver-daemon-essai).
# - Les unités remplacées sont gardées à côté (`.avant-socket`).
# - **Le filet de la lib** (`<nom>.service.d/check-lib.conf`) : si le binaire est lié par
#   RUNPATH à une lib du magasin, l'unité refuse de démarrer un binaire et une lib de bâtis
#   différents (`rag3db-check-lib`, à côté du binaire), ou une extension vector d'un autre
#   bâti. Reposé à chaque installation ; `--filet-seul` ne pose que lui (sans redémarrer
#   l'hôte : il vaut au prochain démarrage). Un binaire au moteur statique n'en a pas besoin.
set -euo pipefail

binaire="$HOME/.local/bin/rag3weaver-backend"
adresse="127.0.0.1:7970"
inactivite=30
arenes=4
nom="rag3weaver-daemon"
env_fichier=""
maintenant=0
montrer=0
retirer=0
filet_seul=0
while [ $# -gt 0 ]; do
  case "$1" in
    --binaire) binaire="$2"; shift 2 ;;
    --adresse) adresse="$2"; shift 2 ;;
    --inactivite) inactivite="$2"; shift 2 ;;
    --arenes) arenes="$2"; shift 2 ;;
    --env) env_fichier="$2"; shift 2 ;;
    --nom) nom="$2"; shift 2 ;;
    --maintenant) maintenant=1; shift ;;
    --montrer) montrer=1; shift ;;
    --retirer) retirer=1; shift ;;
    --filet-seul) filet_seul=1; shift ;;
    *) echo "option inconnue : $1 (voir l'en-tête du script)" >&2; exit 2 ;;
  esac
done
[ -n "$env_fichier" ] || env_fichier="$HOME/.config/rag3weaver/$nom.env"
unites="${XDG_CONFIG_HOME:-$HOME/.config}/systemd/user"
journal="${XDG_STATE_HOME:-$HOME/.local/state}/rag3weaver/daemon.log"
[ "$nom" = rag3weaver-daemon ] || journal="${XDG_STATE_HOME:-$HOME/.local/state}/rag3weaver/$nom.log"

texte_socket() {
  cat <<EOF
# Posé par extension/rag3weaver/scripts/installer-l-unite-de-l-hote.sh
[Unit]
Description=rag3weaver : la socket de l'hôte ($adresse), qui le lance à la première connexion

[Socket]
ListenStream=$adresse
Accept=no

[Install]
WantedBy=sockets.target
EOF
}

texte_service() {
  cat <<EOF
# Posé par extension/rag3weaver/scripts/installer-l-unite-de-l-hote.sh
[Unit]
Description=rag3weaver : l'hôte des backends (lancé par $nom.socket)
Requires=$nom.socket
After=$nom.socket

[Service]
Type=simple
EnvironmentFile=-$env_fichier
Environment=RAG3WEAVER_DAEMON_IDLE_MIN=$inactivite
$( [ "$arenes" = 0 ] || echo "Environment=MALLOC_ARENA_MAX=$arenes" )
ExecStart=$binaire daemon
KillSignal=SIGTERM
KillMode=mixed
# L'arrêt borne l'attente des réactions à 30 s (RAG3WEAVER_STOP_WAIT_S) ; au-delà de
# 90 s, systemd tue : le journal du moteur est rejoué à l'ouverture suivante.
TimeoutStopSec=90
Restart=on-failure
RestartPreventExitStatus=1
RestartSec=5
StandardOutput=append:$journal
StandardError=append:$journal
EOF
}

# Le filet de la lib, d'après le RUNPATH du binaire.
texte_filet() {
  local lib id
  lib=$(readelf -d "$binaire" 2>/dev/null | sed -n 's/.*RUNPATH.*\[\(.*\)\]/\1/p' | cut -d: -f1)
  [ -n "$lib" ] && [ -f "$lib/librag3db.so" ] || return 1
  id=$(readelf -p .rag3db_build_id "$lib/librag3db.so" 2>/dev/null | grep -o 'rag3db-build-[A-Za-z0-9-]*' | head -1)
  [ -n "$id" ] || return 1
  cat <<EOF
# Posé par extension/rag3weaver/scripts/installer-l-unite-de-l-hote.sh
[Service]
# Le binaire et la lib de son RUNPATH sont du même bâti, sinon l'unité ne démarre pas.
ExecStartPre=$(dirname "$binaire")/rag3db-check-lib $binaire $lib/librag3db.so
# Et l'extension vector à côté de la lib, du même bâti.
ExecStartPre=/usr/bin/grep -qaF $id $lib/libvector.rag3db_extension
EOF
}

poser_le_filet() {
  local d="$unites/$nom.service.d"
  if ! texte_filet > /dev/null; then
    echo "filet de la lib : $binaire n'a pas de RUNPATH vers une lib du magasin (moteur statique ?) — pas de check-lib.conf"
    return 0
  fi
  [ -x "$(dirname "$binaire")/rag3db-check-lib" ] || { echo "pas de rag3db-check-lib à côté de $binaire : le filet ne peut pas être posé" >&2; return 3; }
  mkdir -p "$d"
  [ -f "$d/check-lib.conf" ] && cp -p "$d/check-lib.conf" "$d/check-lib.conf.avant-$(date +%Y%m%d-%H%M%S)"
  texte_filet > "$d/check-lib.conf"
  echo "filet de la lib : $d/check-lib.conf"
}

if [ "$montrer" = 1 ]; then
  echo "=== $unites/$nom.socket"; texte_socket
  echo "=== $unites/$nom.service"; texte_service
  echo "=== $unites/$nom.service.d/check-lib.conf"; texte_filet || echo "(pas de RUNPATH vers une lib du magasin : pas de filet)"
  exit 0
fi

if [ "$filet_seul" = 1 ]; then
  poser_le_filet || exit $?
  systemctl --user daemon-reload
  echo "$nom : le filet vaut au prochain démarrage de l'hôte (pas de redémarrage ici)"
  exit 0
fi

if [ "$retirer" = 1 ]; then
  systemctl --user disable --now "$nom.socket" 2>/dev/null || true
  [ "$maintenant" = 1 ] && systemctl --user stop "$nom.service" 2>/dev/null || true
  rm -f "$unites/$nom.socket" "$unites/$nom.service"
  systemctl --user daemon-reload
  echo "$nom : unités retirées ($( [ "$maintenant" = 1 ] && echo "hôte arrêté" || echo "un hôte en service reste jusqu'à son arrêt"))"
  exit 0
fi

[ -x "$binaire" ] || { echo "pas de binaire exécutable à $binaire (--binaire)" >&2; exit 2; }
if systemctl --user is-active --quiet "$nom.service" && [ "$maintenant" != 1 ]; then
  echo "un hôte de $nom est en service : l'arrêter est un redémarrage — l'annoncer aux sessions" >&2
  echo "que daemon_status liste, puis relancer avec --maintenant" >&2
  exit 3
fi

mkdir -p "$unites" "$(dirname "$journal")" "$(dirname "$env_fichier")"
for f in "$nom.socket" "$nom.service"; do
  [ -f "$unites/$f" ] && cp -p "$unites/$f" "$unites/$f.avant-socket"
done
[ -f "$env_fichier" ] && echo "environnement repris : $env_fichier" || echo "pas d'environnement à $env_fichier (l'hôte part sans service d'embarquement déclaré)"
texte_socket > "$unites/$nom.socket"
texte_service > "$unites/$nom.service"
poser_le_filet || exit $?
systemctl --user daemon-reload
# L'ancienne unité sans socket démarrait avec la session (WantedBy=default.target) :
# c'est la socket qui démarre désormais.
systemctl --user disable "$nom.service" 2>/dev/null || true
if systemctl --user is-active --quiet "$nom.service"; then
  echo "arrêt de l'hôte en service ($(date '+%H:%M:%S'))"
  systemctl --user stop "$nom.service"
fi
systemctl --user enable --now "$nom.socket"
echo "$nom : la socket écoute $adresse ; l'hôte ($binaire) démarre au premier appel, s'arrête après $inactivite min sans appel ni travail"
systemctl --user --no-pager status "$nom.socket" | head -5 || true
