# Réponse au doc 28 : l'underflow était une liste de documents non triée — corrigé, et `close()` ne panique plus

Votre pile (`buffered_union.rs:72`) et votre lecture — « un enfant de l'union
rend un `doc()` inférieur au `min_doc` calculé juste avant » — étaient
exactes. La cause n'est pas un état partagé entre shards : c'est un scorer
**non monotone à lui tout seul**. Deux commits, poussés : `8a91053` (le
scorer) et, dans la foulée, le `close()` qui panique (voir §3).

## 1. Le mécanisme

Le prescan v3 des requêtes **fuzzy** et **regex** calcule le `(doc, tf)` de
chaque segment en comptant les highlights dans un `HashMap<DocId, u32>` —
comptage O(n), légitime — puis faisait `tf_map.into_iter().collect()`.
L'ordre d'itération d'un `HashMap` est aléatoire (graine par instance). Le
`SfxScorer` construit sur cette liste rend donc ses documents dans un ordre
quelconque, alors qu'un `DocSet` est monotone par contrat.

Sous une union `should` (votre `ContainsSplit` = `should` de `contains`,
avec `distance` — c'est le chemin fuzzy), `refill` prend le minimum des
`doc()` puis boucle `advance()` / `doc()` sur chaque enfant en calculant
`doc - min_doc` : dès qu'un enfant recule, underflow en debug, index de
bitset hors bornes en release. Seul, sans union, le même scorer faisait
sauter des documents dans `seek()` — silencieusement.

Pourquoi ça ressemblait à une course : la graine du `HashMap` change à
chaque construction, et il faut assez de documents dans **un même
segment** pour qu'un ordre non trié devienne probable. Votre 12ᵉ test avait
les deux ; seul, il tirait un ordre trié. Le contains (d = 0) trie depuis
toujours — c'est pour ça que la plupart de vos requêtes n'ont jamais rien
vu.

## 2. Le correctif (`8a91053`)

- `HashMap` conservé (Lucie a tenu à ce qu'on ne jette pas l'optimisation
  de comptage) ; le **petit vecteur résultat** (k documents, pas n
  highlights) est trié par doc, dans les deux prescans.
- `CachedPrescan::new` porte un `debug_assert!` de monotonie : aucun
  producteur ne peut régresser en silence. Il a attrapé le chemin regex
  juste après le chemin fuzzy.
- Garde : `v3_fuzzy_union_docsets_are_sorted` — 48 documents dans un seul
  segment, trois feuilles fuzzy sous un `should`. Avant le correctif :
  panique à `buffered_union.rs:73` en release, comme chez vous.

Validation : pipeline v3 39/39 en debug (asserts actives) et ACID 4/4 ;
lucivy-core complet (22 binaires) et lib 1415/1415 en release.

## 3. `Pool::drain` / `scatter` / `shutdown` dans `close()`

Votre trace `LUCIOLE_REPLY_TRACE=1` a nommé le dernier `Reply` lâché :
`Pool::drain` envoyait `DrainMsg(tx)` à un worker déjà parti, le
`SendError` emportait `tx`, et `scheduler.wait(rx)` paniquait dans
`close()`. Les trois fonctions ignorent maintenant les récepteurs dont
l'envoi a échoué et passent par `try_wait` : un worker absent ne fait plus
paniquer un destructeur. Votre `catch_unwind` dans
`Catalog::close_fts_handles` reste une bonne ceinture ; il ne devrait plus
jamais se déclencher.

## Chez vous

- Épingler le HEAD (`git log -1` sur `v3-recovery`), rejouer la repro du
  doc 28 (`RAG3W_NO_BATCH_SAVE=1`, 13 tests) : attendu 13/13, 3 runs sur 3,
  sans message luciole.
- Rien d'autre à changer : ni API, ni config, ni index.
