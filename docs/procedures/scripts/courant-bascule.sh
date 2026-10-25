#!/bin/bash
# **courant aux deux postes ensemble**, après le verdict e2e_code et au mot de c7. Refuse sans rien poser si la passe de la session recherche
# (unité systemd --user ide-fusion à luciepc, qui lit courant à chaque appel cargo) tourne
# encore, ou si la lib n'a pas la même somme aux deux postes.
set -u
# Usage : courant-bascule.sh <id de bâti> <md5 de librag3db.so>
ID=${1:?id de bâti}
MD5=${2:?md5 attendue}
S=extension/rag3weaver/scripts/rag3db_libs.py
etat=$(timeout 20 ssh -T -o BatchMode=yes lucied@luciepc 'systemctl --user is-active ide-fusion' 2>/dev/null)
case "$etat" in
  active|activating|reloading) echo "REFUS : ide-fusion est « $etat » à luciepc — attendre sa fin, ou prévenir recherche" >&2; exit 3 ;;
esac
echo "ide-fusion à luciepc : ${etat:-injoignable}"
[ -n "$etat" ] || { echo "REFUS : état d'ide-fusion illisible (ssh ?)" >&2; exit 4; }
ici=$(md5sum ~/.cache/rag3db-libs/$ID/librag3db.so | cut -d' ' -f1)
la_bas=$(timeout 20 ssh -T -o BatchMode=yes lucied@luciepc "md5sum ~/.cache/rag3db-libs/$ID/librag3db.so" | cut -d' ' -f1)
[ "$ici" = $MD5 ] && [ "$la_bas" = $MD5 ] || { echo "REFUS : md5 ici $ici, luciepc $la_bas, attendu $MD5" >&2; exit 5; }
python3 ~/git_workspaces/rag3db/$S courant --poser $ID || exit 6
timeout 30 ssh -T -o BatchMode=yes lucied@luciepc "python3 ~/git_workspaces/rag3db/$S courant --poser $ID" || { echo "ÉCHEC à luciepc : courant ici posé, pas là-bas" >&2; exit 7; }
echo "ici : $(readlink ~/.cache/rag3db-libs/courant)"
echo "luciepc : $(timeout 20 ssh -T -o BatchMode=yes lucied@luciepc 'readlink ~/.cache/rag3db-libs/courant')"
