# Cadrage : le plein texte en fichiers et le point de reprise du moteur

Demandé par c7 le 20 novembre, pour le ticket `docs/tickets/2026-10-10-index-plein-texte-en-fichiers-hors-du-point-de-reprise.md`
(décision de Lucie du 10 octobre : « on fera plutôt comme Neo4j pour l'instant »). Docs seules : pas de code avant que c7
l'ait lue. À relire par le cœur (le point de reprise) et l'arbre principal (l'état d'index). Lu sur origin/master du
20 novembre ; lucivy aux versions du Cargo.lock (`lucivy-core` et `ld-lucivy` 4.3.12).

## Ce qui existe déjà : un protocole de génération côté produit

Le ticket du 10 octobre décrit l'index comme « réparé après coup ». C'est vrai, mais la réparation repose déjà sur un
lien solide avec la base :

- **Où** : un dossier par table qui porte l'index et par cellule, `<base>.fts/Lucivy_<Table>[__org__project]/`, avec
  4 shards (`catalog.rs:1155, 1777-1783`, `fts_handle.rs:38-43, 353`).
- **Le lien** : un numéro de génération entier, des deux côtés.
  - Dans le dossier, le fichier `_generation`, écrit par `.tmp`, `sync_data`, `rename`, puis fsync du dossier
    (`fts_directory.rs:249-286`).
  - Dans la base, la clé méta `fts_generation:<index>` (`catalog.rs:9753-9756`).
  - `Catalog::flush_blob_store` les pose dans cet ordre : fsync des fichiers lucivy, `_generation` = N+1, puis la clé
    en base (`catalog.rs:9730-9795`).
- **Le paquet de la synchronisation de code** (le défaut, `code_sync.rs:2274-2379`) fait tout cela **dans** la
  transaction du moteur, avant son COMMIT. La marque en base est donc atomique avec les lignes, par le journal du
  moteur. Le **lot L3** (`catalog.rs:8756-8956`) fait l'inverse : COMMIT du moteur, puis lucivy, puis la marque, hors
  transaction. Une erreur après le COMMIT y rend `MustReopen`.
- **Les gardes** :
  - la génération à l'ouverture (`catalog.rs:1779-1828`) : un écart entre le dossier et la base, ou un `fts_pending:`
    encore posé, fait jeter le dossier et le rebâtir depuis les lignes ;
  - la promesse orpheline (`fts_promesse`, `catalog.rs:7256-7291`) ;
  - les comptes (`check_fts_counts`, `catalog.rs:1302-1355`).
- **Les témoins existent** : `tests/e2e_tx_par_paquet_arret.rs`, cas 810, 841, 860, 887 et 937. Une mort entre les
  fichiers et la validation y est rebâtie aux bons comptes, et une base saine se rouvre sans rebâtir.

**Ce qui manque n'est donc pas la cohérence, mais son prix.** Sur tout écart, la réparation est un rebâti **entier**
depuis les lignes : le dossier est jeté, puis repeuplé par lots de 2 000, `FTS_REBUILD_BATCH`. Pendant ce temps, la
recherche répond « mots : en cours ». La cause : lucivy ne garde qu'une génération sur disque. Après chaque commit, il
efface ce que le nouveau `meta.json` ne référence plus (`garbage_collect_files`, `segment_updater_actor.rs:460`), et
`IndexWriter::rollback` ne revient qu'au dernier commit. rag3weaver le dit lui-même : « on ne garde pas la génération
N−1 pour y revenir, on rebâtit » (`fts_directory.rs:24-25`). Un ROLLBACK de paquet coûte en plus une réouverture du
catalogue (`catalog.poison`, `code_sync.rs:2381`).

Deux écarts au ticket, à corriger quand il sera repris : son « cas tués d'`e2e_arret_brutal` » n'existe pas (ce test
porte sur l'index vectoriel) ; les cas d'arrêt du plein texte sont dans `e2e_tx_par_paquet_arret.rs`.

## Deux voies

### A. Ancrer l'index au point de reprise du moteur (la forme Neo4j)

**Ce qu'elle demande au moteur** :
- le point de reprise connaît les dossiers d'index qu'une extension déclare ;
- dans `checkpointStorage`, il demande à l'extension de sceller une génération (un commit lucivy, sans effacer
  l'ancienne) ;
- il inscrit le numéro de cette génération dans le catalogue, publié avec le reste : par l'enregistrement CHECKPOINT
  au journal, suivi de l'application des pages ombres (l'en-tête s'écrit par elles) ;
- il laisse l'extension retirer l'ancienne génération après la remise à zéro du journal.

Le crochet existe : `Index::checkpoint`, que le R-tree de geo emploie (`extension/geo/src/index/rtree_index.cpp:144`).
Mais il réécrit son fichier en place, sans fsync ni renommage : c'est exactement le défaut à éviter.

**Ce qu'elle ne règle pas seule** : après une mort, le rejeu du journal ramène les lignes validées **après** le point
de reprise, mais rien ne ramène l'index au même point. Le moteur ne connaît pas les écritures de lucivy, que rag3weaver
fait lui-même. Pour que le rejeu les refasse, le plein texte devrait devenir un index du moteur, nourri par
`insert`/`delete`/`commitInsert` et journalisé comme HNSW. C'est le chantier que le ticket du 10 octobre écartait (les
blobs au journal à 25 octets par cellule, les points de reprise forcés). Sinon, rag3weaver devrait rejouer lui-même le
delta d'après le point de reprise. **Taille** : grande (un nouveau genre d'index, le format, le rejeu), à cheval sur le
moteur, lucivy et rag3weaver.

### B. Garder deux générations et revenir à celle que la base nomme (dans le produit)

Le lien est déjà là : la clé `fts_generation` en base est atomique avec les lignes du paquet. Il suffit que la
génération que la base nomme existe encore sur disque quand le dossier est en avance :

- **Dans lucivy** : un commit qui garde les fichiers de la génération précédente, sans `garbage_collect_files`
  immédiat, et un « revenir au `meta.json` de la génération N », par shard ; ou bien `PreparedCommit::set_payload` pour
  porter N dans `meta.json`.
- **Dans rag3weaver** : à l'ouverture, si `_generation` = N+1 et la base dit N, revenir à N au lieu de jeter le dossier.
  L'effacement de N−1 attend que la base ait validé N+1. Le ROLLBACK d'un paquet revient de même à N, au lieu
  d'empoisonner le catalogue.
- **Le lot L3** (le moteur valide avant lucivy) : le cas inverse, la base en avance sur le dossier. Il garde son
  `fts_pending:` et le rebâti de ses seules entités, ou bien rejoue son tampon, qui est par `_uuid`.

**Le moteur n'a rien à changer.** **Taille** : moyenne, dans lucivy (garder une génération, y revenir) et dans
rag3weaver (la garde de génération, le ROLLBACK du paquet).

## Ce que je recommande

**B d'abord.** Il garde la forme actuelle, qui est juste, et en retire le coût : plus de rebâti entier après une mort
ou un ROLLBACK de paquet, seulement un retour à la génération que la base nomme. A ne se justifie que si l'on veut que
l'index soit couvert par le **rejeu** du moteur, ce qui revient à en faire un index du moteur ; c'est un autre ordre de
grandeur, et le ticket du 10 octobre avait écarté ce chemin pour le coût du journal. « Comme Neo4j » se lit alors comme
« le produit garantit la cohérence de bout en bout » ; B la garantit sans rebâti.

## Le témoin rouge (pour B)

Partir du cas 810 de `e2e_tx_par_paquet_arret.rs` (SIGKILL au paquet 2, fichiers synchronisés et marqués, lignes non
validées). Aujourd'hui, le repreneur dit « dossier jeté, rebâti depuis les lignes », et c'est vert pour les comptes.
Le témoin de B exige en plus que **le dossier ne soit pas jeté** : le repreneur revient à la génération N, sans rebâti,
mesuré par l'absence du message et par le nombre de documents relus. **Rouge aujourd'hui**, puisqu'il rebâtit. Un
second cas : un ROLLBACK de paquet, sans mort, qui doit laisser l'index à N sans réouverture.

## Les étapes, si B est retenu

1. Lucivy (session lucivy) : garder la génération d'avant et y revenir, par shard, avec son témoin dans lucivy.
2. rag3weaver (arbre principal) : la garde de génération qui revient à N au lieu de jeter ; le ROLLBACK du paquet ;
   l'effacement de N−1 après la validation de N+1 ; le témoin rouge ci-dessus, puis vert.
3. Le moteur : rien. La session hotfixs relit le tout (l'ordre fsync, rename, marque en base) et le banc d'arrêt.

## Relu par le cœur (20 novembre)

L'ordre du point de reprise et ce que couvre le rejeu sont confirmés. Avec A, une mort avant que l'enregistrement CHECKPOINT soit
durable laisse l'ancien état nommer l'ancienne génération, qui doit donc exister encore. Le point qui publie est cet
enregistrement au journal, suivi de l'application des pages ombres, et non l'écriture de l'en-tête elle-même ; l'ancienne
génération vit jusqu'à la remise à zéro du journal. Après le rejeu, l'index reste au point
de reprise, en retard de tout ce que le journal rejoue : A ne suffit pas sans un index du moteur journalisé. Le cœur
recommande B lui aussi, et deux précisions s'ajoutent :

1. **B ne dépend pas du point de reprise V2** : des lecteurs tournent pendant qu'il s'écrit, mais aucun écrivain ne
   valide, et le lien de B passe par la clé `fts_generation`, journalisée avec les lignes.
2. **À l'étape 2, le retrait de N−1 attend que la validation de N+1 soit durable** : le journal synchronisé au COMMIT,
   et pas seulement la transaction validée. C'est le défaut du moteur aujourd'hui ; l'écrire, pour qu'un mode sans fsync
   du journal, s'il en vient un, ne le casse pas.

Pour le lot L3 (le moteur valide avant lucivy), la base est en avance et on ne peut pas revenir en arrière : son
`fts_pending:` et le rebâti de ses seules entités restent.

Question pour c7 et Lucie : B, ou A ?

## Relu par l'arbre principal (20 novembre) : trois corrections à B

L'arbre principal est d'accord avec B et avec l'ordre fichiers → `_generation` → clé en base → COMMIT du paquet. Il
corrige trois points, qui remplacent ce que la voie B disait plus haut.

1. **Garder toute génération au-dessus de la dernière validée, pas « deux générations ».** `flush_blob_store` est appelé
   à chaque frontière, et pas une fois par paquet : `ingest` (`catalog.rs:7121`, à chaque `ingest_entities`) et
   `drain` (`:9626`). Sous la transaction par paquet de la synchronisation, `fts_tampon` n'est pas posé (seul L3 le
   pose) : un paquet avance donc `_generation` plusieurs fois, N+1, N+2… avant son COMMIT. Après une mort, la base dit
   N et le dossier peut être à N+k. La règle : garder sur disque toute génération ≥ celle que la base a validée en
   dernier, et n'effacer qu'en dessous, après un COMMIT durable. **Le témoin (cas 810) doit forcer k ≥ 2** (deux
   ingestions dans le paquet tué) : une lucivy qui ne garderait que deux générations passerait sinon un témoin à k = 1
   et casserait en vrai. Le k réel d'un paquet se lit dans `ingest_profile` (la ligne « rendre durable le plein texte
   (fichiers) », par paquet).
2. **Revenir à N retire le rebâti, pas la réouverture après un ROLLBACK.** `Catalog::poison` et `terminer` nomment
   d'autres caches qui gardent des écritures défaites (identifiants de nœuds, modèles enregistrés, file vidée, magasin
   de blobs préparé) ; le plein texte n'est qu'un des motifs. Le second cas du témoin devient donc « ROLLBACK, puis
   réouverture **sans rebâti** » ; un ROLLBACK sans réouverture serait un autre lot, pour les autres caches.
3. **L'unité du retour est le dossier de l'index, ses quatre shards ensemble.** `_generation` est par dossier,
   `meta.json` par shard. Revenir à N, c'est revenir à N sur les quatre, ou ne rien faire : un retour coupé entre deux
   shards retombe sur le rebâti d'aujourd'hui, `fts_pending:` posé d'abord (comme la garde le fait, `:1810-1818`), et
   `_generation` ne se réécrit à N qu'**après** les quatre retours, par le même `.tmp` → `sync_data` → `rename`.

**Pour l'étape 1, à demander à lucivy** : qu'un **lecteur** (`lecture_seule`, `:1797`) qui trouve le dossier à N+k et
la base à N puisse **servir N en lecture**, en ouvrant le `meta.json` d'une génération nommée sans toucher au dossier.
Ce serait mieux qu'« à rebâtir par un écrivain ».

**Le lot L3, vérifié** (la question de l'arbre). La marque qui le protège n'est pas `fts_pending:` mais la **promesse**.
Le premier verbe d'écriture d'une génération passe par `check_ecriture` (`catalog.rs:7230-7235`), qui écrit
`fts_promesse` (le jeton du processus, `promettre_le_plein_texte`, `:7241-7254`). Dans un lot, cette écriture tombe dans
la transaction ouverte par `commencer_lot_atomique` (`:8774-8780`) et part avec son COMMIT. Une mort entre ce COMMIT et
lucivy (`commit_lot_after_native`, `:8844-8878`) laisse donc en base une promesse d'un autre jeton :
`reprendre_une_promesse` la trouve à la réouverture et marque toutes les entités au plein texte à rebâtir. L'écart est vu.
Réserve : lu, pas joué, et pas vérifié pour chaque chemin d'écriture du lot ; à confirmer par un témoin (une mort entre le
COMMIT du lot et `appliquer_le_tampon`, puis la réouverture qui rebâtit) avant de s'y fier. S'il rougissait, ce serait un
défaut d'aujourd'hui, avec ou sans B, et un ticket à part.

**Qui fait quoi.** L'arbre principal prend l'étape 2 quand c7 aura dit B et que l'étape 1 de lucivy aura posé, rouge
d'abord, avec le témoin à k ≥ 2.
