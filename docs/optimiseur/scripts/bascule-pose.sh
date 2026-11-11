#!/bin/bash
# Un poste (le portable de Lucie ou luciepc) : POSE le nouveau binaire de l'hôte, sans le réveiller.
# La socket reste ARRÊTÉE : aucune base ne se rouvre avant l'instantané de l'arbre principal ; bascule-reveil.sh suit.
# Usage : bascule-pose.sh <id de lib> <étiquette de l'ancien, ex. avant-44c790c2>
# Attend ~/.local/bin/rag3weaver-backend.nouveau et rag3db-check-lib.nouveau. S'arrête au premier écart et remet l'ancien.
set -u
NOUVELLE=${1:?id de lib}; AVANT=${2:?étiquette de l\'ancien}
B=$HOME/.local/bin
ENV=$HOME/.config/rag3weaver/rag3weaver-daemon.env
FILET=$HOME/.config/systemd/user/rag3weaver-daemon.service.d/check-lib.conf
INSTALLEUR=${INSTALLEUR:-/home/lucied/git_workspaces/rag3db-docs-G/extension/rag3weaver/scripts/installer-l-unite-de-l-hote.sh}
dire() { echo "== $(date +%H:%M:%S) $(hostname -s) $*"; }
remettre() {
  dire "ÉCART : $* — je remets l'ancien (socket laissée arrêtée)"
  systemctl --user stop rag3weaver-daemon.socket rag3weaver-daemon.service 2>/dev/null
  [ -f $B/rag3weaver-backend.$AVANT ] && cp -p $B/rag3weaver-backend.$AVANT $B/rag3weaver-backend
  [ -f $B/rag3db-check-lib.$AVANT ] && cp -p $B/rag3db-check-lib.$AVANT $B/rag3db-check-lib
  [ -f $ENV.$AVANT ] && cp -p $ENV.$AVANT $ENV
  [ -f $FILET.$AVANT ] && cp -p $FILET.$AVANT $FILET
  systemctl --user daemon-reload
  echo ROUGE; exit 1
}
[ -d $HOME/.cache/rag3db-libs/$NOUVELLE ] || { echo "la lib $NOUVELLE n'est pas au magasin : rien posé"; echo ROUGE; exit 3; }
[ -x $B/rag3weaver-backend.nouveau ] && [ -x $B/rag3db-check-lib.nouveau ] || { echo "pas de .nouveau : rien posé"; echo ROUGE; exit 3; }
$B/rag3db-check-lib.nouveau $B/rag3weaver-backend.nouveau $HOME/.cache/rag3db-libs/$NOUVELLE/librag3db.so || { echo "le .nouveau ne va pas avec $NOUVELLE : rien posé"; echo ROUGE; exit 3; }
# L'hôte s'arrête (proprement, surveillé cinq minutes, rien de tué) et la socket aussi.
etat=$(systemctl --user show rag3weaver-daemon.service -p ActiveState --value)
dire "état de l'hôte : $etat"
systemctl --user stop rag3weaver-daemon.socket
if [ "$etat" != inactive ]; then
  systemctl --user stop rag3weaver-daemon.service &
  for i in $(seq 1 300); do [ "$(systemctl --user show rag3weaver-daemon.service -p ActiveState --value)" = inactive ] && break; sleep 1; done
  [ "$(systemctl --user show rag3weaver-daemon.service -p ActiveState --value)" = inactive ] || { dire "l'hôte ne s'arrête pas en 5 min : je ne tue rien"; echo ROUGE; exit 1; }
fi
dire "hôte arrêté, socket arrêtée"
cp -p $B/rag3weaver-backend $B/rag3weaver-backend.$AVANT && cp -p $B/rag3db-check-lib $B/rag3db-check-lib.$AVANT && cp -p $ENV $ENV.$AVANT || remettre "copie de l'ancien"
[ -f $FILET ] && cp -p $FILET $FILET.$AVANT
mv $B/rag3weaver-backend.nouveau $B/rag3weaver-backend && mv $B/rag3db-check-lib.nouveau $B/rag3db-check-lib || remettre "pose du nouveau"
dire "binaire $(md5sum $B/rag3weaver-backend | cut -c1-8), check-lib $(md5sum $B/rag3db-check-lib | cut -c1-8) ; ancien en .$AVANT"
sed -i "s|^RAG3DB_LIBRARY_DIR=.*|RAG3DB_LIBRARY_DIR=$HOME/.cache/rag3db-libs/$NOUVELLE|" $ENV
grep -q "$NOUVELLE" $ENV || remettre "env"
bash $INSTALLEUR --filet-seul || remettre "filet"
grep -q "${NOUVELLE:13:9}" $FILET || remettre "le filet ne nomme pas la nouvelle lib"
systemctl --user daemon-reload
dire "posé : env et filet sur ${NOUVELLE:13:9} ; la socket reste ARRÊTÉE jusqu'à bascule-reveil.sh"
echo VERT
