# `usages` lit des conteneurs sans la borne des générations

- **État** : fermé (18 novembre 2026, session recherche). Défaut prouvé par un rouge, puis corrigé.
- **Ouvert le** : 18 novembre 2026, session recherche.
- **Pour** : recherche
- **Famille** : produit
- **Gravité** : à départager. Sous les générations (2a′), une réponse de `usages` pourrait montrer la déclaration
  d'un conteneur retiré.
- **Atteignable en service** : à départager. Il faut une base sous générations et un conteneur qui déclare un nom
  (C++ : `declarations` sur la classe d'un en-tête), retiré par une édition.
- **Touche rag3weaver** : oui (`src/dataflow/usage_nodes.rs`)

## Ce que c'est

Relevé en concevant B des outils de référence (page
`docs/recherche/18-11-2026/02-les-outils-de-reference-avant-l-index.md`). `usage_nodes.rs` traduit six de ses
lectures par `catalog.dialect_arc().select(…)`, en direct : lignes 150, 203, 248, 322, 526 et 580 sur master
de576c6709.

Elles ne passent pas par `ReadCatalog::traduire_select`, qui borne une sélection aux lignes visibles
(`Visibilite::borner_select`). Les sauts de `neighborhood_nodes.rs`, eux, passent par `traduire_hop` et
`traduire_select`.

Quatre lectures partent d'uuids déjà rendus par une lecture bornée (150, 322, 580), ou d'un pivot par son nom
(526, la table des symboles). Celles qui comptent sont les deux sélections **par contenu** de la table des
conteneurs (203 et 248 : `Contains` sur `declarations`). Elles peuvent rendre une ligne retirée qui porte encore le
nom.

## Ce qu'il faut pour le fermer

- **Un témoin, rouge d'abord** : sous générations, une classe C++ déclare `bar` dans un en-tête, une édition la
  retire et la synchronisation la retire. Puis `usages bar` ne montre pas la déclaration retirée.
- **Le correctif** : les six lectures passent par `traduire_select`. Le dialecte mémoire de B en a besoin de toute
  façon : il remplace la traduction à cet endroit.

## Ce qui le ferme (18 novembre 2026, recherche)

- **Le rouge** : `e2e_code_sync::usages_does_not_show_the_declaration_of_a_retired_container` (f560496d6d,
  passe `passe-visibilite-201031` à luciepc). Après l'édition, la ligne de `Foo` reste dans sa table, retirée.
  `usages bar` rendait encore « déclaré dans Foo — foo.h:3 : `int bar(int x) const` ».
- **Le correctif** : les six sélections de `usage_nodes.rs` passent par `ReadCatalog::traduire_select`, qui les borne
  aux lignes visibles, comme les sauts de `neighborhood_nodes.rs`.
