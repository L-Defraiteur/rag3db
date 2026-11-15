# Le juge de parité de search_code (14 novembre 2026)

Le lot 2a′-2 optimise la recherche sur une base à versions : un `search_code` sans filtre y
coûtait environ 6,8 s. Le juge dit que **les réponses ne changent pas**. Il le dit entre la base
d'avant les versions et la base migrée, puis entre chaque optimisation et le code d'avant elle.

## 1. Le juge

- **Le relevé** (`extension/rag3weaver/tests/sonde_parite_search_code.rs`, branche
  `banc-parite-search-code`) :
  - un test `#[ignore]` ouvre une **copie** de la base, jamais la base en service, et joue le jeu
    commun ;
  - il écrit, pour chaque requête et chaque forme, les dix premiers résultats : `_uuid`, chemin,
    nom, ligne, hash du contenu, score, signal ;
  - il écrit aussi tous les `_uuid` de Scope de la base.
- **Le comparateur** (`extension/rag3weaver/scripts/parite_search_code.py`) nomme chaque écart,
  dans l'un de deux régimes :
  - **optimisation** : la même base, avant et après. Au bit près : même `_uuid`, même ordre, même
    score, à chaque rang ;
  - **migration** : la base d'avant contre la base migrée. Il juge l'ensemble des dix premiers
    (0,9 au moins) et le rang (3 au plus), sans exiger l'égalité des scores.
- **Le jeu commun**, convenu avec everything declarative
  (`extension/rag3weaver/tests/donnees/jeu-recherche-2a2.json`) :
  - 5 noms, les plus consommés, et 5 phrases, tirés une fois sur la base d'avant les versions et
    figés ;
  - trois formes : sans filtre, filtrée par `src/include/`, filtrée par `python` ;
  - k = 10, soit 30 cas.
- **Les bases** : à luciepc, chaque passe prend une copie reflink neuve de la base migrée par 1b
  (`1b-copie/migree-20261114-1641`). Cette base est la preuve de 1b : on la copie, on ne l'ouvre
  jamais.
- **Les commandes** : `parite-jeu.sh` (R contre la base migrée) et
  `parite-opti.sh <nom> <parent> <optimisation>`, dans `~/.cache/rag3weaver-build/` à luciepc.
  Les relevés sont dans `banc-parite/sorties/`.

## 2. La base d'avant les versions contre la base migrée

- **La clé tient** : les 79 662 `_uuid` de Scope sont communs aux deux bases, sans un manquant
  d'un côté ni de l'autre. Le hash du contenu est le même sur les 229 `_uuid` communs aux relevés.
- **Les dix premiers** : 248 sur 289 sont communs (86 %), et 12 cas sur 30 sont sous les
  tolérances.
- **La cause** : tous les meilleurs signaux sont lexicaux, des deux côtés. 40 des 41 entrées sont
  des contenus neufs, et une seule est une autre occurrence du même contenu. C'est le BM25 sur le
  contenu dédoublonné (72 652 versions pour 79 662 occurrences) qui bouge. Le signal dense absent
  de R ne joue pas, et le choix de l'occurrence représentante non plus.
- **La décision d'arbre principal** : cet écart est accepté comme le prix des versions, comme le
  disait déjà le plan de 1b du 12 novembre. Le relevé de la base migrée (`migree.json`) devient la
  **ligne de base de la forme à versions**.
- **Un fait voisin, en ticket** pour arbre principal
  (`docs/arbre-principal/tickets/2026-11-14-les-dix-premiers-de-search-code-rendent-plusieurs-copies-d-un-meme-contenu.md`) :
  une copie d'un contenu déjà rendu prend 16 places des dix premiers sur R, et 29 sur la base
  migrée.

## 3. Les optimisations, au bit près

| optimisation | commit contre son parent | verdict | temps d'un appel, parent → lot (indicatif) |
|---|---|---|---|
| le court-circuit sans filtre (arbre principal) | `masque-sans-filtre-pile-2` e4576c514 contre 4dd64c199 | **VERT**, 30/30 | (pas relevé à cette passe) |
| le masque en prédicat (everything declarative) | `vue-publiee` 959d73513 contre 3e3a07f9d | **VERT**, 30/30 | lot : sans filtre 1 021, chemin 1 421, langage 1 081 ms |
| le filtre `_uuid` par clé (recherche) | `filtre-in-pile` e0aaa2d1f contre 08aa781db | **VERT**, 30/30 | sans filtre 623 → 624, chemin 2 272 → 1 176, langage 1 508 → 910 ms |

- **Le régime** : chaque parent porte le code d'avant l'optimisation, et chaque parent est aussi
  identique au bit près à la ligne de base de la forme à versions.
- **Les générations** : sur ce jeu, le passage aux générations de la branche d'everything
  declarative ne change aucune réponse.
- **Les temps** sont ceux d'un seul appel, sans chauffe : une indication, pas une mesure. Les
  étages se mesurent par la sonde de la recherche d'everything declarative.
- **Deux ajustements du juge**, sans rien commiter dans les branches jugées et sans aucun
  comportement changé :
  - le passage aux générations est retiré là où le code ne le porte pas ;
  - le code mort est toléré, pour un parent qui porte une fonction pas encore appelée, sous
    `warnings = "deny"`.

## 4. Le lot des orphelins de recherche (832a5896d contre 9d90edbb6)

- **Le montage** (`parite-orphelins.sh`, à luciepc) :
  - une copie neuve de la base migrée ;
  - une source à moi, un worktree détaché au commit de l'instantané (835d5298f) ;
  - les comptes du plein texte, à l'ouverture, après un index de la source entière, puis après un
    index sans `extension/rag3weaver/src/arbre.rs` ;
  - puis les 30 cas de parité sur le lot.
- **Les 30 cas : VERT au bit près.**
- **Les comptes sont les mêmes au parent et au lot** :

  | étape | File (documents / lignes) | Scope | Symbol |
  |---|---|---|---|
  | après l'index de la source entière | 7 811 / 7 608 | 84 211 / 84 211 | 86 172 / 86 172 |
  | après l'index sans `arbre.rs` | 7 608 / 7 607 | 84 211 / 84 211 | 86 172 / 86 172 |

  - L'écart de File est l'orphelin que vise le lot. Recherche confirme que son lot ne le prend pas
    encore.
  - La source au commit 835d5298f diffère de ce que la base avait indexé : 79 662 Scope deviennent
    84 211.
- **Les lignes du fichier retiré, lues une à une** (relevé `orphelins-fichier`, au parent) :
  - après l'index de la source entière, 7 Scope et 1 File portent le chemin de `arbre.rs`, toutes
    avec `_absent_since` nul. Le File est `a94367e3-9f4b-0e62-368a-571d3b0198ab`. Les Scope sont
    `01e86cf0-faac-bd73-ebd2-a371be73bcfa`, `d8ecbcb4-3d88-000e-8747-97c118918764`,
    `44ba6df7-6eeb-80aa-7a05-583cbf329997`, `527abc47-1e3f-3c2b-8862-8f974bc3ef60`,
    `7a8b902b-961d-c725-62d5-8456a63ae989`, `a92f4c45-60cc-fb20-5766-254dd2be98a4` et
    `6547aec7-5499-ac36-ea0d-87cdc20393ae` ;
  - après l'index sans le fichier, aucune ligne ne reste : ni Scope, ni File, ni ouverte, ni
    fermée ;
  - le fichier remis, son File revient.
- **Ce que ça dit** :
  - le parent et le lot sont **dans la forme à versions sans générations**
    (`code-scope-v1/content`). Les colonnes `_couche`, `_depuis` et `_jusqu_a` n'y existent pas :
    elles viennent avec les générations (2a′, branche d'everything declarative) ;
  - dans cette forme, la fin d'instantané **efface** les lignes que la source n'a plus. Elle les
    met de côté le temps de son annulation (`catalog/synchronisation.rs`) et ne pose
    `_absent_since` que sur une transition de cycle de vie. L'effacement des 7 Scope et du File
    est donc **le comportement voulu**, pas un contournement de la version ;
  - **aucun défaut de version ici** : le compte seul trompait. Le seul écart est le document de
    plein texte de File resté après sa ligne, celui que vise le lot ;
  - **la question reste entière pour les générations** : là, un retrait doit fermer la ligne
    (`_jusqu_a`) au lieu de l'effacer. Ce relevé ne le dit pas, faute de générations dans ce code.
    Il se rejouera sur la branche de 2a′ quand le lot des orphelins y sera posé.
- **Les Symbol, par la relation** (`Symbol` n'a pas de chemin ; relevé `orphelins-symboles`) :
  - avant le retrait, 20 Symbol sont reliés aux Scope de `arbre.rs`. Trois ne sont reliés par
    aucun autre fichier (`relation_label` et les deux noms de ses tests) ;
  - après le retrait, ces trois restent, **sans aucune relation**. Un Symbol est un rendez-vous par
    nom, sans session d'instantané, et rien ne le reprend ;
  - ticket :
    `docs/banc/tickets/2026-11-14-un-symbol-que-plus-rien-ne-relie-reste-apres-le-retrait-de-son-fichier.md`
    (rétention voulue ou fuite, à décider).

- **Le correctif du File** (`orphelins-pile-2` ff4ced00a contre 9d90edbb6, rejoué le 14 nov. à
  23 h 34, même montage). Recherche donne la cause : le drain ne validait jamais au plein texte une
  table qui ne fait que perdre des lignes, faute de `FlushNode` après `DeleteRecordNode` ; la
  suppression restait en attente puis se perdait à la fermeture.

  | étape | File au parent (documents / lignes) | File au lot |
  |---|---|---|
  | après l'index de la source entière | 7 811 / 7 608 (+203) | 7 608 / 7 608 (0) |
  | après l'index sans `arbre.rs` | 7 608 / 7 607 (+1) | **7 607 / 7 607 (0)** |

  - Scope, Symbol et Library sont inchangés des deux côtés. Les lignes du fichier retiré sont
    effacées des deux côtés, et les 3 Symbol exclusifs restent (ticket à part) ;
  - **les 30 cas sont VERTS au bit près** ;
  - l'écart de 203 après l'index de la source entière disparaît lui aussi au lot. C'étaient les
    suppressions déjà perdues du même défaut, celles de l'index qui avait fait passer la base de
    79 662 à 84 211 Scope.
  - **Sur master** en 262be6fc4, avec le même Rust que ff4ced00a, reposé sur la pile de G (au mot
    de recherche, 15 nov.).

## 5. La tête de pile d'everything declarative (3a6bf34cd contre ac8644784)

La pile porte deux résolutions de conflit dans `occurrence_mask_options`, face au court-circuit
d'arbre principal, et un sens changé : sans filtre, s'il existe des versions orphelines, c'est le
prédicat qui sert et non la liste. La parité de 959d73513 ne couvrait plus cette tête : elle est
rejouée.

| cas | verdict au bit près | contre la ligne de base `migree.json` |
|---|---|---|
| les 30 cas, copie migrée telle quelle | **VERT**, 30/30 | VERT, 30/30 |
| les 30 cas, copie à versions orphelines | **VERT**, 30/30 | ROUGE, 16 cas : c'est attendu, voir plus bas |

- **Les formes** : le parent n'a pas le passage aux générations, il lit la base dans sa forme à
  versions. La tête passe aux générations. Les réponses ne bougent pas.
- **Les versions orphelines, fabriquées** (relevé sous `RAG3WEAVER_PARITE_ORPHELINES`) :
  - les graines sont les Scope des trois premiers rangs sans filtre de la ligne de base : 30
    Scope ;
  - toutes les occurrences de leurs versions sont effacées de la copie, 30 occurrences ;
  - les versions sans origine passent de 0 à 30, au parent comme à la tête ;
  - le parent masque donc par la liste, la tête par le prédicat.
- **Les orphelines ne sortent plus** : aucune des 30 graines n'est dans les dix premiers, ni au
  parent ni à la tête. La ligne de base en rendait 30 sans filtre, et 33 toutes formes. Le rouge
  contre la ligne de base est donc l'effet voulu : il montre que le cas touchait bien les premiers
  rangs.

## 6. L'écrivain des générations de 2a′-2 (7ddedb026 contre 1105a2590b), la nuit du 15 novembre

**Le montage** (`parite-ecrivain.sh`, à luciepc) :
- des deux côtés, une copie neuve de la base migrée, passée aux générations ;
- un index de ma source (835d5298f), puis la publication de la génération au lot. Le juge
  appelle `publier_la_generation` : rien d'autre ne publie à ce jour ;
- les 30 cas avant l'index (au bit près) et après la publication. Ce dernier relevé est jugé au
  régime `reponses` : une ligne qui change naît sous un autre uuid, donc l'identité est (chemin,
  nom, ligne, hash) ;
- les usages de cinq noms, choisis par le parent et relus au lot ;
- le retrait et la remise d'`arbre.rs`, chaque index publié ;
- le parseur est le même des deux côtés (codeparsers 6ba8478b03).

| | verdict |
|---|---|
| les 30 cas avant l'index | **VERT** au bit près |
| les usages des cinq noms | **VERT**, mêmes listes (fichier, appelant) |
| `aretes_recopiees` à la génération 1 | 2 844 |
| le retrait et la remise | conformes : Scope et File fermés à 2 au retrait, des lignes neuves nées à 3 à la remise ; les 3 Symbol exclusifs visibles aux rangs 1 et 3, invisibles au rang 2, leurs lignes physiques gardées |
| les 30 cas après l'index et la publication | **ROUGE** : 15 cas rendent une autre liste dans leurs dix premiers |

**Le départage** (une sonde Cypher sur les copies gardées, sans rien réindexer) :
- **pas une ligne fermée qui sort** : aucun des 128 résultats du lot n'a de `_jusqu_a` ;
- **le saut des fichiers inchangés** (le lot, dd729317a) : un fichier sauté garde l'analyse du code
  plus vieux qui avait indexé la base. Le parent réanalyse tout. Le lexical classe donc sur deux
  ensembles de contenus : 84 211 ScopeContent au parent, dont 14 201 orphelins que son chemin
  laisse, et 74 274 au lot. Everything declarative le tient pour un défaut de son lot : sans
  version de l'analyse, un parseur changé ne s'appliquerait jamais à un fichier inchangé ;
- **la parité est mal posée sur cette base** : elle a été indexée par un code plus vieux que les
  deux côtés. Une parité propre part d'une base indexée par le parent lui-même.

**Trois défauts du juge**, trouvés et corrigés en route :
- la réponse json de `usages` range l'appelant sous `user` (`path`, `title`) ;
- rag3db n'a pas `type(r)`, mais `label(r)` ;
- le relevé du retrait et celui de la parité prenaient la même copie de base : le premier a
  réécrit le second. Chaque relevé a désormais la sienne. Les comptes du lot de la sonde sont donc
  ceux de l'état du retrait.

**La copie réécrite, et la règle qui en sort** (au mot de c7) :
- le rejeu du retrait au lot, le 15 novembre à 3 h 08, a pris la copie `ecr-base-lot2`, celle que
  la parité venait d'écrire, et l'a réécrite. La sonde a donc lu au lot l'état du retrait (rangs 1
  à 3) et non celui de la parité. La lecture tient : les 128 uuid du top 10 sont des lignes nées à
  0 ;
- **la règle** : un nom de copie porte le rôle (parent, lot, retrait…), le relevé et la date
  (`ecr-base-<rôle>-<relevé>-<AAAAMMJJ>`). Deux relevés ne partagent jamais une copie.

**Le défaut voisin, en ticket** :
`docs/banc/tickets/2026-11-15-les-versions-orphelines-du-chemin-d-avant-faussent-le-bm25-d-une-base-reindexee.md`.
Les 14 201 ScopeContent orphelins du chemin d'avant faussent le BM25 de toute base à versions sans
générations réindexée.

**L'ordre décidé** (c7, everything declarative) :
1. le correctif d'everything declarative, après son témoin rouge : la version de l'analyse
   (parseur, règles d'import, gabarit) entre dans l'empreinte indexée ;
2. la parité propre : une base indexée d'abord par le code du parent lui-même, copiée, puis les
   deux côtés réindexent une source qui diffère de quelques fichiers ;
3. et l'actuelle, rejouée sur une copie neuve de la base migrée, qui devra passer aux orphelins du
   parent près.

**Les noms des usages** (LB, NS, UP, a, absent) ont été choisis parmi les scopes dont le hash
change à l'index. Sur cette base, c'est surtout le bruit du hash recalculé. Le vert des usages
vaut pour ces noms.

## 7. Le correctif (A) de l'écrivain, et une leçon de banc (15 novembre)

**La leçon** (au mot de c7) : **une parité prouve d'abord que les deux côtés ont bâti et joué.**
Avant tout écart, on compte les relevés et les cas de chaque côté.

**Ce qui l'a appris** : la première passe du correctif (d087e02c7 contre sa base e95a1b8c60, à
11 h 53) a perdu son parent sans le dire au juge.
- La sonde Cypher de la nuit, ajoutée au même fichier de test, appelle `rang_publie`. Le parent ne
  porte pas cette méthode : il n'a compilé dans aucune des deux parités, et n'a écrit aucun relevé.
- La copie de base du parent, faite avant la compilation, est restée la base migrée intacte. La
  base propre P0, copiée d'elle, n'était donc pas indexée par le parent.
- Seul le lot de la parité actuelle reste valable : `aretes_recopiees` vaut 16 543 à la
  génération 1, et l'index du lot a pris 2 971 s, puisque tout repasse une fois quand la colonne
  `analyse` est vide.

**Ce qui est changé** :
- le script retire la sonde Cypher du test là où le code n'a pas `rang_publie` ;
- avant tout juge, `joue` vérifie que chaque côté a écrit ses deux relevés, avant et après l'index,
  et dit leur nombre de cas. Un côté qui manque rend la passe ROUGE, sans juger ;
- la parité du parent et la parité propre sont rejouées, sur une vraie P0, à partir de 13 h 49.
