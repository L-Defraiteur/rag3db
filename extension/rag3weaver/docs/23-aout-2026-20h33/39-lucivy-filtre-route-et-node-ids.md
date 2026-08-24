# Filtre pré-calculé : seuls les shards concernés travaillent — FTS et sparse

Trois changements qui touchent votre `search_filtered(allowed_ids)` des deux
côtés, plus deux petites API. Poussés sur `v3-recovery` (voir `git log -3`).

## 1. FTS : `search_filtered` ne visite plus que les shards qui tiennent les ids

Avant : le jeu `allowed_ids` entier partait à **tous** les shards, et chaque
shard balayait la colonne `_node_id` de **tous** ses segments pour bâtir son
bitset — même quand vos 12 ids survivants étaient tous sur un shard.

Maintenant : le routeur sait sur quel shard chaque `node_id` a été inséré ;
les ids sont groupés par shard, chaque shard reçoit sa part, et un shard
qui n'en tient aucun n'a **pas de nœud de recherche** dans le DAG (pas de
balayage, pas de scoring, rien à fusionner). Les statistiques BM25 restent
globales (le prescan ne change pas), donc **les scores sont identiques** au
classement non filtré restreint aux ids autorisés — c'est ce que la garde
vérifie pour un id, un shard entier, des ids dispersés, tous, et des ids
inconnus.

Cas de repli : un id que le routeur n'a jamais vu (index construit avant
que le routage soit persisté) renvoie au comportement d'avant, tout partout.
Un jeu vide rend une liste vide sans rien lancer.

## 2. Les ex æquo sont départagés de façon déterministe

Deux résultats de même score sortaient dans un ordre arbitraire (tas
binaire). Ils sortent maintenant par `(shard, segment, doc)` croissants :
une recherche est reproductible, et une recherche filtrée est une
sous-séquence de la recherche complète. Si un de vos tests comparait des
listes à score égal en tolérant l'ordre, il peut devenir strict.

## 3. Sparse : le filtre passe sous le scoring

`SparseHandle::search_filtered` ne filtre plus après avoir scoré la
fenêtre : quand `|ids| × lanes` est petit devant les postings à parcourir,
il **seek** chaque lane sur chaque id autorisé et ne score que ceux-là
(`wand::search_ids`) ; sinon, fenêtre + filtre comme avant. Mêmes sommes
f32 dans les deux cas. Et `ShardedSparseHandle::search_filtered` fait le
même routage par shard que le FTS (§1), via `Pool::scatter_to`.

## 4. Deux API que vous attendiez sans le dire

```rust
let hits = handle.search_filtered(&q, 10, None, allowed)?;
let ids: Vec<u64> = handle.node_ids_of(&hits)?;     // _node_id par résultat, fast field, sans charger le document
let where_is = handle.shard_for_node_id(id);         // Option<usize>
```

`node_ids_of` est ce qu'il faut pour joindre des hits avec vos records
sans passer par `search_with_docs`. Le sparse a déjà `shard_for_node_id`.

## Chez vous

- Épingler le HEAD. Aucun changement de signature sur ce que vous
  appelez déjà.
- Si vous passiez par `search_with_docs` uniquement pour récupérer
  `_node_id`, `node_ids_of` évite le chargement des documents.
- Vos `allowed_ids` viennent d'un filtre Cypher en amont : c'est exactement
  le cas que ce routage accélère — mesurez sur une base à plusieurs shards
  avec un filtre sélectif, le gain doit être visible dès 4 shards.
