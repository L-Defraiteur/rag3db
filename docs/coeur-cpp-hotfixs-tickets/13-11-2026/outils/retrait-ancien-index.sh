#!/bin/bash
# retrait-ancien-index.sh (14 nov. 2026, hotfixs) — le retrait de l'ancien index d'une base de code passée par 1b.
#
#   répétition, sur une copie : retrait-ancien-index.sh repetition <copie/nom.rag3db> <backend.json de la copie>
#   fenêtre, sur une base de l'hôte : retrait-ancien-index.sh fenetre <nom.rag3db> <backend.json> "<jusqu'à>"
#
# Après la bascule de 1b, l'ancien index reste dans la base pour le retour, et le produit ne le lit plus :
# Scope_Chunk (ses morceaux, leurs vecteurs, son index HNSW), Scope_CHUNKED_FROM, Lucivy_Scope. Le pas :
# 0. les préalables, lus en lecture seule — la forme à versions (`code_scope_form = code-scope-v1/content`),
#    Scope routé par ScopeContent et sans signaux, des morceaux ScopeContent_Chunk, pas d'estampille de retrait ;
# 1. (fenêtre) la fermeture qui tient ; 2. le jeu de 8 requêtes avant, par le binaire en local (search_code
#    en bm25 et en bm25+vector, read_file, usages) ; 3. l'instantané avec sa recette RETOUR.sh ;
# 4. DROP de l'index HNSW de Scope_Chunk, de Scope_CHUNKED_FROM, de Scope_Chunk ; les clés de méta de
#    l'ancien index retirées ; l'estampille `code_scope_retour` = « impossible … » ; CHECKPOINT ;
# 5. l'effacement de Lucivy_Scope ; 6. le contrôle : plus de table ni de dossier, l'estampille posée ;
# 7. le jeu après, identique à celui d'avant hors `/result/run` ; 8. rien n'est recréé à l'ouverture ;
# 9. les octets rendus ; 10. (fenêtre) la réouverture.
# Un pas rouge arrête tout. Le retour est le RETOUR.sh de l'instantané (avant l'estampille, tout revient).
#
# Variables : REQUETE (lecture seule) et ECRIRE (écriture), bâtis contre la lib de la base sur ce poste ;
# BIN = rag3weaver-backend qui connaît la forme à versions ; LIB = le dossier de la lib au magasin ;
# INSTANTANES = instantanes.sh de G (en répétition, instantanes-repetition.sh et INSTANTANES_DIR) ; ETIQUETTE.
set -u
MODE=${1:?repetition|fenetre}; BASE=${2:?base.rag3db}; MANIFESTE=${3:?backend.json}; JUSQU_A=${4:-}
: "${REQUETE:?}" "${ECRIRE:?}" "${BIN:?}" "${LIB:?}" "${INSTANTANES:?}" "${ETIQUETTE:?}"
H=$(cd "$(dirname "$0")" && pwd)
NOM=$(basename "$BASE" .rag3db); FTS=$BASE.fts
O=$H/sorties/$NOM-$MODE-$(date +%Y%m%d-%H%M%S); mkdir -p "$O"
exec > >(tee -a "$O/retrait.log") 2>&1
dit() { echo "== $(date +%H:%M:%S) $*"; }
rouge() { echo "ROUGE : $*"; [ "$MODE" = fenetre ] && echo "La base reste fermée. Retour : ${RETOUR:-aucun instantané}"; exit 1; }
lire() { "$REQUETE" "$BASE" "$@" 2>/dev/null | grep -v '^$'; }
# Une valeur seule (la dernière ligne d'une requête à une colonne) ; refuse si la requête n'a pas parlé.
valeur() { local s; s=$(lire "$1"); grep -q '^> ' <<< "$s" && ! grep -q '^ERREUR' <<< "$s" || rouge "lecture : $1 — $s"; tail -1 <<< "$s"; }
jeu() { LD_LIBRARY_PATH=$LIB RAG3DB_LIBRARY_DIR=$LIB "$BIN" "$MANIFESTE" < "${JEU:-$H/jeu.jsonl}" > "$1" 2> "$1.err"; [ "$(wc -l < "$1")" = "$(wc -l < "${JEU:-$H/jeu.jsonl}")" ] || { tail -5 "$1.err"; rouge "le jeu n'a pas rendu ses réponses"; }; }
octets() { du -sb "$BASE" | cut -f1; }
octets_fts() { du -sb "$FTS/$1" 2>/dev/null | cut -f1; }

dit "$MODE $BASE ; sorties $O"
dit "0. préalables"
forme=$(valeur "MATCH (m:_catalog_meta {_key: 'code_scope_form'}) RETURN m._value")
[ "$forme" = "code-scope-v1/content" ] || rouge "forme « $forme » : pas une base à versions"
conf=$(valeur "MATCH (m:_catalog_meta {_key: 'entity_config:Scope'}) RETURN m._value")
grep -q '"searchIndex":"ScopeContent"' <<< "$conf" && grep -q '"signals":\[\]' <<< "$conf" || rouge "Scope n'est pas routé par ScopeContent sans signaux"
retour=$(valeur "MATCH (m:_catalog_meta) WHERE m._key = 'code_scope_retour' RETURN count(m)")
[ "$retour" = 0 ] || rouge "l'estampille code_scope_retour est déjà posée : retrait déjà fait"
morceaux_neufs=$(valeur "MATCH (c:ScopeContent_Chunk) RETURN count(c)")
[ "$morceaux_neufs" -gt 0 ] || rouge "aucun morceau ScopeContent_Chunk"
morceaux=$(valeur "MATCH (c:Scope_Chunk) RETURN count(c)"); aretes=$(valeur "MATCH ()-[r:Scope_CHUNKED_FROM]->() RETURN count(r)")
avant_base=$(octets); avant_fts=$(octets_fts Lucivy_Scope)
echo "forme $forme ; ScopeContent_Chunk $morceaux_neufs ; à retirer : Scope_Chunk $morceaux, Scope_CHUNKED_FROM $aretes, Lucivy_Scope $avant_fts o ; base $avant_base o"

if [ "$MODE" = fenetre ]; then
  dit "1. fermeture qui tient"
  python3 -I "$H/../fenetre-purge/banc_recherche.py" fermer "$MANIFESTE" "retrait de l'ancien index (hotfixs et G)" "$JUSQU_A" || rouge "fermeture"
fi
[ -z "$(fuser "$BASE" 2>/dev/null)" ] || rouge "la base est ouverte par un processus"

dit "2. jeu avant"
jeu "$O/jeu-avant.jsonl"

dit "3. instantané $ETIQUETTE"
"$INSTANTANES" "$ETIQUETTE" "$BASE" || rouge "instantané"
RETOUR=${INSTANTANES_DIR:-~/.local/share/rag3weaver/instantanes}/$ETIQUETTE/RETOUR.sh; [ -f "$RETOUR" ] || rouge "pas de RETOUR.sh"

dit "4. DROP"
STAMP="impossible : ancien index (Scope_Chunk, Scope_CHUNKED_FROM, Lucivy_Scope) retiré le $(date '+%Y-%m-%d %H:%M') sur $(hostname), instantané $ETIQUETTE"
ordres=()
lire "CALL show_indexes() RETURN *" | grep -q '^Scope_Chunk|Scope_Chunk_vec__granite_278m|' && ordres+=("CALL DROP_VECTOR_INDEX('Scope_Chunk', 'Scope_Chunk_vec__granite_278m')")
ordres+=("DROP TABLE Scope_CHUNKED_FROM" "DROP TABLE Scope_Chunk"
  "MATCH (m:_catalog_meta) WHERE m._key = 'fts_generation:Lucivy_Scope' OR m._key STARTS WITH 'vector_index_dropped:Scope_Chunk:' DELETE m"
  "MERGE (m:_catalog_meta {_key: 'code_scope_retour'}) SET m._value = '$STAMP'"
  "CHECKPOINT")
"$ECRIRE" "$BASE" "${ordres[@]}" > "$O/ddl.txt" 2>&1 || { grep -v 'extension VECTOR' "$O/ddl.txt"; rouge "un ordre du retrait a échoué (le reste n'est pas joué)"; }
grep -v 'extension VECTOR' "$O/ddl.txt" | grep '^> ' 
"$ECRIRE" "$BASE" "RETURN 1" > /dev/null 2>&1 || rouge "la base ne se rouvre pas après le DROP"

dit "5. effacement de Lucivy_Scope"
[ -d "$FTS/Lucivy_Scope" ] && rm -rf -- "$FTS/Lucivy_Scope"

dit "6. contrôle"
tables=$(lire "CALL show_tables() RETURN name")
grep -qx 'Scope_Chunk\|Scope_CHUNKED_FROM' <<< "$tables" && rouge "une table de l'ancien index est encore là"
[ "$(valeur "MATCH (m:_catalog_meta {_key: 'code_scope_retour'}) RETURN m._value")" = "$STAMP" ] || rouge "l'estampille n'est pas posée"
[ -e "$FTS/Lucivy_Scope" ] && rouge "Lucivy_Scope est encore là"
echo "ni Scope_Chunk, ni Scope_CHUNKED_FROM, ni Lucivy_Scope ; estampille : $STAMP"

dit "7. jeu après"
jeu "$O/jeu-apres.jsonl"
python3 -I "$H/comparer_jeu.py" "$O/jeu-avant.jsonl" "$O/jeu-apres.jsonl" | tee "$O/comparaison.txt"
tail -1 "$O/comparaison.txt" | grep -qx VERT || rouge "le jeu ne rend plus la même chose : lire $O/comparaison.txt"

dit "8. rien n'est recréé par l'ouverture"
lire "CALL show_tables() RETURN name" | grep -qx 'Scope_Chunk\|Scope_CHUNKED_FROM' && rouge "le binaire a recréé une table de l'ancien index"
[ -e "$FTS/Lucivy_Scope" ] && rouge "le binaire a recréé Lucivy_Scope"

dit "9. octets"
apres_base=$(octets)
echo "Lucivy_Scope : $avant_fts o rendus ; base : $avant_base → $apres_base o ; total rendu $((avant_fts + avant_base - apres_base)) o"

if [ "$MODE" = fenetre ]; then
  dit "10. réouverture"
  python3 -I "$H/../fenetre-purge/banc_recherche.py" ouvrir "$MANIFESTE" || rouge "réouverture"
fi
dit "VERT ; sorties $O"
