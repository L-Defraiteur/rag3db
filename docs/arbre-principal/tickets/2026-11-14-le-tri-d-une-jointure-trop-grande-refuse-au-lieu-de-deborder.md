# Le tri d'une jointure trop grande refuse au lieu de déborder sur le disque

- **État** : ouvert (14 novembre 2026), arbre principal. Un souhait, pas un défaut.
- **Famille** : moteur
- **Gravité** : basse — le moteur refuse en le disant ; une requête trop grande pour le tampon n'a pas
  de résultat, même quand le disque en aurait la place.
- **Atteignable en service** : oui, par toute requête dont la jointure triée dépasse le tampon.
- **Touche rag3weaver** : non (le moteur) ; rag3weaver a contourné de son côté.
- **Pour** : cœur.
- **Signalé par** : arbre principal, migration 1b sur une copie de `code-rag3db-depot`, 14 novembre
  2026, lib `0c41f9fe6`, au portable ; sur la demande de c7.

## Ce que c'est

La migration 1b transférait les vecteurs par lots de 2 000 versions, par une seule requête :

```
MATCH (n:ScopeContent_Chunk)-[:ScopeContent_CHUNKED_FROM]->(v:ScopeContent)
      -[:ScopeContent_DERIVED_FROM]->(s:Scope)<-[:Scope_CHUNKED_FROM]-(o:Scope_Chunk)
WHERE v._uuid IN $versions AND o._parent_field = n._parent_field AND o._index = n._index
  AND o._text_hash = n._text_hash AND … AND o.embedding IS NOT NULL
RETURN n._uuid, n._text_hash, o.embedding ORDER BY n._uuid, s._uuid
```

Elle joint chaque morceau neuf aux morceaux de **toutes** les occurrences de sa version, vecteurs de
768 flottants compris, puis trie. Au lot 42 000 / 72 652 (79 662 occurrences), le moteur refuse :

> Buffer manager exception: Unable to allocate memory! The buffer pool is full and no memory could be
> freed!

Le tampon est de 8 Gio (la règle du produit). Ce n'étaient pas des pages sales. Chaque lot d'avant
était validé : 40 drains de dérivation, une transaction chacun, et 21 écritures de vecteurs en
auto-commit. Un `CHECKPOINT` explicite après chaque lot ne change rien : même refus, au même lot.
Les pages sont épinglées par l'opérateur de la requête.

## Ce qu'on voudrait

Qu'un tri ou une jointure plus grands que le tampon débordent sur le disque, comme un moteur établi
le fait (PostgreSQL : `work_mem` puis fichiers temporaires ; DuckDB : opérateurs hors mémoire), au lieu
de refuser. Ou, à défaut, que le refus nomme l'opérateur et sa taille.

## Ce qui a été fait de notre côté

rag3weaver ne dépend pas de ce souhait. Le transfert lit un représentant par version, puisque toutes
ses occurrences ont le même contenu, donc les mêmes morceaux. Il fait trois lectures bornées par le
lot, et apparie en Rust (`src/migration_des_versions.rs`, `transferer_les_vecteurs`, branche
`arbre-principal/migration-1b`).

## Le témoin à écrire (cœur)

Une jointure triée dont le résultat dépasse un tampon réduit (`RAG3DB_BUFFER_POOL_SIZE` petit) rend
son résultat, ou refuse en nommant l'opérateur et la taille demandée.
