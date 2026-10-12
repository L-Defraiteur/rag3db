#!/bin/bash
# Usage : test/unite/temoin.sh <binaire rag3weaver-backend> <dossier du paquet> <libvector.rag3db_extension du même bâti>
# (Linux, systemd utilisateur ; ne touche jamais l'unité réelle : nom et dossier d'essai.)
# Les cinq témoins de l'unité systemd de l'hôte, sous une unité et un dossier
# d'essai (jamais l'unité réelle) :
#   1. l'unité démarre l'hôte, un client le trouve (un seul démarrage) ;
#   2. l'hôte tué (-9) est relancé par l'unité, le même client continue ;
#   3. systemctl --user stop ferme proprement (daemon.json retiré, code 0) ;
#   4. un hôte manuel déjà là : l'unité échoue en 1, sans boucle ;
#   5. l'unité désinstallée, un client relance l'hôte à la main.
set -u
B=$1; PKG=$2; EXT=$3
E=${TMPDIR:-/tmp}/rag3weaver-unite-essai; rm -rf "$E"; mkdir -p "$E"
DEMON=$E/demon; U=rag3weaver-daemon-essai
export RAG3WEAVER_DAEMON_DIR=$DEMON RAG3WEAVER_DAEMON_ADDRESS=127.0.0.1:0 RAG3WEAVER_BACKEND=$B
cp -r "$PKG/../../templates/proto/boutique" "$E/boutique"
python3 - "$E/boutique/backend.json" "$EXT" <<'PY'
import json, sys
p, ext = sys.argv[1], sys.argv[2]; m = json.load(open(p)); m["vector_extension"] = ext; json.dump(m, open(p, "w"))
PY
M=$E/boutique/backend.json
unite() { node "$PKG/bin/rag3weaver-unit.js" "$@" --name $U; }
demarrages() { grep -c "\[démon\] rag3weaver-daemon" "$DEMON/daemon.log" 2>/dev/null || echo 0; }
fiche_pid() { python3 -c "import json;print(json.load(open('$DEMON/daemon.json'))['pid'])" 2>/dev/null || echo 0; }
prop() { systemctl --user show -p "$1" --value $U.service; }
attendre() { local t=$1; shift; for _ in $(seq 1 $((t*5))); do eval "$@" && return 0; sleep 0.2; done; return 1; }
verdict() { if eval "$2"; then echo "  ✓ $1"; else echo "  ✗ $1 ($2)"; ROUGE=1; fi; }
ROUGE=0; MANUEL=0
nettoyer() {
  exec 7>&- 2>/dev/null
  unite uninstall >/dev/null 2>&1
  for p in $MANUEL $(fiche_pid); do [ "$p" != 0 ] && kill -TERM "$p" 2>/dev/null; done
  sleep 1
}
trap nettoyer EXIT

echo "== 1. l'unité démarre l'hôte, un client le trouve"
unite install --env RAG3WEAVER_DAEMON_DIR=$DEMON --env RAG3WEAVER_DAEMON_ADDRESS=127.0.0.1:0 | sed 's/^/  /'
P1=$(prop MainPID)
verdict "MainPID de l'unité = pid de daemon.json ($P1)" '[ "$P1" != 0 ] && [ "$(fiche_pid)" = "$P1" ]'
mkfifo "$E/cmd"; python3 "$(dirname "$0")/client.py" "$B" "$M" < "$E/cmd" > "$E/client.log" 2>&1 &
exec 7>"$E/cmd"
echo "appel a" >&7
attendre 30 'grep -q "appel a →" "$E/client.log"'
sed 's/^/  /' "$E/client.log"
verdict "l'appel du client répond" 'grep -q "appel a → ok" "$E/client.log"'
verdict "un seul démarrage au journal (le client n'a rien lancé)" '[ "$(demarrages)" = 1 ]'

echo "== 2. l'hôte tué (-9) est relancé par l'unité, le même client continue"
kill -9 "$P1"
attendre 40 '[ "$(prop MainPID)" != 0 ] && [ "$(prop MainPID)" != "$P1" ] && [ "$(fiche_pid)" = "$(prop MainPID)" ]'
P2=$(prop MainPID)
echo "  pid $P1 tué, l'unité a relancé $P2 (NRestarts=$(prop NRestarts))"
verdict "relancé par l'unité, daemon.json réécrit par le nouvel hôte" '[ "$P2" != "$P1" ] && [ "$(fiche_pid)" = "$P2" ]'
echo "appel b" >&7
attendre 30 'grep -q "appel b →" "$E/client.log"'
verdict "le même client rouvre sa session et l'appel répond" 'grep -q "appel b → ok" "$E/client.log"'
verdict "deux démarrages au journal, les deux par l'unité" '[ "$(demarrages)" = 2 ]'
echo "fin" >&7; exec 7>&-; sleep 1

echo "== 3. systemctl --user stop ferme proprement"
systemctl --user stop $U.service
verdict "daemon.json retiré" '[ ! -e "$DEMON/daemon.json" ]'
verdict "ligne d'arrêt au journal" 'tail -5 "$DEMON/daemon.log" | grep -q "arrêté"'
verdict "sortie en 0, unité inactive (pas en échec)" '[ "$(prop ExecMainStatus)" = 0 ] && [ "$(prop ActiveState)" = inactive ]'

echo "== 4. un hôte manuel déjà là : l'unité échoue en 1, sans boucle"
"$B" daemon >> "$DEMON/daemon.log" 2>&1 &
MANUEL=$!
attendre 30 '[ "$(fiche_pid)" = "$MANUEL" ]'
systemctl --user start $U.service 2>/dev/null
sleep 13
echo "  unité : $(prop ActiveState), code $(prop ExecMainStatus), NRestarts=$(prop NRestarts)"
verdict "l'unité est en échec, code 1, aucun redémarrage" '[ "$(prop ActiveState)" = failed ] && [ "$(prop ExecMainStatus)" = 1 ] && [ "$(prop NRestarts)" = 0 ]'
verdict "le refus est dit au journal" 'grep -q "un hôte répond déjà" "$DEMON/daemon.log"'
verdict "daemon.json porte toujours l'hôte manuel" '[ "$(fiche_pid)" = "$MANUEL" ]'
kill -TERM $MANUEL; attendre 30 '! kill -0 $MANUEL 2>/dev/null'; MANUEL=0
systemctl --user reset-failed $U.service

echo "== 5. l'unité désinstallée, un client relance l'hôte à la main"
unite uninstall | sed 's/^/  /'
verdict "fichier d'unité retiré" '[ ! -e "$HOME/.config/systemd/user/$U.service" ]'
avant=$(demarrages)
printf 'appel c\nfin\n' | python3 "$(dirname "$0")/client.py" "$B" "$M" > "$E/client2.log" 2>&1
sed 's/^/  /' "$E/client2.log"
P3=$(fiche_pid)
verdict "le client a lancé un hôte à la main et l'appel répond" 'grep -q "appel c → ok" "$E/client2.log" && [ "$P3" != 0 ] && [ "$(demarrages)" = $((avant+1)) ]'
verdict "cet hôte n'est pas celui d'une unité" '[ "$(ps -o ppid= -p $P3 | tr -d " ")" != "$(systemctl --user show -p MainPID --value)" ] && ! systemctl --user status $U.service >/dev/null 2>&1'
[ "$ROUGE" = 0 ] && echo "== tout vert" || echo "== ROUGE"
exit $ROUGE
