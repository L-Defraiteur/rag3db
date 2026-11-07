# Le plein texte explose sur une requête longue

- **État** : ouvert, le 7 novembre 2026. Mesuré par rag3weaver archi sur l'hôte en service (journal
  des runs, graphe `search_structured`, mémoire de la session d'essai des hooks, deux fiches) ;
  pour la session recherche, la borne attend la décision de c7.
- **Gravité** : blocage. Un appel tient le nœud plein texte des dizaines de secondes, et le nœud
  tourne jusqu'au bout après l'abandon du client.
- **Atteignable en service** : oui, par tout appelant qui envoie une longue requête (un prompt
  entier, ce que faisait le hook).
- **Touche rag3weaver** : oui (`BM25SearchNode`, `build_bm25_query`, `src/search.rs`).

## Ce qu'on voit

| requête | nœud `bm25` |
|---|---|
| 1 135 caractères, 197 mots | 32 936 ms |
| 729 caractères, 102 mots | 13 158 ms |
| 426 caractères, 16 mots (chargés d'ids) | 1 944 ms |
| un mot | 2 ms |

L'embarquement et le dense restent sous 140 ms sur les mêmes requêtes.

## Cause

Sur le chemin lucivy, le mode par défaut lance deux requêtes (159087582) : la requête entière en
`contains` relâché à distance 1, et, pour plusieurs mots, la recherche par mots (`Parse` : lucivy
la traduit en OU d'un `contains` flou par mot et par champ). Rien ne borne ni le nombre de termes
ni la longueur de la requête entière : le coût croît avec les deux.

## Le témoin

À écrire : une requête de 200 mots sur un petit corpus, le nœud sous un budget (par exemple
500 ms) et un avertissement dans `meta.warnings` qui dit la coupe ; une requête courte inchangée.

## Ce qu'il faut pour le fermer

Une borne dans le nœud, pour tout appelant, dite dans la méta (options soumises à c7) : au-delà
de K termes, la recherche par mots ne garde que K termes ; au-delà de L caractères, la requête
entière n'est plus cherchée en flou. Séparément, pour l'hôte : un nœud dont le client a abandonné
devrait s'arrêter (ticket à ouvrir par l'hôte s'il n'existe pas).
