# search_code ne fusionne pas le dense par défaut

- **État** : ouvert (19 novembre 2026)
- **Famille** : produit (recherche)
- **Gravité** : la recherche par défaut du code se fait par mots seuls. Le sens (le dense) ne compte que si l'appelant
  demande `signals: ["vector"]`, et aucun agent ne le demande.
- **Atteignable en service** : oui, à vérifier sur les bases de Lucie ; mesuré sur une copie de la base du banc.
- **Touche rag3weaver** : oui ; la ligne fautive n'est pas encore trouvée.
- **Pour** : everything declarative
- **Signalé par** : ED, pendant la mesure de 2.12 (le namespace devant la classe), sur la demande de départage de c7.

## Ce qu'on voit

Sur une copie reflink de `banc-parite/ecr-base-ecrivain-neuf-5-lot-reindex-20261115` (forme `Generations`), à
luciepc, lib 6e2476d41, sonde `sonde_parite_search_code::releve_le_namespace_devant_la_classe` (branche
`namespace-probe`, e0859ad3fb). Requête « rag3db », filtrée sur `src/include/`, limite 100 :

| `options.signals` | résultats | avec `vector` | avec `bm25` |
|---|---|---|---|
| absent (le défaut) | 100 | **0** | 100 |
| `["vector"]` | 100 | 100 | 0 |
| `["bm25"]` | 100 | 0 | 100 |

Le réglage par défaut rend **la même liste que BM25 seul, au rang et au score près**. Le dense marche quand on le
demande : l'embarqueur et les vecteurs sont là (l'index dit « vecteurs 88 % »). La réponse par défaut ne dit nulle
part que le dense aurait été replié.

## Ce qui est écarté

- **Une déclaration** : le manifeste de la base ne déclare ni `workspace.index_signals` ni `search`. Le graphe
  `templates/tools/search_workspace.mmd` ne pose que des poids (`bm25:0.45, vector:0.55, sparse:0.4`), pas de signaux.
- **Le code lu** : un agent de lecture a suivi la résolution de la cible (`resolve_search_target` dans
  `src/catalog.rs`, `src/read_view.rs`) et `SearchSourceNode` (`src/dataflow/generic_search_nodes.rs`). Par le code,
  le défaut d'une base à versions est hybride : `ScopeContent` hérite des signaux de `Scope`, soit `HYBRID`. Il n'a
  trouvé aucune ligne qui retire `VECTOR`.
- **Une question de scores** : écartée par la limite 100. Aucun résultat du dense n'apparaît, même loin.

## Ce qu'il faut pour le fermer

- Rejouer l'appel par défaut avec le journal des nœuds, ou un témoin qui lit `target.default_signals` à
  l'exécution, pour trouver où `VECTOR` tombe : la cible vivante, un repli muet de la branche dense, ou la fusion.
- Rouge d'abord : un témoin de la lib, sur une base de code à versions avec vecteurs ; l'appel sans `signals` doit
  rendre au moins un résultat marqué `vector`.
- Puis le correctif. Ensuite la parité du banc rejouée : les réponses par défaut changeront, et c'est voulu.
