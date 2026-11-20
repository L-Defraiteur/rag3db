# Le juge des déclarations courtes, la fusion et le nom exact

20 novembre 2026, de 16 h 30 à 17 h 40, à luciepc. Suite de la page `04-le-juge-des-declarations-courtes.md` : ED a
repris son lot pour que les déclarations courtes sans vecteur restent trouvables par leur nom.

- **(A)**, `variable-not-candidate-5` `93dd826183` (règles 13) : la fusion juge une ligne sans morceau sur les signaux
  qu'elle peut avoir.
- **(B)**, `531a4dbd17` : un paramètre `exact_name='name'` du nœud de fusion, dans le graphe `search_workspace.mmd`, met
  devant un résultat dont le nom est exactement la requête. Recherche seule : ni règles ni index.

## 0. La recette changeait : les hôtes lisaient le graphe de recherche du service

Le manifeste de la copie de G (`mesures/code-principal-migree-repassee-20261120/diag/backend.json`) cite ses outils dans
les **gabarits installés du service** : `~/.config/rag3weaver/code-principal/templates/tools/search_workspace.mmd`. Mes
hôtes lisaient donc ce graphe, et pas celui de la tête jouée. Un changement de gabarit comme (B) n'était jamais chargé :
le premier rejeu de (B) a rendu au rang près les chiffres de (A).

**La pièce qui le prouve** : rejoué sur **ses propres gabarits**, master (`216c3a70e6`) ne rend pas les rangs du juge
de 16 h 30.

| nom | master, gabarits du service | master, ses gabarits |
|---|---|---|
| research | 1 | 7 |
| threads | 1 | 6 |
| pool | 3 | 9 |
| echecs | 1 | 3 |

Les juges de la journée restent valables sur ce qui passe par le binaire (l'analyse, l'index, la fusion codée en Rust) ;
seul un changement de gabarit leur échappait. **La recette corrigée** (`juge-requetes2.sh`) réécrit, pour chaque côté,
ce préfixe vers les gabarits de son worktree, et refuse si le graphe de recherche n'est pas celui du worktree.

## 1. (A) seule, par les signaux

Sur les bases du juge de 16 h 30 : en BM25 seul, `research` est au rang 7, `threads` au rang 5 et `ENTETE` au rang 2, des
deux côtés. Sur master, leur propre vecteur les mettait premiers ; avec (A), ils n'ont plus que le BM25. (A) fait ce
qu'elle peut. Seul un nom exact dominant, (B), les remet en tête.

## 2. (B), chaque côté sur ses gabarits, en requêtes seules

Les bases indexées à 16 h 30 (master `feuille-master-1630`, lot `feuille-lot-1636`), lues par le binaire et les
gabarits de chaque côté. Sorties : `banc-parite/sorties/feuille-5c-20261120/` à luciepc.

**Les noms courts : vert.** Les 8 noms que master met premiers le restent avec (B). Cinq montent au premier rang :
research (7 → 1), threads (6 → 1), echecs (3 → 1), pool (9 → 1), CONFIGS (3 → 1). Des fonctions d'une ligne,
`norm_path_theirs` (1) et `secondary_indexes` (2) gardent leur rang, `lines` sort du top 10 (6 → absent).

**Le critère de G** : recouvrement 92 %, usages identiques (Catalog 900 = 900). Le même premier est à 6/10 au sens strict,
**10/10 avec la règle de c7** (17 h 55) : « même premier, ou premier = la définition du nom exact ». Les quatre
premiers changés vont tous à la définition qui porte exactement le nom : `commit` → `fn commit` (declare.rs:264),
`mutex` → `static mutex_t &mutex()`, `chunk_uuid` → `fn chunk_uuid` (uuid.rs:46), `acquireLock` →
`Transaction::acquireLock` (transaction.cpp:96). La recette applique désormais cette règle elle-même : un premier
changé est tenu quand la requête est un identifiant et que la ligne du nouveau premier le définit.

## 3. Le défaut : (B) ressuscite ce que 2.12 écarte

- « rag3db » filtré sur `src/include/` donne en tête `namespace rag3db {` (`src/include/common/index_search_types.h:11`,
  18 caractères), un namespace replié que 2.12 retire du plein texte et des morceaux ;
- « PortDef » donne en tête `impl PortDef {` (`extension/rag3weaver/src/dataflow/port.rs:222`, 14 caractères).

La voie du nom exact lit les scopes sans passer par la candidature. ED corrige : (B) ne considère que les lignes
candidates, celles qui ont un document de plein texte. Le rejeu suit sur sa tête.

## 4. Les premiers changés par le sens

Sur les 40 requêtes, trois requêtes de sens changent de premier ; aucun nouveau premier n'est une déclaration de valeur
courte. ED les explique un par un.

| requête | premier sur master | premier avec (B) |
|---|---|---|
| « parse a json manifest », sans filtre | `css_parser.rs:403` | `experiments/mtga/scripts/prepare_engine_relations.py:14`, une ligne de 220 caractères (`manifest=json.loads(…)`) |
| « rebuild the vector index », langage python | `scripts/test_backend_persistence.py:89` | `experiments/mtga/scripts/build_plant_variant.py:32`, une ligne de 152 caractères (`index=ROOT/'README.md';s=index.read_text()…`) |
| « transaction rollback », langage python | `experiments/mtga/scripts/audit_validated_run.py:40` | `extension/rag3weaver/scripts/test_backend_snapshot.py:128`, un `assert not again['ok'] and 'déjà annulée' in …` de 65 caractères |

« rag3db » sans filtre donne en tête `declare const rag3db: {` (23 caractères), une déclaration courte ; mais la requête
est son nom, et c'est l'effet voulu de (B), pas un premier pris par un sens.
