#!/bin/bash
# Témoin du filet de l'installeur (14 novembre 2026, mot de Lucie) : deux `--filet-seul` de suite ne laissent dans
# le dossier d'unités que `check-lib.conf` ; l'ancien filet va dans le dossier des sauvegardes de pose. Joué dans un
# XDG_CONFIG_HOME et un dossier de sauvegardes jetables : jamais les unités réelles. Usage :
#   scripts/test_installer_filet.sh [chemin de l'installeur] [binaire lié par RUNPATH au magasin, rag3db-check-lib à côté]
set -u
I=$(realpath "${1:-$(dirname "$0")/installer-l-unite-de-l-hote.sh}")
B=${2:-$HOME/.local/bin/rag3weaver-backend}
D=$(mktemp -d); trap 'rm -rf "$D"' EXIT
export XDG_CONFIG_HOME="$D/config" POSTE_SAUVEGARDES="$D/sauvegardes"
[ -x "$B" ] && [ -x "$(dirname "$B")/rag3db-check-lib" ] || { echo "SAUTÉ  pas de binaire de l'hôte avec rag3db-check-lib à côté ($B)"; exit 0; }
for i in 1 2; do bash "$I" --nom essai-filet --binaire "$B" --filet-seul > "$D/sortie-$i" 2>&1 || { echo "ÉCHEC  l'installeur rend une erreur : $(tail -2 "$D/sortie-$i")"; exit 1; }; sleep 1; done
U="$XDG_CONFIG_HOME/systemd/user/essai-filet.service.d"
rouge=0
dans=$(ls "$U" 2>/dev/null | tr '\n' ' ')
if [ "$dans" = "check-lib.conf " ]; then echo "OK     le dossier d'unités ne garde que check-lib.conf"; else echo "ÉCHEC  le dossier d'unités porte : $dans"; rouge=1; fi
n=$(ls "$POSTE_SAUVEGARDES" 2>/dev/null | grep -c '^essai-filet\.check-lib\.conf\.')
if [ "$n" = 1 ]; then echo "OK     l'ancien filet est gardé dans le dossier des sauvegardes de pose (1 copie)"; else echo "ÉCHEC  copies dans les sauvegardes de pose : $n (attendu 1)"; rouge=1; fi
exit $rouge
