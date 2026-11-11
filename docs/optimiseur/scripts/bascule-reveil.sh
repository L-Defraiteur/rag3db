#!/bin/bash
# Un poste : RÉVEILLE l'hôte posé par bascule-pose.sh, après l'instantané des bases (mot de l'arbre principal).
# Vérifie d'abord le service d'embarquement (un backend ouvert sans lui reste en plein texte sans le lot
# embarqueur-revenu), puis la socket, /sante, MainPID, la lib dans les maps, LISTEN_FDS, l'env, le jeton.
# Usage : bascule-reveil.sh <id de lib> <adresses d'embarquement séparées par des espaces, ex. 127.0.0.1:7979 …>
set -u
NOUVELLE=${1:?id de lib}; shift
dire() { echo "== $(date +%H:%M:%S) $(hostname -s) $*"; }
rouge=0
# 9 nov. 2026 (c7) : un service lent n'est pas un service absent. Absent (rien en 30 s) refuse ; lent
# (plus de 5 s) prévient et réveille quand même : l'hôte de Lucie coupé pèse plus qu'un embarquement lent.
for a in "$@"; do
  s=$(date +%s%N); d=$(curl -s -m 30 "http://$a/sante" | grep -oE '"dim":[0-9]+'); ms=$(( ($(date +%s%N)-s)/1000000 ))
  dire "service $a : ${d:-INJOIGNABLE} en $ms ms"
  [ -n "$d" ] || rouge=1
  [ -n "$d" ] && [ $ms -gt 5000 ] && dire "AVERTISSEMENT : $a lent ($ms ms) — réveil quand même, le dire à c7"
done
[ $rouge = 0 ] || { dire "un service d'embarquement ne répond pas en 30 s : je ne réveille pas (le dire, et attendre)"; echo ROUGE; exit 4; }
systemctl --user start rag3weaver-daemon.socket || { echo ROUGE; exit 1; }
S=$(mktemp)
for i in $(seq 1 60); do curl -fsS -m 3 http://127.0.0.1:7970/sante > $S 2>/dev/null && break; sleep 1; done
[ -s $S ] || { dire "/sante ne répond pas"; echo ROUGE; exit 1; }
P=$(systemctl --user show rag3weaver-daemon.service -p MainPID --value)
dire "/sante : $(cut -c1-220 $S)"
dire "MainPID $P ; exe $(readlink /proc/$P/exe) ; lib : $(grep -o 'rag3db-build-[0-9a-f]\{9\}' /proc/$P/maps | sort -u | tr '\n' ' ')"
grep -q "${NOUVELLE:13:9}" /proc/$P/maps || { dire "la lib chargée n'est pas ${NOUVELLE:13:9}"; rouge=1; }
grep -q "\"pid\":$P" $S || { dire "le pid de /sante n'est pas le MainPID"; rouge=1; }
dire "LISTEN_FDS : $(tr '\0' '\n' < /proc/$P/environ | grep -E '^LISTEN_FDS=') ; RAG3DB_LIBRARY_DIR : $(tr '\0' '\n' < /proc/$P/environ | grep -oE 'RAG3DB_LIBRARY_DIR=.*' | grep -o 'rag3db-build-[0-9a-f]\{9\}')"
dire "jeton md5 : $(md5sum ~/.local/share/rag3weaver/daemon.token | cut -c1-8)"
rm -f $S
[ $rouge = 0 ] && echo VERT || echo ROUGE
