# Session codeparsers — rapport

Tenu à jour sur place. Dernière mise à jour : 4 octobre 2026, nuit (après le banc du résolveur unique).

La session tient l'analyseur (dépôt `L-Defraiteur/codeparsers`, sous-module
`extension/rag3weaver/codeparsers`) et, côté rag3weaver, les outils qui
lisent le graphe du code : `usages`, `impact`, la lecture du catalogue, le
banc des relations, et la section « Liens » en cours. Elle ne touche ni
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

**Le résolveur unique, en fusion** : codeparsers `fichier-seul` @ `d567e7c`,
intégré par l'arbre principal (branche rag3db `resolveur-unique`), batterie
complète en cours chez lui. Banc des relations : « dépend de » 1,00 / 0,64
(master) → 1,00 / 0,85 ; les autres questions inchangées ; égalité des
graphes entre paquets de 1, 64 et un seul tenant, 3 passes sur 3. Correctifs
de la nuit, tous dans `fichier-seul` :
- un accès de champ ne vise plus une méthode homonyme ; une fermeture exclut
  ce qu'elle capture (`de76555`) ;
- un type de champ ou de retour déclaré dans le fichier est lu dès
  l'analyse — le rappel perdu `self.dialect.drop_vector_index()` (`d61b83a`) ;
- en Rust, un `use` qui part d'un nom du fichier n'est pas une bibliothèque
  (`d567e7c` — oublié dans `d61b83a`, voir les pièges).

**Les paires `.h` / `.cpp`, partie codeparsers faite** : branche
`declarations` @ `44c4310` (au-dessus de `fichier-seul`). Une méthode
déclarée est un membre de sa classe (6 395 sur le dépôt), une fonction
déclarée un membre de son namespace (1 100) ; les noms déclarés ne sont plus
des références (−61 598) ; `kz::Foo::qux` et `Foo::~Foo` ne sont plus des
`AnonymousFunction` (2 476 → 738). Reste côté rag3weaver : HAS_PARENT de la
définition vers sa classe (arbre principal), « déclaré foo.h:5 » dans
`usages` (moi) — après la fusion de `fichier-seul`, pour ne pas mêler les
mesures.

**Ensuite**, la section Liens (garée, en attente de Lucie).

**(B) La section « Liens »** — branche `liens` (`1ff5a1dcd`), pas sur master.

- `graph_walk.rs` : le moteur commun (sens, saut nu, degrés), extrait du
  voisinage, qui s'en sert désormais.
- `LinksNode` : marche en largeur à plusieurs départs, les deux sens, un
  carrefour (degré > 50) ni traversé ni point de rencontre, cinq liens au
  plus, texte vide quand rien ne se relie.
- `templates/tools/links.mmd` : gabarit pour le crochet after de la session
  recherche, qui passe `result_uuids` (accepté et écrit par elle :
  `"after": {…, "results_port": {"node": "render", "port": "results"}}`).
- Oracle : sans plafond, 66 paires sur 66 de la longueur de `SHORTEST`, et
  chaque arête rendue existe dans son sens.

**Le constat qui attend Lucie** : à quatre sauts, le graphe de code relie
presque toute paire de résultats par des utilitaires sous le plafond
(`execute_with_params`, `as_f64`) — « … et 27 autres paires reliées ». À
deux sauts (lien direct, ou un intermédiaire partagé), les lignes restent
lisibles et une recherche dispersée ne rend qu'un lien ou rien. Rendus
réels : `e2e_liens::trois_rendus_reels`, `LIENS_MAX_HOPS=2` pour comparer.

**(C1) Le boost de cohésion** : pas commencé. Feu vert de la session du
banc de l'étage sur le principe ; référence à jour : 0,405 / 13 / 26
(granite-278m, doc `22h40/01` de la pile). L'annoncer à l'orchestration et
à elle avant de jouer, régime doux.

## Ce qui attend quelqu'un

- **Lucie** : la longueur des liens (2 ou 4 sauts) et les trois exemples.
- **Arbre principal** : un seul pointeur pour `ace6fd8` (parent homonyme)
  et `fichier-seul`, après la preuve au banc.
- **Arbre principal, à proposer** : un appel par chemin vers un type
  externe (`Tokenizer::from_file`) prend rendez-vous avec le seul
  `from_file` du projet (`gcp_auth.rs`) : vu par les liens sur les
  `from_bytes` des embarqueurs. Même voie que les champs, autre cas.
- **Session recherche** : brancher le crochet `links` dans le manifeste
  (`after`, `results_port`, `max_lines`) quand la longueur est choisie.

## Reprendre

Deux dépôts :

- rag3db, worktree `/home/lucied/git_workspaces/rag3db-codeparsers` ;
  branches à moi : `liens` (garée), `codeparsers-pointeur-8` et
  `codeparsers-rendez-vous-champs` (fusionnées, à supprimer quand Lucie le
  dit).
- codeparsers, dans le sous-module ; `master` = `ace6fd8`, pointé par
  rag3db : `56d039f` ; branche `fichier-seul` = `2c17314`. Le clone du sous-module prenait l'adresse pro :
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
- **Les mesures lourdes se prennent une à la fois** : l'annoncer à
  l'arbre principal et à la session embarquements, attendre leurs deux
  « libre », dire « fini » en rendant.
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
