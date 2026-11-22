# Les ratés de la recherche, cause par cause

22 novembre 2026, session recherche, avec banc. Demande de c7 : prendre un par un les ratés du jeu de banc
(l'or n'est pas dans le top 10 de `search_code`), classer leur cause, puis un lot par cause, le plus gros d'abord.
Le jeu de banc juge chaque lot de la recherche.

**État de la page : provisoire.** La base de mesure de banc avait un défaut (premier point). Ce tableau sera refait
sur la base reconstruite. Les rangs ci-dessous ont été relevés sur l'index du master courant
(`rag3db @ d3ef06279e`), par `search_code` borné au fichier de l'or.

## 1. D'abord la mesure

- **La base de mesure indexait un arbre du 18 octobre.** `source-code-principal-20261120` a pour racine l'arbre de
  Lucie à luciepc, resté à `284cc00c0b` (18 octobre). Six ors du code (`sweep_dead_index_caches`,
  `FtsMaintenancePolicy`, `SearchSettings`, `recent_searches`, `stop_run`, `merge_highlights`) et quatre pages
  (`AGENTS.md`, `docs/README.md`, `docs/nouveautes.md`, `docs/procedures/push-d-un-lot.md`) n'y existent pas. Cela
  fait 10 ratés sur 26 qui ne mesurent pas la recherche. Le remède : rebâtir la base sur un worktree de banc, au
  master du jour, avec le commit dans son nom.
- **Les questions du jeu sont indexées avec le code.** `docs/banc/jeux/questions-recherche-v1.md` sort en tête du
  dense pour q21, q24, q28 et q30. Les `tests/e2e_banc_*.rs` sortent premiers au BM25 pour q01, parce qu'ils
  contiennent le texte des questions. Il faut les écarter de la base de mesure ou du jugement.
- **Les gains de (a)+(b) tiennent** (phrase entière exacte, `contains` à `fields`) : or premier 9 → 11, rappel@10
  18 → 20, sans recul. Les deux côtés ont été mesurés sur la même base.

## 2. Le tableau (index du master courant)

Causes : **A** l'or manque ou est mal découpé ; **B** la doc de l'or est détachée ou son texte trop mince pour le
dense ; **C** les tests et usages de l'or passent devant la déclaration ; **D** d'autres fichiers ou des pages passent
devant ; **E** la question est mal posée.

| id | or | dense dans son fichier (rang / score / premier) | BM25 dans son fichier | cause | seconde | note |
|---|---|---|---|---|---|---|
| q01 | `FuseResultsNode` | 3 / 0,694 / 0,706 | ~40 | B | D | sa doc est au-dessus de `DuplicatePolicy` |
| q02 | `FuseResultsNode` | 9 / 0,705 / 0,728 | 8 | B | D | même doc détachée |
| q12 | `SearchOptions` | 3 / 0,655 / 0,728 | 3 | B | C | sa doc est au-dessus de `RerankOptions` ; struct de 80 lignes de champs |
| q14 | `merge_highlights` | 3 / 0,678 / 0,703 | absent | B | D | la doc dit « passages », la question « surlignages » |
| q17 | `sweep_dead_index_caches` | 1 | 1 | A (base) | | premier au dépôt entier sur l'index courant |
| q19 | `check_fts_counts` | 3 / 0,708 / 0,740 | 8 | D | B | `open_fts_files` porte une doc égarée qui parle de vérifier |
| q20 | `open_fts_files` | 3 / 0,704 / 0,730 | 3 | B | C | trois docs empilées (`drop_fts_index`, `verify_fts_files`, la sienne) |
| q21 | `FtsMaintenancePolicy` | 1 | | A (base) | D | la page des questions de banc sort première |
| q23 | `BackendManifest` | 6 / 0,704 / 0,738 | absent | E | B | « backnd » ; « manifeste » n'est pas dans le texte ; 160 lignes sans doc |
| q24 | `SearchSettings` | 7 / 0,716 / 0,737 | absent | B | D | sa doc (« settings of the read tools ») n'a aucun mot de la question |
| q28 | `recent_searches` | 5 / 0,706 / 0,765 | 2 | E | C | la doc dit « d'un backend », la question « de l'hôte » |
| q29 | `stop_run` | 1 | 1 | A (base) | C | cinquième au dépôt entier sur l'index courant |
| q30 | `shutdown` | 10 / 0,725 / 0,771 | absent | E | C | plusieurs réponses justes ; homonymes `Catalog::shutdown`, `Backend::shutdown` |
| q36–q41 | pages | | | A (base) pour q37, q38, q39, q41 | | q36 et q40 à revoir sur la nouvelle base |
| q46 | `SearchOptions` (« SearchOptons ») | | | à revoir | | faute anglaise, l'or existe dans les deux arbres |

Familles 2 et 3 de banc (un signal trouve l'or, la fusion l'enterre) : q10, q43, q45 (dense) ; q11, q27 (BM25). Elles
désignent la fusion, et sont à classer sur la nouvelle base.

## 3. Les lots qui en sortent, le plus gros d'abord

1. **La base de mesure** (banc) : rebâtie au master du jour, sans les fichiers du jeu de questions. Rien ne se juge
   avant.
2. **Les docs égarées dans le source** (codeparsers, qui l'a pris) : `FuseResultsNode`, `SearchOptions`,
   `open_fts_files`, `ExactNameNode`/`SignalInput`, `maintenance_periodique`, `fils_du_moteur`, `handle_of_folder`. Le
   parseur dit vrai : un `///` documente l'item qui le suit. Une garde peut les empêcher de revenir.
3. **La fusion qui enterre un premier** (familles 2 et 3) : un or qu'un signal met premier ne devrait pas sortir du
   top 10. Lot de recherche, jugé par banc.
4. **Les tests devant la déclaration** (C, seconde cause six fois) : à mesurer sur la nouvelle base avant d'y toucher.
5. **Les questions mal posées** (q23, q28, q30) : à reformuler par banc.
