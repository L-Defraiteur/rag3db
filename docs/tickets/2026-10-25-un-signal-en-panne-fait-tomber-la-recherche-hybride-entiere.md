# Un signal en panne fait tomber la recherche hybride entière

- **État** : ouvert, le 25 octobre 2026 (vu par G sur l'hôte de Lucie ; confié à la session « recherche »).
- **Gravité** : blocage. `search_code` ne répond plus du tout, alors que le plein texte est prêt.
- **Atteignable en service** : oui. Il suffit d'un rattrapage de vecteurs au-delà du seuil (2000 morceaux), qui retire l'index vectoriel par conception.
- **Touche rag3weaver** : oui (`FuseResultsNode`, `BM25SearchNode`, `VectorSearchNode`).

## Ce que c'est

Pendant que l'index vectoriel `Scope_Chunk_vec__granite_278m` est retiré pour un rattrapage, la branche
dense échoue au binder. L'échec annule la transaction que les branches partagent : le BM25 échoue
à son tour, et la fusion rend une erreur au lieu des résultats du plein texte.

```
FuseResultsNode: every signal branch is down — text signal failed: BM25SearchNode: search failed:
… The transaction was rolled back because a statement in it failed … l'instruction qui l'a défaite :
Binder exception: Table Scope_Chunk doesn't have an index with name Scope_Chunk_vec__granite_278m. ;
dense signal failed: VectorSearchNode: search failed: … Binder exception: Table Scope_Chunk doesn't
have an index with name Scope_Chunk_vec__granite_278m.
```

## La recette

Un backend de code dont la dette de vecteurs dépasse 2000 morceaux : le journal dit « l'index
`Scope_Chunk_vec__…` tombe le temps du rattrapage ». Puis `search_code` en mode `indexed`. Vu le
25 octobre 2026 à 15 h 52, sur l'hôte de Lucie (44c790c20, lib 58b46a9bf), après la réouverture du
backend.

## Le témoin

À écrire : un catalogue avec un index vectoriel retiré, une recherche hybride, et le plein texte qui
répond seul en disant que la branche dense manque.

## Pour le fermer

Chaque branche lit dans sa propre transaction, ou la branche dense vérifie que son index existe avant
de lire. Une branche en panne se dit dans la réponse ; les autres répondent quand même.
