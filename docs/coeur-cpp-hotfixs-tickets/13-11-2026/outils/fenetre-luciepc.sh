#!/bin/bash
# fenetre-luciepc.sh <base> "<tables>" "<jusqu'à>" (14 nov. 2026, hotfixs) — la fenêtre de luciepc pour une base de code :
# le banc de recherche par l'hôte avant, la fermeture qui tient, l'instantané, la réparation des deux index de relations,
# la purge des orphelins du plein texte et son compact, la réouverture, le banc après comparé au banc d'avant.
# Un pas rouge arrête tout et laisse la base fermée ; le retour est le RETOUR.sh de l'instantané.
set -u
b=${1:?base}; TABLES=${2:?tables}; JUSQU_A=${3:?jusqu_a}
R=~/.cache/rag3db-locks; BANC=$R/fenetre-purge/banc_recherche.py
M=~/.config/rag3weaver/$b/templates/backends/code/backend.json; BASE=~/.local/share/rag3weaver/$b.rag3db
export ECRIRE=$R/reconstruction-code-lourd/ecrire-373b REQUETE=$R/fenetre-purge/requete-373b
export CONTROLE=~/.local/share/rag3weaver/instantanes/avant-373b5477b-controle/check_rel_directions OUTILS=$R/reparation-deux-sens
export PURGE_BIN=$R/fenetre-purge/diag_purge_orphelins INSTANTANES=~/.cache/rag3weaver-notes-A/instantanes.sh
export ETIQUETTE=avant-fenetre-$b-$(date +%Y%m%d-%H%M)
O=$R/fenetres/$b-$(date +%Y%m%d-%H%M%S); mkdir -p $O
exec > >(tee -a "$O/fenetre.log") 2>&1
REQUETES=(commit "index du plein texte" "point de reprise" mutex "fermer un backend" "chunk_uuid" "les liens par COPY" "test e2e" acquireLock FileSource)
dit() { echo "== $(date +%H:%M:%S) $*"; }
rouge() { echo "ROUGE : $*"; echo "La base reste fermée si elle l'a été. Retour : ~/.local/share/rag3weaver/instantanes/$ETIQUETTE/RETOUR.sh ; réouverture : python3 $BANC ouvrir $M"; exit 1; }
[ -f "$M" ] && [ -f "$BASE" ] || rouge "manifeste ou base introuvable"
dit "$b : banc de recherche avant, par l'hôte"
python3 -I $BANC hote "$M" $O/banc-avant.json "${REQUETES[@]}" || rouge "banc d'avant"
dit "$b : fermeture qui tient"
python3 -I $BANC fermer "$M" "réparation des deux index et purge du plein texte (hotfixs)" "$JUSQU_A" || rouge "fermeture"
dit "$b : instantané $ETIQUETTE"
bash "$INSTANTANES" "$ETIQUETTE" "$BASE" || rouge "instantané"
dit "$b : réparation des deux index"
$OUTILS/reparer-les-deux-sens.sh dedans "$BASE" - "$TABLES" || rouge "réparation"
dit "$b : purge des orphelins du plein texte"
$R/fenetre-purge/fenetre-purge.sh dedans "$BASE" - || rouge "purge"
dit "$b : réouverture"
python3 -I $BANC ouvrir "$M" || rouge "réouverture"
dit "$b : banc après"
python3 -I $BANC hote "$M" $O/banc-apres.json "${REQUETES[@]}" || rouge "banc d'après"
BANC_LIMITE=100 python3 -I $BANC hote "$M" $O/banc-apres-large.json "${REQUETES[@]}" || rouge "banc large d'après"
python3 -I $BANC comparer $O/banc-avant.json $O/banc-apres.json $O/banc-apres-large.json | tee $O/comparaison.txt
tail -1 $O/comparaison.txt | grep -qx VERT || { echo "ROUGE : un résultat d'avant manque dans les 100 premiers d'après (la base est rouverte) : lire $O/comparaison.txt"; exit 1; }
dit "$b : VERT ; sorties $O"
