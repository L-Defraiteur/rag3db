# Une méthode propre contre une méthode de trait de même nom

- **État** : corrigé le 19 octobre 2026 (codeparsers). La voie (a) : codeparsers marque `trait_impl`
  la méthode d'un `impl Trait for Type`, rag3weaver la garde sur le scope, et `choose_target` fait
  passer la méthode propre avant, à types égaux. Le témoin est rejoué vert par le vrai chemin.
- **Gravité** : relation manquante. L'appel n'est pas relié, et rien de faux n'est posé.
- **Atteignable en service** : oui. `impact row_to_map` ne nomme pas les tests qui appellent
  `catalog.get(…)`.
- **Touche rag3weaver** : oui (`src/code.rs`, `choose_target`). Peut-être codeparsers, s'il ne dit
  pas qu'une méthode est portée par une impl de trait.

## Ce qui se passe

`phase0_create_drain_all_field_types` et `phase0_update_and_delete` (tests/e2e_search.rs)
appellent `catalog.get("Document", &uuid)`, et le type `Catalog` se lit bien sur la mention. Mais
deux définisseurs de `get` ont `Catalog` pour parent :
- la méthode propre de `Catalog` (src/catalog.rs) ;
- le `get` de `impl ReadCatalog for Catalog` (src/read_view.rs).

`choose_target` en voit deux pour le même type et s'abstient. Rust tranche par une règle que la
résolution ne connaît pas : la méthode propre d'un type passe avant la méthode d'un trait de même
nom.

## Pourquoi le témoin passait sur master

Le témoin `impact_nomme_les_tests_que_des_changements_reels_ont_casses` atteignait ces tests à
trois sauts. Le chemin passait par phase0 → `Catalog::exists`, `count` ou `drainer` (typés,
justes) → `Catalog::get` → `row_to_map`. Le maillon `exists` → `get` était posé par le choix du
même fichier, et il est faux : dans `exists`, les `get` sont des `Vec::get` et des `map.get`. Le lot
qui retire ce choix (« une relation douteuse se compte et se dit, elle ne se devine pas », 19
octobre 2026) a donc cassé un vert obtenu par une devinette. Trace prise dans le témoin, par une
instrumentation de `choose_target`, non commitée.

## Le correctif tranché

(a), par c7. À types égaux, un définisseur porté par un bloc `impl Trait for Type` cède devant
celui du type lui-même. Il faut savoir, au niveau du scope, qu'une méthode est portée par une impl
de trait. Si codeparsers ne le dit pas, l'ajouter. Le lot se mesure sur code-lourd (sonde contre
master), et le cas `row_to_map` revient dans le témoin, vert par le vrai chemin, dans le même
commit.

(b) a été écarté : les deux arêtes, marquées ambiguës, ce serait encore deviner.

## Le témoin

`impact_row_to_map_nomme_les_tests_qui_appellent_catalog_get` (tests/e2e_impact.rs), joué sans
saut depuis le correctif ; et `la_methode_propre_passe_avant_celle_du_trait` (unitaire, code.rs),
`une_methode_d_impl_de_trait_porte_trait_impl` (codeparsers, tests/blocs_impl.rs).
