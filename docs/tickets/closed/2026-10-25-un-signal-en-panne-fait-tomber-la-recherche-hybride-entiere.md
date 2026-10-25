# Un signal en panne fait tomber la recherche hybride entière

- **État** : corrigé le 25 octobre 2026 (session « recherche ») : la branche dense demande à l'instantané si son index se lit avant de le lire. Ouvert le même jour (vu par G sur l'hôte de Lucie).
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

## Le correctif (25 octobre 2026)

La cause : sous L6, les signaux d'un appel lisent dans une seule transaction `READ ONLY`
(`read_pool.rs`), et le moteur condamne une transaction manuelle dès qu'une de ses instructions
échoue (marche T0). Le `QUERY_VECTOR_INDEX` d'un index absent échouait au binder ; le plein texte,
le creux et la résolution des morceaux tombaient ensuite sur le refus.

- `VectorSearchNode` demande d'abord à l'instantané si son index se lit
  (`SchemaDialect::vector_index_listing` : `CALL SHOW_INDEXES()` pour rag3db, sans paramètre,
  qui lit le catalogue de la transaction ; rien pour pgvector, qui lit la colonne sans index).
  Cette question ne peut échouer ni à l'analyse ni au binder. Une première forme,
  `WHERE table_name = $table`, échouait à l'analyse (`table` est réservé) et condamnait
  l'instantané à son tour : les témoins l'ont vue rouge dès la recherche d'avant le retrait.
- Absent, la branche se replie en **indisponible** (pas en échec), sans qu'aucune instruction
  n'échoue : la transaction reste saine, les autres signaux répondent.
- La raison est dite dans la réponse (`meta.warnings`, rendue en pied) : « the vector index `…`
  is dropped while its vectors catch up » si le drapeau `vector_index_dropped:{table}:{index}`
  est posé, sinon « the vector index `…` is missing ».
- Témoins (`tests/e2e_lectures.rs`, par `call_in`, sous l'instantané) :
  `la_recherche_rend_les_mots_pendant_le_rattrapage_des_vecteurs` et
  `la_recherche_rend_les_mots_quand_l_index_vectoriel_manque`.

Reste, pour le cœur : une autre faute d'une branche condamnerait encore l'instantané partagé.
Ticket `2026-10-25-une-instruction-qui-echoue-condamne-une-transaction-en-lecture-seule`.
