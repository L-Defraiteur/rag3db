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

**Les chiffres** : au § 2 bis.

## 2 bis. Ce que la mesure a dit (14 novembre 2026, luciepc, `poste mesure`)

Le mot de Lucie, par c7 : « si on apporte la feature faut faire le max pour qu'elle devienne viable ». La
mesure ne porte donc plus seulement sur le drapeau : elle dit ce que coûte une recherche, étage par
étage, avant et après les versions.

**Les bases.**
- **R** : la base d'avant les versions, telle que les utilisateurs l'ont aujourd'hui. C'est une
  copie reflink de l'instantané de G `avant-835d5298f/code-rag3db-depot` : la même base que 1b a
  migrée, prise avant sa migration. Recopiée à luciepc, md5 identique. 79 662 `Scope`.
- **A, B, C** : la copie de `1b-copie/migree-20261114-1641`, montée aux générations par la
  migration du § 1. Elle a 79 662 occurrences et 72 652 versions.

**Le jeu** : 10 requêtes. Ce sont les 5 noms les plus appelés et 5 phrases, figés ensuite par banc
dans `tests/donnees/jeu-recherche-2a2.json`. Chaque requête est jouée sous trois formes :
- sans filtre ;
- filtrée par chemin, `src/include/` (12,4 % des scopes) ;
- filtrée par langage, python (8,1 %).

1 tour de chauffe, puis 5 tours. Les étages sont lus sur les `NodeRun` que le flux publie déjà, sans
rien ajouter au code de production.

**Une réserve sur le dense et le creux** : dans cette passe, ils n'ont tourné ni en R ni en A. La
sonde ouvrait le backend sans embarqueur, et l'adresse du manifeste ne suffit pas : le journal dit
« embarqueur absent ». Les 1 557 ms du nœud dense en A ne sont que son masque, calculé avant de
découvrir qu'il n'avait pas de modèle. Corrigé dans 02bcdc9e3 : les sondes ouvrent l'embarqueur du
manifeste (`connect_embedder`). **Les chiffres du lexical restent justes.**

**Une recherche, médianes sur 50 appels (10 requêtes × 5 tours)** :

| forme | R | A | du lexical (R → A) |
|---|---|---|---|
| sans filtre | 965 ms | **7 032 ms** | 874 → **6 928** ms |
| chemin `src/include/` | 247 ms | 2 154 ms | 150 → 2 041 ms |
| langage python | 229 ms | 1 371 ms | 127 → 1 247 ms |

Les autres étages pèsent peu, et pareil des deux côtés : `resolve` environ 50 ms, le crochet
`links` de 30 à 47 ms, la source 3 ms, la fusion et le rendu près de 0.

**L'intérieur du nœud lexical, sans filtre** (`tests/sonde_noeud_lexical.rs`) :

| étape | médiane |
|---|---|
| 1. les racines (`select_all`) | 19,7 ms |
| 2. le masque (`derived_references`) | 901,8 ms |
| **3. `_uuid IN [72 652 versions]` en offsets (`resolve_filter_to_ids`)** | **5 001,2 ms** |
| 1 + 2 + 3 | 5 922,6 ms, sur les 6 928 du nœud |

Le reste, environ 1 s, est la recherche lucivy. lucivy mesure de son côté 6 à 10 ms de filtre par
requête. **Ce n'est donc pas BM25, mais la résolution de l'`IN` par le moteur.** Le nœud dense refait
le même masque de son côté.

**Le drapeau.**
- La migration prend 18 à 20 ms.
- En B, les racines par la clause puis `resolve_offsets` coûtent 1 198 ms, contre 19,7 ms par
  `select_all` en A : **environ 1,2 s de plus par recherche sans filtre, au-dessus du seuil**.
- La forme de repli est acquise (c7) : l'ensemble par (couche, rang), posé une fois.

**L'arrêt de la passe** : le noyau a tué la sonde à 18 h 03 min 49 s, pendant B. Elle tenait
anon-rss 33,1 Gio au plafond `POSTE_MEM_MAX=32G`, pour une base de 4 Go, après environ 250 appels.
R avait passé ses 150 appels. La pente du RSS par appel est mesurée à part.

**Le partage des chantiers** (c7, 14 novembre 2026) :
- **arbre principal** prend le court-circuit de 1a : sans filtre ni vue, pas de masque ; seule reste la
  remontée version → occurrences des résultats retenus. C'est un lot à part, avant 1b, et il ne vaut
  que si aucune version n'est orpheline (`NOT EXISTS`, compté une fois par appel) ;
- **everything declarative** prend la vue publiée par (couche, rang), pour le cas filtré et le cas
  sous vue : elle se pose une fois en offsets et remplace l'étape 3. Il prend aussi la fuite de RSS ;
- **lucivy** prend la vue côté index (`define_view`), mais seulement si ses 6 à 10 ms comptent une
  fois les 5,9 s retirées ;
- **banc** tient le juge de parité (`tests/sonde_parite_search_code.rs`) : au bit près entre la base
  migrée et chaque optimisation, sur l'ensemble des 10 premiers entre R et la base migrée ;
- **un ticket moteur** pour la résolution de l'`IN` (5 s pour 72 652 clés primaires).

**Le point de composition avec le lot d'arbre principal.**
- Sous 2a′-1 tel qu'il a atterri, une édition ou une suppression **efface** encore ses `Scope` (la
  fin d'instantané, puis la file de suppressions). Les versions `ScopeContent` survivent : les
  orphelines existent sur une base vivante.
- 2a′-2 ferme cela : retirer, c'est poser `_jusqu_a`, et la ligne garde son arête.
- Quand le drapeau tombe :
  - le test du court-circuit devient « aucune version sans occurrence **visible** au rang lu » ;
  - la projection version → occurrences ne garde que les occurrences visibles.
- Le témoin d'arbre principal, « une orpheline ne fait jamais perdre un résultat », se joue sous
  2a′-2, avec le cas d'une version dont toutes les occurrences sont retirées.

## 3. La suite de 2a′-2 (page 03, § 7)

Dans l'ordre accepté par c7 :
- la mesure du § 2, et ses chiffres à c7 avant le drapeau ;
- `outline` dans le témoin de non-fuite ;
- l'écrivain d'une seconde génération, qui appelle `marquer_des_lignes_invisibles`. Le
  `#[allow(dead_code)]` de la méthode est déjà parti (3a7cf136c) : publique pour la sonde, elle n'est
  plus du code mort ;
- les arêtes et la forme C du § 6 de la page 03.

Chaque témoin est vu rouge d'abord.
