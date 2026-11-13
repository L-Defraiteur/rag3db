# Une recherche filtrée ne classe pas comme sa vue

- **État** : ouvert (13 novembre 2026, au soir). Le témoin est rouge ; le correctif se fait le 14,
  avant la mesure du plein texte. Version 4.3.7 sans tag, publication à la décision de Lucie.
- **Gravité** : classement faux. Les bons documents sont trouvés, mais dans un mauvais ordre de score.
- **Atteignable en service** : oui. Toute recherche lucivy filtrée par `allowed_ids`, ce que feront les
  vues par génération (page 30 d'arbre principal).
- **Touche rag3weaver** : oui, par ses recherches filtrées.
- **Famille** : moteur

## Ce que c'est

Une recherche filtrée par `allowed_ids` doit classer comme si l'index ne contenait que les documents
permis. C'est ce que prévoient `AggregatedBm25StatsOwned::with_subset_docs`
(`lucivy_core/src/bm25_global.rs:58-75`) et son appel dans `lucivy_core/src/search_dag.rs:451`, sous la
condition `filtered && !self.prescanned`. Le commentaire du code cite la mesure qui l'avait justifié :
« with N global and df on ten documents, one hit went from 0.02 to 2.9 ».

Le témoin montre que ce n'est pas le cas sur le chemin de `ShardedHandle::search_filtered`.

## Le témoin (lucivy, branche `4.3.6-score-filtre-par-terme`, 68733ad)

`lucivy_core/tests/test_filtered_scores_match_the_subset.rs` construit :
- un index de 100 documents de même longueur (4 mots), dont 50 contiennent « alpha » ;
- une recherche filtrée sur 10 d'entre eux ;
- les mêmes 10 documents seuls dans un second index.

Pour `term`, `contains`, `phrase` et `parse`, les scores doivent être égaux document par document.
Tous les documents ayant la même longueur, la longueur moyenne, gardée exprès sur le corpus, ne fausse
pas la comparaison.

Rouge sur 1 comme sur 2 shards, pour les quatre types (31 différences) :

| Requête | Filtrée | Les 10 seuls |
|---|---|---|
| `term`, `contains`, `parse` « alpha » | 2,2637 | 0,0465 |
| `phrase` « alpha item3 » | 4,2097 | 1,9924 |

2,26 correspond à N = 100 (tout l'index) avec df = 10 (la vue) : `with_subset_docs` n'est pas appliqué.

## Où chercher

- `search_filtered` → `search_internal` → `search_once` → le DAG de recherche. Lire pourquoi
  `subset_docs` reste `None` : la condition `filtered && !self.prescanned`, ou un autre chemin qui ne
  passe pas par ce nœud.
- Ensuite, pour les requêtes qui interrogent l'index inversé sans pré-recherche (`phrase`, peut-être
  `parse`) : `doc_freq(term)` (`bm25_global.rs:109-115`) somme sur tout l'index. Il faudra aussi compter
  ce df dans la vue.

## Ce qui reste à décider

Rien. Le comportement voulu est écrit dans le code et dans son commentaire. Il s'agit de le rendre
effectif, rouge puis vert, sur 1 et 2 shards et pour les quatre types.
