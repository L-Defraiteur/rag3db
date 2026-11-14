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
