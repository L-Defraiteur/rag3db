# Des morceaux de code sont écrits sans texte : 20 % de code-rag3db

- **État** : corrigé côté moteur (le commit qui déplace ce fichier dans `closed/`, 12 octobre 2026, cœur C++) — la lecture, pas le stockage : `StringColumn::scanFiltered` (chemin pris quand un groupe porte des versions — lignes supprimées ou écrites sous transaction) bornait la fin d'une fenêtre de 2 048 lignes par `startOffsetInChunk + pos < numValues` au lieu de `pos - offsetInResult < numValuesToScan` : quand la fenêtre chevauche deux segments de la colonne et que le second est plus court que la position, ces lignes n'étaient pas écrites dans le vecteur, qui gardait la chaîne de la fenêtre d'avant (ou rien). Venu de l'amont (« Segmentation », 18 septembre 2025). Même borne dans `ListColumn::scanFiltered`. Les données sur disque sont intactes : la relecture par clé (lookup) était juste ; les morceaux « sans texte » ont leur texte sur disque, c'est le balayage qui ne le rendait pas (vide, ou le texte d'un autre morceau 2 048 lignes plus tôt, sans aucun signe). L'arbre principal note les suites côté produit (la réécriture des vides de 7b54c5f84 reste sans danger). Témoin du banc dans le lot
- **Gravité** : réponse fausse (une recherche ou un usage ne voit pas le passage, sans le dire jusqu'aux défenses de `40f24a50b`) ; et blocage (la passe de vecteurs bouclait dessus, le gel de code-lourd)
- **Atteignable en service** : oui, tout index de code
- **Touche rag3weaver** : oui (l'écriture des morceaux, à trouver)

## Ce que c'est

Sur une copie de l'index `code-rag3db-depot` (le dépôt rag3db, prise à 20 h 24, lib
`e71c85b551cf`), comptée par F (`e2e_mesure_index_entier::les_morceaux_a_texte_vide`) :
**26 092 `Scope_Chunk` sur 128 732 ont `_text` nul ou vide, soit 20 %**. Ce n'est pas quelques
lignes abîmées : il y en a des centaines par gros fichier (`lucivy_core/src/sharded_handle.rs` 301,
`tree-sitter-scss/src/parser.c` 211, `luciole/src/runtime.rs` 185, `rag3weaver/src/catalog.rs` 141,
`dialect.rs` 94…).

**Le texte a existé** : les 26 092 portent tous un `_text_hash` qui est celui d'un vrai texte,
aucun celui d'une chaîne vide (`e2e_mesure_index_entier::les_morceaux_vides_ont_le_hash_d_un_texte`).
Le texte est donc perdu entre le hachage et l'écriture en base.

Ni codeparsers ni le découpage ne créent un morceau vide (relu par codeparsers : un champ vide est
sauté, le découpeur ne rend aucun morceau blanc ; record_nodes.rs:1450-1496, chunker.rs:110-121).

## Ce qu'il a coûté

- **Le gel de code-lourd** (luciepc, quatre fois, autour de 18-20 %) : l'embarquement sautait un
  morceau sans texte sans le marquer, la réclamation le reprenait à chaque passe ; quand les
  morceaux dus qui restaient étaient tous vides, `run_index` bouclait sans fin (WAL qui grossit,
  aucun appel au service). Défenses dans `40f24a50b` : un morceau sans texte n'est jamais dû, une
  passe sans progrès s'arrête nommée.
- **Des faux négatifs** : ces passages ne sont ni cherchables par leur texte ni embarqués. Depuis
  `40f24a50b`, l'index le dit (journal de fin, `index_states.chunks_without_text`, en-tête des
  lectures).

## Pistes, à prouver

- Le chargement en masse (CSV, `cellule_csv`, record_nodes.rs:695-729) : un `\` non mis entre
  guillemets sous `escaped_newlines=true` (relevé par codeparsers). Le compte par gros fichier va
  dans ce sens ; la comparaison des `\` faite sur les fichiers ne vaut rien (les décalages des
  morceaux portent sur le texte du scope, pas du fichier).
- Témoin en cours (codeparsers) : `sharded_handle.rs` seul, par le chemin de masse et par
  `ingest_code`, sur deux bases neuves ; le compte des vides de chaque côté.

## Ce qu'il faut pour le fermer

La cause prouvée par un témoin rouge (un fichier passé en masse rend des morceaux sans texte), le
correctif, et **la réécriture des morceaux touchés** : la montre ne les reverra pas (le fichier n'a
pas changé) ; l'index doit lancer de lui-même la réécriture des morceaux sans texte quand leur
compte est non nul. La question de fond, gardée pour Lucie : un morceau sans texte doit-il exister
(un scope sans embarquement mais avec ses relations) ou ne jamais naître.

## Ce qui ne reproduit pas (sondes de codeparsers, 11 octobre au soir)

Sonde `sonde_morceaux_vides.rs` (worktree `rag3db-vide`, non commitée ; bases gardées sous
`~/.cache/rag3weaver-build/sonde-vides-*`), lib du même bâti que l'hôte de luciepc
(`rag3db-build-6df47a543…`). Colonnes comptées : `Scope_Chunk`, `_text` nul, `_text` vide, vides
avec hash. **Aucun vide, nulle part** :

- `sharded_handle.rs` seul, en masse (`sync_source`) puis paquet par paquet (`ingest_code`) : 351 morceaux ;
- base mémoire : `lucivy_core` (1 757), `ld-lucivy` (28 703), le dépôt entier (130 228) ;
- base disque, fermeture et réouverture : `ld-lucivy` sans vecteurs puis avec vecteurs (embarqués
  pendant l'ingestion), le dépôt entier sans vecteurs ;
- une masse arrêtée à 320 fichiers sur 1 147 puis reprise, avec réouvertures ;
- `edit_file` sur `sharded_handle.rs` (ligne remplacée à longueur égale, ligne insérée en tête), puis
  resynchronisation de la source ;
- la passe de rattrapage (`embarquer_le_retard` en boucle, 57 passes, puis réouverture) et la
  réclamation seule répétée mille fois sur les mêmes lignes.

Une passe propre ne perd donc rien : ni la masse, ni le SET de réclamation, ni celui du vecteur, ni
le point de reprise. Restent hors des sondes : **l'hôte lui-même** (sessions concurrentes, montre
et réactions, rebâti du plein texte à l'ouverture après une génération interrompue), **un vrai
service d'embarquement lent**, **les arrêts brutaux** et le rejeu du journal, et **la copie prise au
vol** (celle que F a comptée a été prise pendant que l'hôte tenait la base). La sonde par l'hôte,
avec un faux service lent et une session qui interroge pendant l'index, est la suivante (codeparsers,
12 octobre au matin).

La réclamation seule reproduit en revanche **la boucle** : sans le service, elle rend sans fin ses
propres lignes (512 000 en 8,7 s) — ce que les défenses de l'arbre principal arrêtent.

## Le chemin réel, par l'hôte, est sain (12 octobre, nuit)

Codeparsers, avec le témoin de bout en bout de l'orchestration : binaire release de master
`8610eaec6` contre la lib du poste (`6df47a54…`), un hôte à part, `code-tree add` d'un worktree de
rag3db sur une base neuve, l'outil `index` par une session MCP avec le vrai service `granite-278m`
(régime doux, 1 369,5 s). Pendant l'index, la session a lu (`search_code`, `usages`) et renommé un
symbole sur disque, et la montre a réindexé. Compté ensuite, base fermée : **`Scope_Chunk` 102 469,
`_text` nul 0, vide 0 ; `File_Chunk` 6 303, vides 0.**

Le chemin d'aujourd'hui ne crée donc pas de morceau vide en une histoire propre. Les vides de la
copie de F et de code-lourd viennent de **leur histoire** : des binaires plus anciens (code-lourd a
été indexé par `a3c1ca98f`, puis par le binaire suivant), des `kill -9`, et une copie prise au vol.
La copie de code-lourd que Lucie prend au réveil, base fermée, dira si ses vides ont un parent et un
hash ; l'outil de comptage est la sonde de codeparsers (`compter_une_base`), versée au ticket.

Ce qui reste à faire pour fermer : établir l'origine sur cette copie ; si c'est un défaut d'un
binaire ancien déjà corrigé, le dire ici ; dans tous les cas, **la réécriture des morceaux sans
texte** d'une base qui en a (l'index la lance quand le compte est non nul), puisque la montre ne les
reverra pas.

### À départager au matin (orchestration, 12 octobre, nuit)

Les 26 092 vides de la copie de F viennent d'une base indexée par un autre binaire, ou par un autre
chemin que l'index entier (synchronisation, réédition par la montre, réindexation partielle, reprise
après un arrêt) :

1. **Quel commit du binaire a indexé `code-rag3db` vers 15 h le 11 octobre** (le binaire de Lucie
   d'alors), et ce qui a changé depuis dans l'écriture des morceaux.
2. **Les fichiers touchés** (`sharded_handle.rs` 301 vides…) **ont-ils été réédités par la montre**
   entre 15 h et 20 h 24 (heure de la copie), ou sont-ils seulement les plus gros ?
3. **Sur sa copie, F** : les morceaux vides ont-ils des `_embed_*`, des marqueurs ou des dates qui les
   distinguent des pleins (écrits par un autre passage) ?

### Ce que lit la copie de F, et la relecture du chemin de réingestion (12 octobre, nuit)

**La copie (F, `ce_qui_distingue_les_morceaux_vides`)** : la perte se fait **par scope entier**
(14 724 scopes tout vides, 65 442 tout pleins, 1 232 mêlés). Chez les vides, tout le reste est
présent à 100 % : `_text_hash`, `_embed_hash`, `_sparse_hash`, `_embed_claim`, le vecteur
`granite_278m` et son marqueur, les bornes. Le vecteur a donc été calculé depuis un vrai texte, et le
texte a disparu ensuite. Les scopes vides sont plutôt petits et en tête de fichier (rang moyen 4,7,
`_start_line` moyen 133).

**Le binaire de la fenêtre** (la mémoire) : de 17 h 50 à 20 h 35, l'hôte de Lucie servait le
binaire de `e60f2342e` contre la lib rebâtie à 17 h 39 sur **le même commit** (`9f9d9409…`) ; la
montre de `code-rag3db` tournait depuis 17 h 10.

**La relecture du proto** (aucune écriture applicative ne vide `_text`) : trois chemins qui n'existent
qu'en réingestion, jamais à un index neuf (qui passe par COPY) —
1. l'upsert `UNWIND … MERGE … SET n.c = item.c` sur des lignes de morceaux existantes (uuids
   déterministes : un scope modifié repart en entier) ; le texte est envoyé, une perte y serait un
   défaut du moteur au SET d'une chaîne sur une ligne existante ;
2. le remplacement du vecteur par le contournement HNSW (`embed_clear` puis `embed_set`), deux SET
   de plus sur la ligne ;
3. **pas de dédoublonnage par uuid sur le chemin d'upsert** (le COPY en a un) : deux scopes de même
   clé dans un fichier donnent les mêmes uuids de morceaux dans un même lot.

Et ce qui fait **durer** le vide : le critère « complet » de `split_unchanged` compte les morceaux,
les marqueurs et les arêtes, pas `_text` ; un scope vidé est jugé complet et toute réingestion du
même contenu le saute.

**Départager (1)/(2)** : sur un vide, le marqueur dense égale-t-il `_text_hash` ? (demandé à F).
**Témoins au matin** : la réédition avec vecteurs, répétée (codeparsers) ; le témoin du proto
`a_reingested_file_keeps_the_text_of_its_chunks` (réingestion par le chemin de la montre, décalages,
deux scopes de même clé) ; le couple `e60f2342e` si les deux restent verts.

**Réparation** : `dd7bb2c1e` (en file) réécrit les morceaux sans texte à l'index, en passant outre
`split_unchanged` ; ajouter « `_text` non vide » à son critère, pour qu'une réingestion de la montre
répare aussi, suit en lot court.

### La réédition n'est pas soutenue ; ce qui sépare, c'est le dépôt (F, 12 octobre, nuit)

- **Croisé avec git** (les fichiers touchés par un commit entre le premier index, 15 h 21, et la
  copie, 20 h 24 : 225 chemins dont 177 indexés) : ils portent des scopes tout vides **moins** souvent
  que les autres (22,6 % contre 30,0 % des fichiers ; 12,5 % contre 18,6 % des scopes). La piste de la
  réédition par la montre n'est pas soutenue — réserve : une édition sans commit ou un checkout de
  sous-module y échappe.
- **Par dépôt** (`File.revision`) : le sous-module `ld-lucivy` a **46 %** de ses scopes tout vides, un
  autre dépôt (`4c7897c8…`, à identifier) 21 %, le dépôt principal **9 %**. **Par langage** : cypher
  89 %, json 44 %, markdown 38 %, python 34 %, rust 32 %, cpp **5 %**. Un seul `_snapshot` pour 7 306
  fichiers (une seule synchronisation) ; les 43 fichiers sans snapshot n'ont aucun vide.
- **Le contenu n'est pas en cause** : `Scope.content` n'est jamais vide ; guillemets, barres inverses
  et longueurs ne distinguent pas les vides ; l'accent grave est en excès chez eux (22 % contre
  8,8 %) mais absent de 78 % d'entre eux.
- **Question ouverte (F)** : l'ordre des lots du chemin de masse — les sous-modules et les petits
  fichiers hors C++ partent-ils dans un même lot, ou un même COPY ?
- **En route** : un SIGSEGV du moteur sur une lecture de ~80 000 scopes à texte long, signalé au cœur
  par F.

Tables par fichier : `~/.cache/rag3weaver-build/embarquements/fichiers-vides.tsv` ; tests :
`e2e_mesure_index_entier`.

### Deux lectures de plus (F, `deux_lectures_pour_departager_les_chemins`)

- **Le marqueur dense (`_embed_hash__granite_278m`) égale `_text_hash` chez 97,3 % des vides**
  (98,7 % des pleins) : la ligne a été embarquée après la dernière écriture de son texte, texte en
  main. **Le texte a été perdu après l'embarquement.** (`_embed_hash` et `_sparse_hash` n'égalent
  `_text_hash` chez personne : ils ne portent pas la même chose.)
- **Les scopes de même clé ne sont pas en excès** chez les vides (13,9 % contre 15,5 %) : la piste du
  doublon d'uuid dans un même lot d'upsert tombe.

Restent, parmi les chemins de la relecture : **l'upsert d'une réingestion** et **le remplacement du
vecteur** (contournement HNSW), tous deux sur une ligne déjà embarquée. Mais la répartition (par dépôt,
par langage, sans excès chez les fichiers réédités et commités) ne les désigne pas d'elle-même. **Rien
n'est conclu** : la sonde de la réédition avec vecteurs, répétée (codeparsers, au matin), et le témoin
du proto le diront ; sinon le couple `e60f2342e`.

### Correction : « perdu après l'embarquement » ne suit pas du marqueur (arbre principal)

La note précédente concluait que le texte avait été perdu après l'embarquement. **Ce n'est pas
établi** : dans une ingestion qui embarque en ligne, `EmbedNode` calcule le vecteur depuis le texte
**en mémoire** (le record), pas depuis la base. Un `_text` mal écrit en base au moment de l'ingestion
laisse donc une ligne vide portant un vecteur et un marqueur justes. Les faits de F sont tous
compatibles avec une **écriture fautive du texte à l'ingestion** par le binaire d'alors.

**La lecture de l'orchestration** : « binaire ». La base a été indexée vers 15 h le 11 octobre par le
binaire de Lucie d'alors, d'avant l'échappement systématique et la conversion du Cypher brut de
l'indexation en IR (F, 17 h-19 h). Un texte à accents graves, guillemets, `$` ou barres inverses écrit
par le Cypher brut est le suspect ; cypher et markdown en sont pleins (89 % et 38 % de scopes vides).
L'index neuf de cette nuit, par le binaire d'aujourd'hui, donne 0.

**La sonde décisive (au matin)** : indexer, avec le plus ancien binaire gardé de Lucie
(`~/.local/bin/rag3weaver-backend.avant-*`), en autonome et isolé, un petit corpus (dix fichiers
`.cypher` et `.md` de lucivy parmi les plus touchés), compter les vides ; refaire avec le binaire
d'aujourd'hui. L'ancien rouge et le neuf vert : la cause est datée, fermée par la conversion de F ; le
ticket se ferme par la réécriture des morceaux vides (`dd7bb2c1e`) et par « complet » qui compte
`_text`.

### La perte est par fichier entier, le plus souvent (arbre principal, 12 octobre)

Sur les tables de F (`~/.cache/rag3weaver-build/embarquements/fichiers-vides.tsv` et `.files`),
chaque fichier classé selon ses scopes tout vides :

| dépôt | aucun vide | tout vide | partiel |
|---|---|---|---|
| principal (1bda16ce) | 4 537 | 1 049 | 266 |
| ld-lucivy (e8b54145) | 349 | 447 | 326 |
| codeparsers (4c7897c8) | 91 | 20 | 43 |

Les fichiers les plus touchés sont vides de bout en bout (`lucivy-fst/src/raw/node.rs` : 106 scopes
sur 106). Un échappement fautif d'un caractère toucherait le scope qui le porte, pas tous les scopes
d'un fichier. Une perte d'un fichier entier désigne l'écriture **par fichier** (une instruction ou un
lot par fichier). L'accent grave plus fréquent dans les scopes vides (22 % contre 8,8 %, F) peut
n'en être que le déclencheur au niveau du fichier. Les 266 + 326 fichiers partiels restent à
expliquer.

Corpus de la sonde décisive, dans `~/.cache/rag3weaver-notes-A/sonde-binaire/corpus.txt` : les
dix fichiers ld-lucivy entièrement vides les plus gros, et dix fichiers rust du principal sans
aucun vide.

### Le code de 13 h 20 relu : ni Cypher brut, ni perte à l'écriture (arbre principal, 12 octobre, 2 h 40)

**Ma correction de 2 h (« un vecteur calculé en mémoire n'atteste pas le texte en base ») est
fausse pour ce chemin, et la lecture « binaire » ne tient pas.** Relu à `440ef5826`, master à 13 h 19,
la révision du binaire installé à 13 h 21 (`8bb2af2d7` au plus, qui ne touche pas l'écriture) :

- `_text` est posé en `record_nodes.rs:1480`, avec `_text_hash` en l. 1482, depuis le même
  `chunk.text`. Il est écrit par **COPY CSV** (`copier_les_noeuds`, `pousser_cellule` l. 737,
  `cellule_csv` l. 695, `copy_nodes_from_csv` dans `dialect.rs:1527`). Le repli est `batch_upsert`
  en **paramètres liés**. Aucun Cypher brut n'écrit `_text`, et l'accent grave n'a besoin d'être
  échappé ni en CSV ni en paramètre.
- Ce chemin est **identique aujourd'hui**. Les lots 1 à 4 de la conversion en IR (`68dbcfc4b`,
  `da02a612e`, `bcea5ea9e`, `c8fae292a`) traduisent des lectures, `Write::Detach` (des arêtes) et
  des réglages ; aucun n'écrit de texte.
- `run_index` exige `RECHERCHE_TEXTE` : les vecteurs viennent **après**, par `reclamer_le_retard`,
  qui **relit `n._text` en base** (`dialect.rs:1828-1837`). Un marqueur égal à `_text_hash`, c'est
  97 % des vides : **le texte était en base quand la passe de vecteurs l'a réclamé.** La perte
  vient donc après, au SET de réclamation (`SET n._embed_claim`), à `embed_set`
  (`SET n.vec = …, n.marker = …`), à `embed_clear`, ou au point de reprise et au rejeu qui suivent.

Cela s'accorde avec ce qu'on sait : le COPY range les morceaux fichier par fichier, et une perte
**par groupe de nœuds** du moteur (2 048 lignes) toucherait des fichiers entiers, et partiellement
ceux qui chevauchent une frontière. Elle suivrait aussi l'ordre des lots, donc le dépôt et le
langage. L'excès d'accent grave n'a pas de mécanisme dans le code lu : sans doute le langage.

Une autre lecture tient aussi : une lib d'un autre bâti chargée par le binaire de 13 h 20, ce que
`f3d2290ad` interdit depuis 00 h 11. La sonde du binaire neuf (0 vide sur 1 989 `Scope_Chunk`, là où
dix fichiers étaient tout vides chez Lucie) ne départage pas : elle n'a ni le volume ni la
passe de vecteurs sur une base de plusieurs groupes.

**Le test qui départage** : dans la copie de F, `offset(n)` des `Scope_Chunk` vides contre les
frontières de 2 048. Si les vides forment des plages alignées, c'est le moteur (un SET ou un rejeu
qui perd une colonne chaîne par groupe) : ticket moteur, banc. Si rien n'est aligné, on revient à la
lib d'un autre bâti.

### Les vides sont les 32 768 premières lignes de la table : un défaut du moteur (12 octobre, 2 h 55)

Test de F (`e2e_mesure_index_entier::les_morceaux_vides_et_les_groupes_de_noeuds`), sur sa copie de
code-rag3db-depot, lib 116acb252ace : **les 26 092 morceaux vides sont tous à des offsets de
`Scope_Chunk` entre 0 et 32 767**, exactement 2^15 lignes. Les groupes de 2 048 qui suivent, du 16
au 63, n'en ont aucun.

| offset / 2048 | vides / lignes |
|---|---|
| 0 à 9 | 1 664 à 1 783 sur ~2 040 (82 à 87 %) |
| 10 à 15 | 1 496 à 1 547 sur ~2 040 (73 à 76 %) |
| 16 à 63 | 0 |

Aucun groupe n'est entièrement vide, mais la frontière est nette, et c'est une puissance de deux.
Les premières lignes écrites dans la table ont perdu leur texte aux trois quarts, les suivantes
non. Cela explique tout le reste : la perte par fichier entier (le COPY écrit fichier par fichier),
l'excès de ld-lucivy (indexé dans les premiers lots), et le texte présent au moment du calcul du
vecteur.

**Le défaut est dans le moteur, plus dans le produit.** Une colonne chaîne des 2^15 premières
lignes d'une table de nœuds se vide, sans doute à une écriture qui suit le COPY de masse : les SET
de la passe de vecteurs (`_embed_claim`, `embed_set` qui pose le vecteur et le marqueur), le point
de reprise, ou le rejeu du WAL après les arrêts brutaux du gel de cet après-midi. Constantes du
bâti : `v11` (2 048 lignes par vecteur), `g17` (groupe de nœuds de 2^17). 2^15 n'est ni l'une ni
l'autre ; c'est à chercher dans le stockage des colonnes chaînes (segments, dictionnaire).

**Reproduction proposée (banc)** :
1. une table de nœuds avec une colonne chaîne de quelques centaines d'octets et une colonne
   vecteur ;
2. un COPY de plus de 32 768 lignes ;
3. des SET par lots `UNWIND … MATCH … SET n.claim, puis n.vec, n.marker` sur toutes les lignes ;
4. un point de reprise, puis la même chose avec un arrêt brutal et un rejeu ;
5. compter `_text = ''` par tranche de 2 048 offsets.

Côté produit, la réécriture des morceaux vides (`7b54c5f84`) répare les bases touchées.

## La forme minimale, sans produit (12 octobre 2026, 3 h 26)

Une table `T(id INT64 PRIMARY KEY, s STRING)`, 6 000 lignes de chaînes de 200 octets (`repeat('x',
200) + id`), `CHECKPOINT` : la colonne se découpe en 5 segments de ~1 250 lignes (`CALL
storage_info('T')` ; les segments viennent de la taille, ≤ min(taille/2, 256 Ko), pas du nombre de
COPY). Une seule ligne supprimée dans la fenêtre lue (`id = 10`), puis `MATCH (n:T) RETURN n.id,
n.s` : sans correctif, **les lignes 1250 à 2047 de la fenêtre sont lues vides** (0 octet) — le
« vide » de ce ticket ; une suppression à `id = 2100` fait rendre aux lignes 3292… de la fenêtre
suivante la chaîne d'une autre ligne. Avec le correctif : 0 faux. Chaînes trop courtes (un seul
segment) ou aucune version dans la fenêtre : vert des deux côtés — c'est pourquoi un index neuf
n'a rien.

