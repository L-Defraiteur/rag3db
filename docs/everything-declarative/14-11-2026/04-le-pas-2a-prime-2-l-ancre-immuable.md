# Le pas 2a′-2 : l'ancre immuable

Branche `generations-2a-prime-2`, partie de master 96f06eb87 après l'atterrissage de 2a′-1
(78c8997de). La page de 2a′ est la page 03 ; son § 7 dit ce que 2a′-1 a posé et ce qui reste.

## 1. Un trou de 2a′-1 comblé : la migration d'une base à versions

2a′-1 posait les générations sur une base **neuve** seulement. Une base déjà à versions
(`code-scope-v1/content`) restait à versions : `register_code_schema` la laissait « forme sans
couches », et aucun chemin ne la faisait monter. La page 03 (§ 5) annonçait pourtant une migration
triviale. Trouvé en préparant la mesure du drapeau, qui devait se faire sur une base à versions
réelle.

`code::migrer_vers_les_generations` la comble :
- la base doit être à versions, et son dialecte savoir les générations. Toute autre forme est
  refusée, comme un dialecte sans générations (postgres, ticket du 14 novembre) ;
- elle pose les trois colonnes sur `Scope` et `File` (la couche du backend, rang 0, vivante) et la
  génération 0 dans `_generation` ;
- l'estampille `code-scope-v1/content/generations` vient **en dernier**. Une migration interrompue
  laisse une base à versions, qui se relit comme avant et se remigre ;
- aucun uuid ne change, et aucune arête n'est réécrite.

Le témoin est `code::tests_des_generations::une_base_a_versions_monte_aux_generations` : rouge sur
8e6c7b0a4, où la fonction vide rend son erreur, puis vert sur 3a7cf136c.

**Rien à rattraper.** Aucune base de Lucie n'est encore à versions, et 1b n'a pas atterri sur une
vraie base (c7, 14 novembre 2026).

## 2. La mesure du drapeau `tout_visible`, avant de le faire tomber

C'est la condition de c7 (page 03, § 7) : le coût de la clause sans filtre se mesure avant que
`marquer_des_lignes_invisibles` l'impose.

**La base** : une copie reflink de `code-rag3db-depot`, migrée par 1b (79 662 scopes, 72 652
versions, un modèle granite, forme `ScopeContent`). **Ce n'est pas code-lourd**, mais l'index du
dépôt rag3db, de même taille. L'instantané de G (`avant-0c41f9fe6/code-lourd`) est de la forme d'avant
les versions : sur lui, 2a′-1 ne pose pas de générations, et le masque des versions ne s'y joue pas.
La sonde y applique la migration du § 1 et en relève la durée : c'est le chemin réel d'une base qui
monte.

**La sonde** : `tests/sonde_recherche_sans_filtre.rs`, sous `poste mesure`, seul sur le poste.

Trois états :
- **A** : le drapeau tenu, comme toute base de 2a′-1 ;
- **B** : le drapeau tombé, toutes les lignes visibles ;
- **C** : B, et 1 % des `Scope` retirés au rang lu.

Dans chaque état, trois mesures séparées :
- **(a)** les racines des occurrences : `select_all` en A, le repli d'`occurrence_roots_for` ; la
  clause de visibilité puis `resolve_offsets` en B et C (`admitted_parent_uuids_for`) ;
- **(b)** le masque des versions : `derived_references`, par lots de 5 000 racines ;
- **(c)** `search_code` de bout en bout, sur 20 requêtes fixes (les 10 noms les plus appelés, et 10
  phrases).

3 tours de chauffe, puis la médiane et le p95 de 20 tours.

**Ce que la lecture du code a appris, avant la mesure** : en A, une recherche sans filtre sur une base
à versions paie **déjà** le masque des versions (b). Le repli lit toutes les racines par `select_all`,
puis leurs références par lots. Le drapeau ne fait que remplacer `select_all` par la clause et
`resolve_offsets`. **L'écart de A à B sur (c) est donc le coût du drapeau.**

**Le seuil** : plus de 20 % ou plus de 50 ms ajoutés à la médiane de `search_code`. Au-delà, la clause
sans filtre change de forme avant que le drapeau tombe. La forme de repli : l'ensemble des offsets
visibles gardé en mémoire par (couche, rang), calculé une fois à la publication d'une génération.

**Les chiffres** : à venir.

## 3. La suite de 2a′-2 (page 03, § 7)

Dans l'ordre accepté par c7 :
- la mesure du § 2, et ses chiffres à c7 avant le drapeau ;
- `outline` dans le témoin de non-fuite ;
- le `#[allow(dead_code)]` de `marquer_des_lignes_invisibles` retiré avec son appelant, l'écrivain
  d'une seconde génération ;
- les arêtes et la forme C du § 6 de la page 03.

Chaque témoin est vu rouge d'abord.
