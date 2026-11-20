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
`norm_path_theirs` (1) et `secondary_indexes` (2) gardent leur rang. Pour `lines`, le « 6 → absent » de mes
relevés est le rang de son **fichier** (une requête par le sens compte le premier résultat dans le bon fichier) : c'est
`cards`, la ligne 14 de prepare_engine_relations.py (220 caractères), qui sort du top 10. La fonction `lines` elle-même
(ligne 12) n'est dans le top 10 d'aucun côté.

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

Sur les 40 requêtes, trois requêtes de sens changent de premier. **J'avais écrit qu'aucun nouveau premier n'est une
déclaration de valeur courte : c'est faux, voir le § 6.** ED les explique un par un.

| requête | premier sur master | premier avec (B) |
|---|---|---|
| « parse a json manifest », sans filtre | `css_parser.rs:403` | `experiments/mtga/scripts/prepare_engine_relations.py:14`, une ligne de 220 caractères (`manifest=json.loads(…)`) |
| « rebuild the vector index », langage python | `scripts/test_backend_persistence.py:89` | `experiments/mtga/scripts/build_plant_variant.py:32`, une ligne de 152 caractères (`index=ROOT/'README.md';s=index.read_text()…`) |
| « transaction rollback », langage python | `experiments/mtga/scripts/audit_validated_run.py:40` | `extension/rag3weaver/scripts/test_backend_snapshot.py:128`, un `assert not again['ok'] and 'déjà annulée' in …` de 65 caractères |

« rag3db » sans filtre donne en tête `declare const rag3db: {` (23 caractères), une déclaration courte ; mais la requête
est son nom, et c'est l'effet voulu de (B), pas un premier pris par un sens.

## 5. (B) corrigé : vert

`03fbecf8b8` : (B) ne promeut plus un résultat qui replie des enfants (`exact_name_leaf='folds'` dans
`search_workspace.mmd`). Même recette, contre master `216c3a70e6`, chacun sur ses gabarits. Sorties :
`banc-parite/sorties/feuille-5d-20261120/`.

- **Le défaut est corrigé** : « rag3db » sur `src/include/` garde le premier de master (`c_api/rag3db.h:330`) ;
  « PortDef » donne `pub struct PortDef {` (`port.rs:179`), la définition du nom exact, et non plus `impl PortDef {`.
- **Le critère de G** : 92 %, **10/10 avec la règle du nom exact**, usages identiques (Catalog 900 = 900).
- **Les noms courts** : les 8 que master met premiers le restent ; research, threads, echecs, pool et CONFIGS montent au
  rang 1.
- **Les premiers changés de mes 30 cas, un par un** :
  - par le nom exact : « rag3db » sans filtre → `declare const rag3db` ; « PortDef » → `pub struct PortDef` ;
    « value » → `IteratorType::reference value() const` (`third_party/nlohmann_json/json.hpp:5228`) ;
  - par le sens : les trois du § 4, inchangés ;
  - **le seul premier sorti du top 10** : « transaction rollback » [python]. Sur master, c'était
    `last_accepted=(deck,restored)` (`audit_validated_run.py:40`), une déclaration de valeur de 29 caractères : le lot
    lui retire son vecteur, c'est l'effet voulu, pas (B).
- **La requête de `lines`**, en trois modes : aucune déclaration courte n'entre dans le top 10. Par défaut et en dense,
  ce sont des fonctions et méthodes de l'extension json, plus `events` (une variable de 92 caractères, qui garde son
  vecteur) ; en BM25 seul, des fonctions de rag3weaver, identiques des deux côtés.

**Verdict : vert sur le nom exact et le critère de G, mais rouge sur le revers de (A) : voir le § 6.**

## 6. Correction : le revers de (A) n'est pas tenu

J'avais jugé le revers sur la **ligne source** du premier rendu (220 et 152 caractères), lue par sa clé `chemin:ligne`.
Or une ligne Python qui enchaîne des affectations porte plusieurs scopes `variable`, et cette clé ne dit pas lequel est
rendu. Relu par l'hôte du lot (`03fbecf8b8`, ses gabarits, base `feuille-lot-1636`, limite 10, format JSON, avec le
nom et le contenu de chaque résultat ; `quel-scope.json`) :

| requête | premier avec le lot | ensuite |
|---|---|---|
| « parse a json manifest » | `manifest`, variable de **51 caractères** (`prepare_engine_relations.py:14`) | `manifest` (33 car., `test_backend_persistence.py:78`), puis `parse_declaration`, le premier de master |
| « rebuild the vector index », langage python | `index`, variable de **22 caractères** (`index=ROOT/'README.md'`, `build_plant_variant.py:32`) | les trois autres `index` des jumeaux mtga |

**Ce sont des déclarations de valeur courtes, sans vecteur, qui prennent la tête de requêtes par le sens.** Un mot de la
requête est leur nom, et (A) les remonte sur le seul BM25 ; ce n'est pas `exact_name`, la requête n'étant pas leur nom
exact. **Le critère « aucune ligne sans vecteur ne prend un premier rang par un sens qu'elle n'a pas » est rouge.**

- **Les versions fermées ne sont pas en cause.** Les scopes rendus sont les versions courantes. Les vieilles versions de
  la ligne entière (`_depuis` 0, `_jusqu_a` 1, avec un `_snapshot`, laissées par une analyse antérieure de la copie)
  partagent la même clé `chemin:ligne`, d'où la confusion. Les générations 0, 1 et 2 des deux bases sont publiées.
- **La recette change** : le revers se juge désormais sur le scope rendu (nom, type, taille du contenu, par la réponse
  JSON), jamais sur la ligne source.

## 7. (A') : la ligne sans vecteur comptée après la liste dense

`vectorless-below-best` `68b82e86df` (fusion seule, graphe inchangé) : une ligne sans vecteur est comptée au rang qui suit
le dernier de la liste dense. En requêtes seules sur `feuille-lot-1636`, par un hôte de la tête et ses gabarits, jugé
sur le scope rendu (`revers-a-20261120/revers-juge.json`). **Rouge, à moitié réparé.**

- Les 13 noms tapés exactement, premiers dans les relevés du lot, le restent, à leur ligne.
- Les 10 requêtes de G n'ont aucune déclaration courte en tête.
- « rebuild the vector index » [python] : **réparé**. En tête, `file_scope_08` (module, 105 caractères), et la variable
  `index` (22 caractères) descend au rang 3.
- **Restent en tête, sans vecteur** : `manifest` (variable, 51 caractères) pour « parse a json manifest », sans filtre et
  en python ; `s` (variable, 19 caractères, `s=index.read_text()`) pour « read_text », sans filtre et en python. Le mot
  de la requête est dans leur contenu, pas leur nom : le signal dominant `bm25_exact` les garde devant.

**La référence d'avant la bascule** (hôte baccb2ea6e, ancienne analyse, ancienne fusion, ouverte par G) : « parse a json
manifest » y rend en tête une méthode (`parse_declaration` sur code-principal, `read_manifest` sur code-lourd), la
variable courte `packageJson` au rang 3 ou 4. **Le revers est donc apporté par la bascule, pas hérité.** « rebuild the
vector index » [python] ne s'y compare pas : l'ancien hôte refuse `language` en argument et ignore `options.language`.

## 8. Le juge sur les bases vivantes, après la fenêtre de re-bascule

Par l'hôte de service (`83c667e420`, 7970), gabarits livrés (`builtin:`), en lecture seule, le 20 novembre vers 21 h 20.

| base | juge | requêtes de G | noms courts | 30 cas |
|---|---|---|---|---|
| code-principal (luciepc) | contre les relevés du lot de feuille-5d | **10/10**, recouvrement 100 % | **13/13** premiers | 0 premier différent |
| code-lourd (luciepc) | à un côté, sur `rag3db-lourd` a72acfeef0 | les 5 identifiants rendent leur définition | 12/20 premiers à leur ligne, 7 battus par un homonyme, 1 absent (`BINARY`) | — |
| code-rag3db-depot (tablette) | à un côté, sur l'arbre de Lucie 327534c972 | les 5 identifiants rendent leur définition | 10/20 premiers à leur ligne | — |

**Vert sur le critère, aux trois bases.** Les trois requêtes au premier changé de code-principal rendent les bons scopes :
`point_de_reprise_final` (function), `copy_links_from_csv` (method) et `add_rag3db_test(e2e_test …)` (texte brut).

**Un point à regarder sur la tablette** : cinq noms tirés de son arbre sont absents du top 10, sans aucun homonyme qui
l'explique : `__all__` (`tools/python_api/src_py/__init__.py:73`), `lib` (`third_party/pybind11/tools/libsize.py:12`),
`result` (`docs/arbre-principal/13-11-2026/couches-2a-union-detache/lancer.py:76`), `con`
(`benchmark/click/load.py:8`), `PORT` (`extension/rag3weaver/tty/mesures/mesure.py:44`). Le nom exact n'en remonte aucun.
Reste à lire si ces scopes sont dans la base ; ce n'est pas classé rouge sans l'avoir vu.

**À un côté** : code-lourd et la tablette indexent un autre arbre que code-principal (`rag3db-lourd`, l'arbre de Lucie à
la tablette). Leurs lignes diffèrent, et une clé `chemin:ligne` ne se compare pas d'une base à l'autre. Le juge y tire
ses noms de l'arbre de la base, et un nom battu par un homonyme défini ailleurs (une requête ambiguë) n'est pas un défaut.
