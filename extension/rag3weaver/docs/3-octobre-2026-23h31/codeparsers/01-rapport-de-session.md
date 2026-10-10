# Session codeparsers — rapport

Tenu à jour sur place. Dernière mise à jour : 4 octobre 2026, 22 h.

La session tient l'analyseur (dépôt `L-Defraiteur/codeparsers`, sous-module
`extension/rag3weaver/codeparsers`) et, côté rag3weaver, les outils qui
lisent le graphe du code : `usages`, `impact`, la lecture du catalogue, le
banc des relations, et la section « Liens ». Elle ne touche ni
`code.rs`, ni `code_sync.rs`, ni le schéma de code, ni les gabarits du
backend sans passer par la session de l'arbre principal : elle propose des
branches.

## Fait

| Lot | Où | Pourquoi |
|---|---|---|
| Genre d'usage et ligne sur chaque relation | codeparsers `11ecd4f`, doc `3-octobre-2026-20h30/01` | « qui appelle » et « qui ne fait que nommer le type » se confondaient |
| 1 552 fausses arêtes retirées, classe par classe | codeparsers `f0faa82`, doc `…/02` | des héritages et des appels inventés par le nom polluaient tout voisinage |
| Deux analyses du même code, mêmes relations | codeparsers `924a1d3` | l'ordre des fichiers changeait les arêtes |
| Appel sur une variable typée | codeparsers `9a1fae3`, doc `…/03` | `x.f()` avec `x: T` vise `T::f`, pas un homonyme |
| Fonctions d'un `mod` Rust en scopes | codeparsers `1f9d653`, doc `…/04` | les tests unitaires Rust n'existaient pas dans le graphe |
| Marque de test, sûre ou de convention | codeparsers `137b9d2`, `594fc61`, doc `…/05` | `impact` doit nommer les tests qui traversent un changement |
| Imports et bibliothèques (`USES_LIBRARY`) | codeparsers `a569417`, doc `…/06` | une dépendance externe est une chose, pas un nom perdu |
| Appels typés par un champ ou un retour | codeparsers `56d039f` (pointeur 8), doc `…/07` | `self.store.get()` vise le `get` du type du champ |
| Le parent est l'homonyme qui contient | codeparsers `ace6fd8` | deux `impl` du même nom donnaient le premier venu pour parent |
| C# épinglé en `=0.23.1` | codeparsers `1e27455` | 0.23.5 est en ABI 15 et fait paniquer tout `.cs` |
| `usages` | rag3db `551ed2d1f` | une chose, ses définitions, ses usages groupés par genre, fichier et ligne |
| `impact` | rag3db `c6065b35e` | ce qu'un changement peut toucher, par niveaux, et les tests qui le traversent |
| Lecture du catalogue | rag3db `c6065b35e`, `42df4136f` | jamais un vide sans dire d'où il vient ; la porte lit l'état **de l'entité** (`index_state_for(pivot)`) : dans le chat réel, le journal de conversation faisait quitter « jamais » à l'état global |
| Banc des relations | rag3db `a58137cb8`, doc `…/08` | vingt questions dont la réponse dépend du graphe : la référence avant tout poids de proximité |
| Un accès de champ ne prend pas de rendez-vous | rag3db `bb98827af` (fusionné par l'arbre principal) | `self.config`, `r.chunk` se reliaient à une fonction homonyme |
| La porte d'index lit l'état de son entité | rag3db `42df4136f` | `index_state_for(pivot)` : le journal de conversation faisait quitter « jamais » à l'état global |
| « Fichier seul », `project_files`, `import_origin` | codeparsers `fichier-seul` `ba8449c`, `07935e5` | entre fichiers, le résolveur devinait (35 fausses sur 50) et le graphe dépendait du paquet ; un appel et 85 paquets rendent maintenant les mêmes 371 289 relations |
| L'analyse linéaire, et une sortie déterministe | codeparsers `fichier-seul` `8d6d212`, `2c17314` | la ligne de contexte et la doc découpaient le fichier entier ; l'ordre des références dépendait du fil. Dépôt entier en un appel 19,9 → 8,7 s, par paquets 34,7 → 12,8 s ; empreinte de chaque fichier identique avant et après |

Mesure du dernier : « tests qui traversent » 0,92 → 0,93 de précision
(chunk_lines 0,68 → 0,74), aucun rappel perdu ; e2e_code : arêtes par le
nom 4 504 → 4 382. Une première version qui écartait aussi les **appels**
par variable sans type perdait de vrais appelants (begin_snapshot,
estimate_of → probe_rate) : la règle ne vise que les non-appels.

## En cours

**Fait depuis la reprise (4 octobre, matin)** :
- le résolveur unique est sur master (`63d154730`), codeparsers master
  avancé sur `d567e7c` ;
- **l'impact d'un fichier** (`9425feb05`) : `NeighborhoodNode(file=…,
  format=summary, summary_group=case)`, gabarit `impact_fichier.mmd`
  (`path_in_source`) — pour la section « Avant d'éditer » de la session
  recherche, qui le branche après ses passes ; 82 à 148 ms sur l'index de
  `src/` ;
- **les déclarations dans `usages`** (`76e850b88`) : « défini foo.cpp:3 —
  déclaré foo.h:5 (dans Foo) », et la déclaration seule quand rien n'est
  défini ; inerte tant que l'arbre principal n'a pas posé la propriété
  `declarations` (son lot en cours, avec le pointeur `declarations`) ;
- **les relations qui dépendent du paquet** : localisées (sonde
  `relations_selon_le_paquet`, `59e836366`) — les numéros d'homonymes des
  clés stables de `code.rs`, pas la sortie de codeparsers ; ticket mis à
  jour, transmis à l'arbre principal ;
- six tickets ouverts dans `docs/tickets/` (`a5e829d5b`).

**La cohésion (C1) : mesurée, éteinte par défaut, option gardée.**
`SearchOptions.cohesion { weight, relations, … }` (`b9347b459`) : un nœud
après la fusion dans `search_base` (`CohesionBoostNode`) multiplie chaque
score par `1 + W × cohésion normalisée` (somme de 1/sauts vers les autres
candidats, carrefours exclus) ; sans option, il laisse passer. Mesure par la
voie `banc_cohesion_produit` du banc étagé (granite-278m, titre indexé),
par-dessus la fusion du produit (bm25 0,45 / vector 0,55) :

| W | MRR | R@1 | R@5 | identifiants | latence |
|---|---|---|---|---|---|
| 0 | 0,419 | 14 | 21 | 0,900 (8/10) | 329 ms |
| 0,2 | 0,431 | 15 | 21 | 0,950 (9/10) | 370 ms |
| 0,5 | 0,420 | 14 | 23 | 0,933 (9/10) | 424 ms |

Un gain petit (+0,012 à W = 0,2, au-dessus de la variance ~0,002), pour
+40 ms ; sur un graphe hybride plus faible (0,312) il était de +0,065 — la
fusion du produit fait déjà l'essentiel (descendre les `file_scope` isolés).
Décision de l'orchestration : éteinte par défaut, **à remesurer si la fusion
change**. Le `CohesionNode` (signal de boost) et le banc
`e2e_banc_cohesion` (graphe à part) restent.

**La section Liens est allumée, en arbre** (`6380dd8a5`, 4 octobre).
Crochet `after` de `search_code` dans les deux manifestes du backend de code
(`backend.json`, `snapshot.json`), deux sauts, `max_lines` 14, port
`render.results`. Rendu dans la forme du « Dependency Graph » de
`results.md.jinja` : le pivot (le nœud le plus relié) une fois, avec son
lieu ; `├── [CONSUMES]` / `[CONSUMED_BY]` disent le sens ; le lieu d'un
voisin n'apparaît que s'il n'est pas un résultat ; un intermédiaire hors des
résultats porte « · hors résultats » ; la coupe dit « … et N autres paires
reliées ». Un seul dessinateur, `crate::arbre::arbre`, pour les Liens et le
« Graphe » de `code_tools.rs` (sortie inchangée, à l'octet) ; le gabarit
jinja garde sa boucle — ticket « trois rendus d'arbre à fondre ».

`max_lines` 14 et non 6 : un arbre de deux groupes et quatre voisins fait
déjà 8 lignes avec ses clôtures, et une coupe au milieu laisserait une
clôture ```` ``` ```` ouverte. Les trois exemples tiennent en 13. Le crochet
« Le même motif existe ailleurs » de la session recherche reste à 6 ; son
« Avant d'éditer » de `read_file` n'est que dans `backend.json` (asymétrie
à elle, laissée).

Preuves : `e2e_liens` (le manifeste réel se charge et porte le crochet ;
muette sans index ; arbre exact d'un voisinage partagé ; deux homonymes
d'un fichier ne font pas un lien ; plus courts chemins contre l'oracle du
moteur), `test_backend_code.py` PASS, `e2e_agent_loop` 8/8 (rouge depuis
`b9347b459` : le nœud `cohere` changeait les comptes du graphe de
recherche). Reste `clear`, résolu par le nom seul : ticket de la marque de
résolution.

**Les relations qui dépendent du paquet : fermé.** Les boucles sur soi
venaient de codeparsers (nom d'un destructeur `~Foo` résolu vers sa propre
classe) : corrigé dans `d51e933`, pointé en `4fe5a3bfc`. Sonde rejouée sur
le dépôt entier : 264 156 relations et 601 929 rendez-vous, identiques à 64
et à 512 ; zéro boucle.

**La ligne de relation des arbres se lit `~ Consumed by ~`** (`98478b5ef`,
demande de Lucie) : le nom en casse de phrase, tirets bas en espaces, entre
deux tildes, règle tirée du nom. Aux trois rendus : `arbre::relation_label`
(Liens, Graphe du grep) et `replace("_", " ") | capitalize` dans
`results.md.jinja`.

**La marque de résolution.** La marque (`choose_target`, colonne
`resolution`) et le filtre sont sur master (f4495af6a, 15efaca4f) : Liens
ne suit pas les arêtes devinées, `usages` dit « (par le nom) », un appel par
chemin vaut un import. Pas dans `impact` : « relie » y tombe à 0,75.

Depuis, pour faire baisser les « nom » :
- codeparsers 5778d92 : un nom dans les arguments d'une macro se lit comme
  hors macro (chemin, receveur) ;
- codeparsers d3c077c : un attribut de compilation n'est pas une
  référence ; un type à chemin garde son chemin ;
- branche `self-types` (5236059bf, code.rs, d'accord avec l'arbre
  principal) : par `self`, le type englobant d'abord, sans exclure
  l'héritage — `record_snapshot_finish` revient dans Liens.

- codeparsers 3f992e4 (palier 2) : le type d'un receveur Rust se lit à
  travers une chaîne std (verrous, unwrap, itérateurs) — gain faible
  (9 907 → 9 798), le volume est en différé (champs, retours) : palier 3
  proposé.

- codeparsers 4abafd2 (palier 3) : champs et retours pelés par leur chaîne,
  constructeurs enveloppants, liaisons à leur position et motifs
  `Some` / `Ok`, lecture de champ Rust écartée. **Le critère tenu** : avec
  le filtre dans `impact`, « relie » 1,00, `e2e_impact` vert — fusionné
  par l'arbre principal (pointeur 04436e1e4, filtre 4351d4567), onze suites
  vertes sur le moteur de 22:37. Exposé au défaut des chaînes faussées au
  point de reprise jusqu'à son correctif ; chiffres à rejouer de zéro
  après lui.

Sonde `tests/sonde_marques.rs` : les « nom » passent de 11 769 à 9 210 ;
`tests/sonde_racines.rs` dit d'où partent les receveurs sans type. Le
reste, par forme : nom seul 28 % (ticket, pas maintenant), chemins de
module 20 % (réexportations — décision à prendre), receveurs variables et
chaînes 33 % (palier 2, à ouvrir), dont 2 285 visent un nom de méthode std.

**tree-sitter-scss pour le bâti Windows** (10 octobre, chantier G) : la
seule version publiée imposait à `cl` un drapeau GCC (D8021). Copie locale
dans codeparsers (`vendor/tree-sitter-scss`, MIT, VENDOR.md), build.rs en
`flag_if_supported` et `-utf-8` pour une cible MSVC, comme tree-sitter-css ;
dépendance par chemin, aucun `[patch]` chez rag3weaver. codeparsers 4c7897c,
pointé sur master de rag3db en f9aa29a28 ; compilé sans un refus sous MSVC
14.51 par l'essai Windows de l'optimiseur.

## Ce qui attend quelqu'un

- **Orchestration ou Lucie** : un chemin de module désigne-t-il aussi son
  dossier (réexportations) ?
- **Moi, après le correctif du défaut des chaînes** : rejouer le banc des
  relations et les sondes sur une indexation de zéro.
- **Ensuite, quand l'arbre principal est libre** (l'orchestration relance) :
  les réexportations — ticket « un chemin vers une réexportation reste par
  le nom », environ un jour, un changement de schéma dans code.rs. En
  ticket aussi : « des receveurs restent sans type », « les nom seul et
  segments de module ».

**Arrêt de la session ici (4 octobre, 22 h)**, sur demande de
l'orchestration.
- **Arbre principal, à proposer** : un appel par chemin vers un type
  externe (`Tokenizer::from_file`) prend rendez-vous avec le seul
  `from_file` du projet (`gcp_auth.rs`) : vu par les liens sur les
  `from_bytes` des embarqueurs. Même voie que les champs, autre cas.

## Reprendre

Deux dépôts :

- rag3db, worktree `/home/lucied/git_workspaces/rag3db-codeparsers` ;
  branches à moi : `liens` (garée), `codeparsers-pointeur-8` et
  `codeparsers-rendez-vous-champs` (fusionnées, à supprimer quand Lucie le
  dit).
- codeparsers, dans le sous-module ; `master` = `d51e933`, pointé par
  rag3db master. Le clone du sous-module prenait l'adresse pro :
  `git config user.email` à vérifier dans tout clone neuf.

Bâtir et tester depuis le worktree, contre la lib de l'arbre principal
(jamais la reconstruire) :

```
L=/home/lucied/git_workspaces/rag3db/build/lecteurs-csv/src
RAG3DB_SHARED=1 RAG3DB_LIBRARY_DIR=$L RAG3DB_INCLUDE_DIR=$L LD_LIBRARY_PATH=$L \
RAG3DB_ROOT=/home/lucied/git_workspaces/rag3db \
CARGO_TARGET_DIR=$HOME/.cache/rag3weaver-build/codeparsers-rw \
LUCIVY_SCHEDULER_THREADS=8 \
RAG3WEAVER_EMBED_SERVICE=127.0.0.1:7979,127.0.0.1:7980,127.0.0.1:7981 \
cargo test -j8 --features rag3db-native,code --test e2e_liens -- --include-ignored --test-threads=1
```

Pièges rencontrés :

- **Un target partagé entre deux révisions** a rendu un faux « 0 arête
  retirée » : un target par révision comparée.
- Un e2e sans `#[ignore]` ne tourne jamais (`run_e2e.sh` passe
  `--include-ignored`).
- L'outil Bash est zsh : une variable non citée ne se découpe pas — un
  `git commit $P` n'a rien commis. Chemins en clair, ou script bash.
- Un HEAD détaché dans le sous-module a perdu un commit : `checkout -B` sur
  le hash.
- Un heredoc non protégé mange les barres obliques inverses d'une chaîne de
  test : passer par l'éditeur.
- `e2e_code` embarque pour de bon : par le service distant, jamais par un
  démon local.
- Le banc des relations indexe `src/` : éditer un fichier de `src/` change
  ses notes.
- **Les travaux lourds passent par le verrou** :
  `~/.cache/rag3weaver-build/poste lourd <cmd>` (bâtir, e2e, banc, sonde du
  dépôt), `poste mesure <cmd>` pour une durée ou une mémoire.
- `test_backend_code.py` depuis le worktree veut l'extension vector : lien
  symbolique `extension/vector/build` vers celle de l'arbre principal (non
  suivi, ne pas commettre), et un binaire plus récent que les gabarits.
- **Mesurer l'analyseur** : `tests/sonde_liste_analyse.rs` écrit la liste
  exacte des fichiers de l'index (`SONDE_RACINE`, `SONDE_LISTE`). Les sondes
  de codeparsers (`examples/sonde_*`) se bâtissent en release, un target par
  révision : `sonde_analyse` (durée, CPU, fils actifs, pic mémoire, un appel
  ou paquets de 64), `sonde_fichiers` (les plus lents), `sonde_relations` et
  `sonde_fichier_seul` (relations selon le paquet), `sonde_empreinte` (sortie
  canonique de chaque fichier, pour prouver « identique »).
- **L'arbre principal est vivant** : d'autres sessions y changent de
  branche. Une comparaison avant / après se fait sur la même liste, dans le
  même créneau.
- **Un commit par chemins listés peut oublier un fichier** : `d61b83a`
  annonçait un correctif absent du commit (resté dans l'arbre de travail, où
  tournaient mes vérifications). `git status` vide après le commit, tests
  rejoués sur l'état commité, avant d'annoncer un hash.
