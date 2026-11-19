```json
{"parent": "0bae8b1a4cf5ca7b31e4ae31d4c459d7f2050ad8", "tree": "extension/rag3weaver", "needs_lib": true, "witness": "a_read_says_when_an_index_writes_its_view", "test": "scripts/cargo_magasin.sh test --lib --features rag3db-native,code,daemon -- a_read_says_when_an_index_writes_its_view", "suites": [], "hints": ["extension/rag3weaver/src/backend.rs", "extension/rag3weaver/src/catalog/retard.rs"], "ours": "b2272d070c7c887e42c707dbb90039d08f044c24"}
```

# Cas de campagne : une lecture qui se tait pendant tout un index

Session recherche, 19 novembre 2026, pour la campagne des correctifs passés (chantier de Lucie ; lanceur d'archi,
juge du banc). Un cas court, en Rust, sur l'attente d'index d'une lecture.

## Le point de départ

- **Commit parent** : `0bae8b1a4c` (18 novembre 2026). Le témoin y est, rouge ; le correctif n'y est pas.
- **Arbre** : `extension/rag3weaver`. **La lib rag3db est nécessaire** : le témoin ouvre un backend sur une base
  rag3db.

## Le signalement, tel qu'il était

> Pendant le premier index d'un dépôt, `usages` rend une réponse vide pendant toute la durée de l'index (46 s sur
> rag3weaver/src), sans rien dire, et l'état d'index ne dit qu'« occupé ». Un agent prend cette réponse vide pour
> une absence d'usages. Quand un index écrit la vue que lit l'appel, la réponse doit le dire tout de suite
> (`index: {ready: false, indexing: true}`, et une ligne « un index écrit ta vue »), sans attendre un index dont
> personne ne connaît la durée. Une fois l'index fini, plus rien ne doit être dit.

## Le témoin

- **Filtre** : `backend::tests::a_read_says_when_an_index_writes_its_view`, dans
  `extension/rag3weaver/src/backend.rs`. Identique sur master (vérifié le 19 nov. sur 3625a18b24).
- **Rouge au parent** : `answer["index"]` est absent (`Null`) au lieu de
  `{"ready": false, "indexing": true, "eta_s": null, "pending_files": 0}`.
- **Suites exigées par les chemins** : aucune (`backend.rs` n'est pas dans `tools/poste/required_suites.txt`).

## Ce que l'arbre du parent porte déjà

La marque d'écriture d'une couche existe (`Lag::is_writing`, `catalog/retard.rs`, posée par l'index). C'est la donnée
dont le correctif se sert. Le témoin la pose à la main.

## Notre correctif (caché à Algareth, pour le juge)

`b2272d070c`, « fix(index): une lecture dit qu'un index écrit sa vue, tout de suite, au lieu de se taire pendant
l'index complet » : `extension/rag3weaver/src/backend.rs` +29 −13.
