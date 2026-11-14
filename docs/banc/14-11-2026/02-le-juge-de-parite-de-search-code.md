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
