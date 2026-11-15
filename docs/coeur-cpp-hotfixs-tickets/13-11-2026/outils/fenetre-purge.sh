#!/bin/bash
# fenetre-purge.sh (14 nov. 2026, hotfixs) — la purge des documents orphelins du plein texte d'une base de code.
#
#   répétition, sur une copie (aucun hôte, la recherche par `mcp --standalone`) :
#     fenetre-purge.sh repetition <copie/nom.rag3db> <backend.json de la copie>
#   compte à froid seul, sur une copie (les pas 3, rien n'est effacé) :
#     fenetre-purge.sh compter <copie/nom.rag3db> -
#   dedans, sur une base déjà fermée et instantanée par le pilote (ni banc, ni fermeture, ni instantané, ni réouverture) :
#     fenetre-purge.sh dedans <nom.rag3db> -
#   fenêtre, sur une base de l'hôte (la fermeture qui tient d'archi requise) :
#     fenetre-purge.sh fenetre <~/.local/share/rag3weaver/nom.rag3db> <backend.json> "<jusqu'à>"
#
# Les pas : 1. le banc de recherche avant ; 2. (fenêtre) la fermeture qui tient, puis l'instantané par
# instantanes.sh ; 3. les orphelins recalculés à froid, table par table (offsets du graphe en lecture seule
# contre les _node_id vivants du .fts) ; 4. pour chaque table qui en porte : la purge (rapport d'index et
# banc lucivy avant et après), puis le compact ; 5. (fenêtre) la réouverture ; 6. le banc de recherche
# après, comparé au banc d'avant. Un pas rouge arrête tout ; en fenêtre, la base reste fermée et le
# retour est la recette RETOUR.sh de l'instantané.
#
# Outils (variables) : PURGE_BIN = le binaire du test lucivy diag_purge_orphelins (branche
# purge-orphelins-depot) ; REQUETE = l'outil de requête en lecture seule (requete.cpp, lib courante) ;
# INSTANTANES = instantanes.sh de G ; ETIQUETTE = l'étiquette de l'instantané.
set -u
MODE=${1:?repetition|fenetre}; BASE=${2:?base.rag3db}; MANIFESTE=${3:?backend.json}; JUSQU_A=${4:-}
H=$(cd "$(dirname "$0")" && pwd); B=$H/banc_recherche.py
: "${PURGE_BIN:?}" "${REQUETE:?}"
NOM=$(basename "$BASE" .rag3db); FTS=$BASE.fts
O=$H/sorties/$NOM-$MODE-$(date +%Y%m%d-%H%M%S); mkdir -p "$O"
exec > >(tee -a "$O/fenetre.log") 2>&1
REQUETES=(commit "index du plein texte" "point de reprise" mutex "fermer un backend" "chunk_uuid" "les liens par COPY" "test e2e")
dit() { echo "== $(date +%H:%M:%S) $*"; }
rouge() { echo "ROUGE : $*"; [ "$MODE" = fenetre ] || [ "$MODE" = dedans ] && echo "La base reste fermée. Retour : ${RETOUR:-aucun instantané}"; exit 1; }
banc() { if [ "$MODE" = fenetre ]; then python3 -I "$B" hote "$MANIFESTE" "$1" "${REQUETES[@]}"; else python3 -I "$B" autonome "$MANIFESTE" "$1" "${REQUETES[@]}"; fi; }
mots_de() { case $1 in Scope|ScopeContent) echo "content:fn,commit,index,return,self,test" ;; File) echo "path:docs,md,rs,src" ;; *) echo ":" ;; esac; }

dit "$MODE $BASE ; sorties $O"
[ -d "$FTS" ] || rouge "$FTS n'existe pas"

if [ "$MODE" != compter ] && [ "$MODE" != dedans ]; then
  dit "1. banc de recherche avant"
  banc "$O/banc-avant.json" || rouge "banc d'avant"
fi

if [ "$MODE" = fenetre ]; then
  : "${INSTANTANES:?}" "${ETIQUETTE:?}"
  dit "2. fermeture qui tient, puis instantané $ETIQUETTE"
  python3 -I "$B" fermer "$MANIFESTE" "purge des orphelins du plein texte (hotfixs et G)" "$JUSQU_A" || rouge "fermeture"
  bash "$INSTANTANES" "$ETIQUETTE" "$BASE" || rouge "instantané (la fermeture tient ; rouvrir par : python3 $B ouvrir $MANIFESTE)"
  RETOUR=~/.local/share/rag3weaver/instantanes/$ETIQUETTE/RETOUR.sh
else
  [ -z "$(fuser "$BASE" 2>/dev/null)" ] || rouge "la copie est ouverte"
fi

dit "3. orphelins à froid"
TABLES=()
for t in "$FTS"/Lucivy_*; do
  [ -d "$t" ] || continue; T=${t##*/Lucivy_}
  # **Garde : le graphe doit s'être lu.** Un outil qui ne démarre pas (bâti pour un autre processeur,
  # 14 nov. à luciepc) rendait zéro ligne sans le dire, et tout le .fts paraissait orphelin.
  "$REQUETE" "$BASE" "MATCH (n:\`$T\`) RETURN offset(id(n))" > "$O/$T-requete.txt" 2>"$O/$T-requete.err"
  grep -q '^> MATCH' "$O/$T-requete.txt" && ! grep -q '^ERREUR' "$O/$T-requete.txt" || { head -3 "$O/$T-requete.err" "$O/$T-requete.txt"; rouge "la lecture du graphe de $T a échoué"; }
  grep -E '^[0-9]+$' "$O/$T-requete.txt" | sort -u > "$O/$T-lignes.txt"
  PURGE_TABLE=$t PURGE_SORTIE=$O/$T-docs-brut.txt "$PURGE_BIN" node_ids_vivants --ignored --nocapture > "$O/$T-vivants.log" 2>&1 || rouge "relevé du .fts de $T"
  sort -u "$O/$T-docs-brut.txt" > "$O/$T-docs.txt"
  comm -13 "$O/$T-lignes.txt" "$O/$T-docs.txt" > "$O/$T-orphelins.txt"
  sans=$(comm -23 "$O/$T-lignes.txt" "$O/$T-docs.txt" | wc -l)
  n=$(wc -l < "$O/$T-orphelins.txt"); docs=$(wc -l < "$O/$T-docs.txt")
  # **Garde : une purge n'efface jamais la majorité d'une table.** Plus de la moitié d'orphelins dit une
  # lecture fausse (mauvaise base, mauvaise table), pas un .fts en trop.
  [ "$docs" -eq 0 ] || [ $((n * 2)) -le "$docs" ] || rouge "$T : $n orphelins pour $docs documents, plus de la moitié : lecture à vérifier, rien n'est effacé"
  echo "TABLE $T : lignes $(wc -l < "$O/$T-lignes.txt"), documents vivants $(wc -l < "$O/$T-docs-brut.txt"), orphelins $n, lignes sans document $sans"
  [ "$n" -gt 0 ] && TABLES+=("$T")
done

if [ "$MODE" = compter ]; then dit "compte fini ; sorties $O"; exit 0; fi

for T in "${TABLES[@]}"; do
  IFS=: read -r champ mots <<< "$(mots_de "$T")"
  dit "4. purge de $T ($(wc -l < "$O/$T-orphelins.txt") orphelins)"
  PURGE_TABLE=$FTS/Lucivy_$T PURGE_IDS=$O/$T-orphelins.txt PURGE_CHAMP=${champ:-content} PURGE_MOTS=$mots \
    "$PURGE_BIN" purge_orphelins --ignored --nocapture > "$O/$T-purge.log" 2>&1 || { grep -E 'BILAN|check|panicked' "$O/$T-purge.log"; rouge "purge de $T"; }
  grep -E '^BILAN|^PURGE' "$O/$T-purge.log"
  dit "4. compact de $T"
  PURGE_TABLE=$FTS/Lucivy_$T PURGE_CHAMP=${champ:-content} PURGE_MOTS=$mots \
    "$PURGE_BIN" compacter --ignored --nocapture > "$O/$T-compact.log" 2>&1 || { grep -E 'BILAN|check|panicked' "$O/$T-compact.log"; rouge "compact de $T"; }
  grep -E '^BILAN COMPACT|^COMPACT' "$O/$T-compact.log"
done

if [ "$MODE" = fenetre ]; then
  dit "5. réouverture"
  python3 -I "$B" ouvrir "$MANIFESTE" || rouge "réouverture"
fi

if [ "$MODE" = dedans ]; then dit "VERT (dedans) ; sorties $O"; exit 0; fi
dit "6. banc de recherche après"
banc "$O/banc-apres.json" || rouge "banc d'après"
BANC_LIMITE=100 banc "$O/banc-apres-large.json" || rouge "banc large d'après"
python3 -I "$B" comparer "$O/banc-avant.json" "$O/banc-apres.json" "$O/banc-apres-large.json" | tee "$O/comparaison.txt"
tail -1 "$O/comparaison.txt" | grep -qx VERT || rouge "un résultat d'avant manque même dans les 100 premiers d'après : lire $O/comparaison.txt"
dit "VERT ; sorties $O"
