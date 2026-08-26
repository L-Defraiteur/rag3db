#!/bin/bash
# Ramener en RAM ce que le noyau a compressé dans le zram.
#
# Pourquoi ça existe (27 août 2026) : avec `vm.swappiness=150` — le défaut
# CachyOS, pensé pour zram — une grosse compilation pousse le **bureau** en
# swap alors que la RAM est libre. La machine « galère » sans que rien ne
# consomme de CPU : chaque retour sur un onglet paie une décompression.
#
# `run_e2e.sh` **prévient** ça en confinant le build dans un cgroup. Ce script
# est la **cure**, pour ce qui est déjà parti — après une passe lancée à la
# main, ou après n'importe quel gros build hors de nos scripts.
#
# Il refuse d'agir si la RAM libre ne suffit pas à tout reprendre.
set -euo pipefail

read -r _ total used _ < <(free -m | grep -i '^Échange\|^Swap')
free_mb=$(free -m | awk '/^Mem|^Mém/ {print $7}')

if [ "${used:-0}" -lt 512 ]; then
  echo "▸ Rien à ramener : ${used} Mo en swap."
  exit 0
fi
if [ "$free_mb" -lt "$used" ]; then
  echo "✗ ${used} Mo en swap mais seulement ${free_mb} Mo disponibles — je n'y touche pas."
  echo "  Ferme quelque chose, ou accepte le swap : le forcer ici tuerait des processus."
  exit 1
fi

echo "▸ ${used} Mo à ramener, ${free_mb} Mo disponibles. Ça prend une minute ou deux."
for dev in $(swapon --noheadings --show=NAME); do
  echo "  $dev…"
  sudo swapoff "$dev"
  sudo swapon "$dev"
done
free -h | grep -i '^Mem\|^Mém\|^Échange\|^Swap'
