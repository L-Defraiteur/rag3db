# Journal des chantiers

**À quoi il sert.** Plusieurs sessions (Claude Code, Codex) travaillent sur ce
dépôt, sur plusieurs machines. Ce fichier tient la liste de ce qui est ouvert,
pour qu'un travail non fusionné, non poussé ou en attente d'une décision ne
soit pas oublié. Il ne recopie pas ce que git sait dire : il porte ce que git
ne dit pas.

**Les deux règles.**
1. Une session qui ouvre une branche, ou un chantier hors de ce dépôt, ajoute
   sa ligne ici dans le même commit que son premier travail.
2. Elle met sa ligne à jour quand elle dit « prêt », quand c'est fusionné, et
   quand elle s'arrête en laissant quelque chose.

**La partie qui se recalcule.** Avant de se fier au tableau, relancer
l'inventaire ; s'il contredit le journal, c'est le journal qui a tort.

```bash
git fetch --all --prune
for b in $(git for-each-ref --format='%(refname:short)' refs/heads); do
  up=$(git rev-parse --abbrev-ref "$b@{upstream}" 2>/dev/null)
  echo "$b | derrière/devant master : $(git rev-list --left-right --count master...$b) \
| non poussés : ${up:+$(git rev-list --count $up..$b)}${up:-pas d'amont} | $(git log -1 --format=%cs $b)"
done
git stash list ; git worktree list ; git status --short
```

Dernière mise à jour : **1er octobre 2026**, après la fusion de `mtg-experiments`.

**À lire avec ce journal** : la réconciliation des objectifs et le knowledge
dump du 1er octobre 2026, dans
`extension/rag3weaver/docs/1-octobre-2026-22h47/` — ce qu'on voulait, ce qui
existe, ce qui est en suspens, et l'ordre proposé. Le rapport de session du même soir
(`05`) dit ce qui est en vol et comment le contrôler.

## 1. Branches ouvertes

- **10 octobre 2026, 20 h 30 — cœur C++, marche A3′ sur master** (`21b6cb370`, `79b7be75f`, `9a57a17b2`) : verrou de clé à l'insertion, verrou d'index du COPY pris avant l'ordonnancement, unicité contre le dernier état validé (une seule visibilité), fil de remplacement de l'ordonnanceur pendant une attente ; sous le mode multi-écrivains seulement, éteint hors du banc ; known_red 59 → 56 ; liste complète verte sur luciepc. Suite : A4′. Rapport : `extension/rag3weaver/docs/3-octobre-2026-23h31/coeur-cpp/01-rapport-de-session.md`.

**Arrêt du 2 octobre 2026, 01 h 25** : toutes les sessions ont cessé de
travailler ; chacune a rendu un rapport et un knowledge dump dans
`extension/rag3weaver/docs/2-octobre-2026-01h07/<sujet>/`. **Pour reprendre :
`…/orchestration/01-rapport-de-session.md`, §6.** `master` est à jour, vert,
l'arbre principal est sur `master`.

Branches encore en cours, toutes poussées (les noms
`rag3db-xx` changent à chaque relance : se fier au sujet) :

| Branche | Arbre | Session (sujet) | État au 2 octobre, 01 h 25 | Premier geste |
|---|---|---|---|---|
| `pas-c-ponderations` (`130984f61`) | `../rag3db-pas-c` | recherche | Quatre tests de l'ordre de priorité écrits, logique à coder. | Coder les étapes 1 et 2, arrêt avant fusion. |
| `regime-carte-partagee-2` | `../rag3db-embarquements` | embarquements | **3 octobre** : régulateur de rafale (durée visée + pause, défaut **provisoire** 50 ms / 150 ms) dans l'ingestion et le démon, écran ménagé par défaut quand la seule carte le porte, `CardClass::Integrated` dite par le pilote, troisième déclencheur de 107m, et **`RAG3WEAVER_EMBED_SERVICE=127.0.0.1:7979,127.0.0.1:7980,127.0.0.1:7981`** : s'attacher aux trois services d'embarquement de luciepc (granite-278m, bge-m3, granite-107m), tests compris. Batterie complète par le service : lib 1 134, e2e 374, 0 rouge ; scripts Python verts. **Ne jamais pointer `RAG3WEAVER_EMBEDDINGS_ADDR` sur un tunnel** (les suites arrêteraient le service) ; deux arbres qui jouent des e2e en même temps prennent chacun leur port local par cette variable. Page : `extension/rag3weaver/docs/3-octobre-2026-14h26/02-le-service-d-embarquement-sur-l-autre-poste.md`. | Lucie choisit le défaut (rafale, pause) sur deux ou trois couples mesurés devant l'écran. |
| `index-en-fond` (fusionnée) | `../rag3db-embarquements` | embarquements | **3 octobre** : « indexer ce dépôt » sur master — `estimate` et `index` (`src/estimate.rs`, `src/dataflow/index_nodes.rs`, `templates/tools/estimate.mmd` et `index.mmd`), l'avancement (`Catalog::index_progress`). `index` rend un journal au contrat de `run_bg` ; dernière ligne `indexation terminée`. Proposition et mesures : `extension/rag3weaver/docs/3-octobre-2026-14h26/03-indexer-ce-depot.md` (ce dépôt : 1 798 s pour être cherchable par mots, 710 s de vecteurs). **Point ouvert — la recherche attend pendant le premier temps** : `spawn_index` tient le verrou du catalogue (`Arc<Mutex<Catalog>>`) pendant tout `sync_source`, qui prend `&mut Catalog` ; une recherche du même processus n'entre qu'à la ligne « plein texte prêt ». Pendant les vecteurs le verrou est rendu entre deux passes de 512. La promesse « la recherche par mots répond pendant que ça indexe » demande que `sync_source` rende le catalogue entre deux paquets. | Troisième segment « relations » de l'avancement, quand le chemin de masse de la session de l'arbre principal est livré (marque `relations_pending:{cellule}:{source}`). Lever le point ouvert. Décisions de Lucie : seuil de confirmation (5 min proposées), défaut (rafale, pause). |
| (services sur luciepc) | — | embarquements | **3 octobre au soir, décidé par Lucie** : le modèle de décision JevK5-4B est servi de luciepc comme les granite, parce qu'il prend deux secondes par décision sur processeur ici et 109 ms sur la carte libre là-bas. Par llama-server (binaire déjà présent, rien bâti), `127.0.0.1:7881` là-bas, tunnel `127.0.0.1:7982` ici ; la carte tient les quatre modèles (18 Gio sur 31). Procédure et relance : `extension/rag3weaver/docs/3-octobre-2026-14h26/02-le-service-d-embarquement-sur-l-autre-poste.md` § 7 bis. **22 h 30** : un petit modèle de langage à outils (Qwen2.5-7B-Instruct Q4_K_M, Apache-2.0) y est servi aussi, `127.0.0.1:7882` là-bas, tunnel `127.0.0.1:7983`, pour la passe « agent faible » du backend de code — ce poste n'a que des modèles de 60 Go (§ 7 ter). La carte porte 24 Gio sur 32. | L'abstraction « un modèle, en service ou en local » (proposition en cours) : la décision en sera le premier client. |
| `modeles-declares` (fusionnée) | `../rag3db-embarquements` | embarquements | **3 octobre au soir, demandé par Lucie** : « un modèle, en service ou en local » — une déclaration commune à toute capacité, pour que l'OCR ou la voix se servent comme l'embarquement sans refaire la tuyauterie. Proposition : `extension/rag3weaver/docs/3-octobre-2026-21h22/01-un-modele-en-service-ou-en-local.md`. Décidé par défaut (page § 6) : clé `models` au manifeste, variables `RAG3WEAVER_SERVICE_<CAPACITÉ>` (`RAG3WEAVER_EMBED_SERVICE` gardée comme alias), repli `refuse` sauf si `fallback: local` est écrit. **Lot 1 sur master** : `src/model_source.rs` (déclaration, résolution, origine du calcul), embarquement dense migré (`models.embed` ou `embeddings`, l'une ou l'autre), fournisseur `local` pour le backend. **Lot 2 sur master** : la décision — `src/decider.rs` (`Decider` : un texte déjà composé, des options d'un seul jeton, une probabilité par option ; fournisseur `llama_server`), déclarée par `models.decide`, donnée aux nœuds sous la clé de service `decider`. Elle ne sait rien de la formulation, qui reste au graphe. Vérifié sur le service de luciepc : 117 ms. **Lots 3 et 4 sur master** : `models.sparse` (creux et dual branchés dans le backend, signal refusé s'il n'est pas déclaré), `models.rerank` et `models.ocr` en local. **Lot 5 sur master** : le démon d'embarquement porte aussi un relecteur et un OCR (`RAG3WEAVER_RERANK_MODEL`, `RAG3WEAVER_OCR_MODEL` à son lancement ; routes `/rerank` et `/ocr` ; son identité les déclare) — un seul démon à relancer sur le poste qui sert. | Les lancer sur luciepc avec le démon bge-m3. Lot 6 (LLM) avec la session qui tient le chat. |
| `codeparsers-genre-d-usage` (sous-module `codeparsers`, dépôt `L-Defraiteur/codeparsers`) | `../rag3db-codeparsers` | codeparsers | **3 octobre** : pointé par rag3db jusqu'à `9a1fae3` ; transport du genre en base sur master (`adccf8d62`). **Pile en attente chez la session de l'arbre principal, à prendre dans l'ordre** : `codeparsers-pointeur-4` (`1f9d653`, fonctions de `mod` Rust en scopes : +415 scopes sur `src/dataflow`, sans texte en double ; livré seul pour que son effet soit attribuable) → `codeparsers-pointeur-5` (`137b9d2`, `ScopeInfo.test`) → `codeparsers-champs-test` (`b6e7db858`, `test_role`/`test_certainty`/`test_name` sur `Scope`, vides et non nuls) → `codeparsers-pointeur-6` (`594fc61`, scopes TS/JS pour `describe`/`it`/`test`, séparé puisqu'il crée des scopes) → `codeparsers-pointeur-7` (`a569417`, imports Rust/Python/C++ et `USES_LIBRARY` : 698 sur `src/dataflow`, `CONSUMES` quasi inchangés). Sur master de codeparsers, **hors pointeur** (consigne : un seul pointeur de plus, quand la pile sera vidée) : `56d039f`, appels typés par un champ ou un retour déclaré (`CONSUMES` +51/−3 sur `src/dataflow`, 25 relus sans faux). Propositions code.rs chez l'arbre principal : `codeparsers-champs-test`, `codeparsers-mentions-type` (une mention typée choisit sa méthode parmi les homonymes). La pile est bloquée par l'assertion « merge_port_values en tête » : les tests de `mod` devenus scopes passent devant la définition en plein texte ; déblocage par le poids de `test_role` (session recherche), la règle « nom exact en tête » attend Lucie. Limite à garder : « local » dépend du corpus analysé (porte de la vision des dépôts reliés). Docs `extension/rag3weaver/docs/3-octobre-2026-20h30/` 01 à 07. **Outil `usages` sur master (`551ed2d1f`)** : nœud générique `UsagesNode` (pivot déclaré par le gabarit), gabarit `templates/tools/usages.mmd` ; pour le code, union du rendez-vous (`MENTIONS`, usages entre fichiers et noms ambigus) et des arêtes directes (`CONSUMES`…, usages du même fichier) — ni l'un ni l'autre seul ne suffit ; un nom ambigu montre tout et range à part les usages non attribués ; requêtes en liste simple étiquetée, `EXPLAIN` sans produit cartésien testé. Attachement aux manifestes et politique : session recherche (fait). **Outil `impact` sur master (`c6065b35e`)** : nœud générique `NeighborhoodNode` (niveaux, budget, plafond de degré, regroupement par champ, départ supplémentaire par un chemin déclaré) et gabarit `impact.mmd` (tests qui traversent, groupés par `test_role`). Un carrefour n'est pas traversé vers le code, mais ses tests sont relevés — c'est ce qui retrouve les tests cassés par trois changements réels de ce dépôt (row_to_map, mark_snapshot, split_unchanged). Règle commune `catalog_read.rs` : un outil de lecture ne rend jamais un vide sans dire d'où il vient (occupé, jamais indexé, partiel). Trouvé en route, corrigé dans codeparsers `ace6fd8` (sans pointeur, après le 8) : le parent d'une méthode Rust était la struct homonyme, pas son impl (520 enfants sur `src/dataflow`). | 1) Reprise de la pile. 2) Imports Go et C# ; chemin complet des imports locaux. 3) `qualifier_type` sur la voie MENTIONS de code.rs. Hors cadrage : le type des liaisons de motif et des paramètres de fermeture (inférence). |

**Livraison du cœur C++, 10 octobre 2026 — la condition 2 de la stèle est fermée.** `master`
a reçu en avance rapide, depuis `../rag3db-moteur` : `0aed3c4b5` (le fichier de la base a une
étendue connue : les pages d'un `COPY` tué ou rejoué ne sont plus perdues ; **version de
stockage 40** — une base touchée n'est plus lisible par un moteur d'avant, une base d'avant
s'ouvre et acquiert son étendue au premier point de reprise) et `ff9bad960` (**le `COPY`
journalisé est le défaut du moteur** : `force_checkpoint_on_copy=false` ; plus de point de
reprise forcé par `COPY`, plus d'attente du départ des autres à sa validation ; au-delà de
256 Mio de journal par transaction, repli sur le point de reprise forcé ; l'indexation avec le
plein texte en base demande elle-même `CALL force_checkpoint_on_copy=true`). Toute lib se
rebâtit sur ce master. Preuves : la liste complète verte sous ce défaut, contrôle de fuite de
pages compris ; la série de confirmation des embarquements (fichiers 80 → 81 s, blobs 89 → 87 s,
`…/8-octobre-2026-16h29/embarquements/04-la-serie-de-confirmation.md`). Pages :
`extension/rag3weaver/docs/3-octobre-2026-23h31/coeur-cpp/04`, `05`, `06` ; la stèle §2.
Prochain lot du cœur C++ : les verrous (page courte d'abord), puis les écritures parallèles.
**Confirmation produit (arbre principal, 10 octobre au soir)** : la batterie rag3weaver complète
sous ce défaut, **74 suites vertes, aucun rouge**. Conditions : lib de 17 h 27, moteur
`aeec6888f` (`ff9bad960` dedans, sans `d10b92306`), `RAG3WEAVER_MOTEUR_ANCIEN=1` dit, hors carte
locale, régime doux. Écartées : `e2e_postgres` (pas de conteneur), les cinq suites `openai-llm`,
et les suites de la carte locale. Jouée en deux parties autour du nettoyage du disque ; rapport
`extension/rag3weaver/docs/10-octobre-2026-arbre-principal/02-rapport-de-session.md`.

**Livraison du cœur C++, 3 octobre 2026** : `master` a reçu en avance rapide dix commits
(`81f8982ea..13284a0fe`), livrés par la session cœur C++ depuis son arbre
(`../rag3db-moteur`), à la demande de l'orchestration.

| Ce qui est entré | Commits |
|---|---|
| `c_api_test` : deux tests faux depuis le renommage du fork (le moteur était juste) | `3bc404937` |
| **Marche T0** : après une erreur dans `BEGIN … COMMIT`, tout est refusé jusqu'au `ROLLBACK` (`TransactionContext::MANUAL_TRANSACTION_FAILED`), erreur de syntaxe comprise | `88f57e2ba`, `4c13619d1`, et `a34538c9a` pour trois cas de l'extension vector |
| **Marche A2** : les offsets provisoires des nœuds d'une transaction sont remappés au commit ; les insertions sont journalisées au commit ; au commit, un offset définitif n'est plus pris pour une ligne locale | `e81423a28`, `875da1772` |
| Un champ `NULL` d'une liste de structures littérales prend le type des autres éléments | `17c1ae41d`, `909c2573c` |
| Index vectoriel : un `DELETE` annulé par `ROLLBACK` ne le laisse plus muet ; un survivant vers lequel ne menaient que des nœuds supprimés reste atteignable | `e64839489`, `13284a0fe` |

Passe de livraison, sur l'empilement tel qu'il est entré : `transaction_test` 74/74,
`api_test` 104/104, `c_api_test` 136/136, `copy_tests` 19/19, stockage 77/77, batterie
Cypher complète 1866/1866, extension vector 74/74 sur disque et 63/63 en mémoire,
`concurrence_test.known_red` vert (36 rouges connus, 24 verts : les lignes de T0 et de C3
retirées, rien d'autre n'a bougé) ; côté Rust, lib 1135 passés et 8 ignorés, quinze suites
e2e (212 tests), les binaires, neuf scripts Python. Non joués : `e2e_postgres`, les cinq
suites qui appellent un vrai modèle, les suites e2e hors de la liste de livraison, et les
tests des extensions fts et httpfs, qui ne sont dans aucun build.

**`c_api_test` entre dans la liste de livraison de toute marche du moteur**, avec les
tests de l'extension vector (disque et mémoire) : c'est la passe de livraison qui a vu
que T0 touchait trois de ses cas.

**Ce que T0 change pour un client** : après une erreur dans un bloc `BEGIN`, il doit
envoyer `ROLLBACK` avant de continuer. Dix cas de test existants ont reçu ce `ROLLBACK`.
Les tests des extensions fts et httpfs ont aussi des blocs `BEGIN` : T0 les touchera le
jour où on les bâtira.

Les branches `t0-transaction-en-echec`, `test-c-api-octets-magiques`,
`a2-remappage-des-offsets-au-commit`, `champ-nul-d-une-liste-de-structures`,
`hnsw-point-d-entree-apres-rollback`, `refus-apres-point-de-reprise-echoue` et
`hnsw-point-d-entree-apres-suppression` sur `origin` sont des états d'avant
l'empilement : à supprimer par Lucie.

L'étude de l'index vectoriel sous plusieurs écrivains :
`docs/3-octobre-2026-15h47/02-hnsw-sous-plusieurs-ecrivains.md`. Elle propose de faire
toute la maintenance de l'index au commit (environ trois jours) ; en attendant, une table
indexée ne devrait pas recevoir deux écrivains à la fois. Le banc l'a confirmé depuis :
deux suppressions de nœuds voisins dans le graphe se heurtent sur « Write-write
conflict » dix fois sur dix, et son mélange aléatoire sur une table indexée corrompt le
tas.

**Marche A5, livrée le 3 octobre 2026** (`1175cc0a2`, `e415e0277`, en avance rapide,
push autorisé par Lucie). Les informations de version d'un bloc de lignes et l'index des
relations validées en mémoire étaient lus et écrits sans garde entre fils ; ils passent
sous un verrou partagé/exclusif, qu'un drapeau atomique épargne au lecteur tant qu'un bloc
n'a jamais eu d'informations de version. **Ce n'est pas propre au mode multi-écrivains** :
un seul écrivain qui supprime pendant que d'autres connexions du même processus lisent
suffit — le témoin `test/transaction/readers_during_delete_test.cpp` sort 6 avertissements
ThreadSanitizer sans A5, aucun avec. La course est prouvée dans ce régime, le plantage
non ; les plantages mesurés (`isSelected`, `isDeleted`) viennent des cas à deux écrivains
du banc. C5 n'est plus probabiliste (900 passes vertes), `tsan_signatures.txt` est vide :
toute signature de C5 ou C6 est un rouge.

Passe de livraison : la liste C++ complète sur la marche avant son rebase
(`transaction_test`, `api_test` 104, `c_api_test` 136, `copy_tests` 19, stockage 77,
Cypher 1866, vector 74 et 63) ; sur l'empilement, le banc (39 rouges connus, 33 verts) et
`transaction_test` 75/75 ; côté Rust, lib 1135, quinze suites e2e (212 tests), binaires,
neuf scripts. Le dernier rebase n'a apporté que du Rust et des docs (sources C++
identiques, vérifié par diff) : rien n'a été rejoué dessus, la passe Rust d'A5 bis le
couvrira. Coût sur nos chemins : rien au temps mur d'une ingestion (93,7 s contre 93,6 s
sur `e2e_mesure_ingestion_code`), quelques pour cent au plus sur la part du moteur, du
même ordre que le bruit de deux passes.

**Un faux vert à éviter dans la liste de livraison** : `ctest -R
concurrence_test.known_red` lancé depuis la racine du build ne trouve aucun test et sort
0. Lancer la comparaison directement :
`cmake -DBENCH=<concurrence_test> -DKNOWN_RED_FILE=… -DPROBABILISTIC_FILE=… -DRESULT_FILE=… -P test/transaction/concurrence/compare_known_red.cmake`,
et lire la ligne « bench matches known_red.txt ».

H3 est inchangé par A5 (rouge 10 sur 10). H4 aussi reste rouge, pour d'autres causes :
voir A5 bis ci-dessous et le §6.

**Marche A5 bis, livrée le 3 octobre 2026** (`7487fae08`, en avance rapide). Un seul
écrivain et une recherche vectorielle suffisaient : la recherche HNSW lit les vecteurs
sans le verrou du groupe (`NodeTable::lookup<false>`,
`extension/vector/src/index/hnsw_graph.cpp`) pendant que le commit de l'écrivain ajoute au
dernier bloc en mémoire, allonge la liste des blocs et réalloue le tampon d'une colonne à
longueur variable — le lecteur lisait de la mémoire rendue. Même défaut dans
`NodeGroup::isVisibleNoLock` (toute recherche par clé primaire) et dans le balayage des
relations validées en mémoire. C'est le régime du démon pendant une ingestion.
`NodeGroup` reçoit une garde lecteurs/écrivain (`inMemAppendMtx`), exclusive pendant
l'ajout, partagée dans ces trois chemins. Le témoin
`test/transaction/vector_search_during_insert_test.cpp` sort 18 avertissements
ThreadSanitizer avant, aucun après ; il se saute si l'extension vector n'est pas bâtie.
La course est prouvée à un seul écrivain, le plantage seulement à plusieurs (H4).

Passe de livraison : la liste C++ complète (`transaction_test` 76, `api_test` 104,
`c_api_test` 136, `copy_tests` 19, stockage 77, Cypher 1866, vector 74 et 63, banc
39 rouges connus et 33 verts) ; ThreadSanitizer à zéro sur C5 et C6 (40 répétitions), sur
C7 — ses deux signatures étaient celles du balayage des relations en mémoire — et sur les
deux témoins ; côté Rust, lib 1177 passés et 9 ignorés, dix-huit suites e2e (les quinze de
la liste, plus `e2e_estimate`, `e2e_code_sync`, `e2e_working_tree` : 245 tests), les
binaires, neuf scripts Python. Les quatre derniers commits de master (Rust seulement,
sources C++ identiques, vérifié par diff) sont entrés après cette passe : rien n'a été
rejoué dessus.

**Un faux vert à éviter dans la passe Rust** : les binaires demandent maintenant la
feature `code` (`cargo build --features daemon,rag3db-native,openai-llm,code`). Sans elle
la construction échoue, et les scripts Python tournent quand même, sur les binaires de la
passe précédente : lire le code de sortie de la construction avant de croire leurs « PASS ».

**Marche A5 ter, ouverte, non commencée : une garde qui ne coûte rien au lecteur.** La
garde simple a été livrée **sans** la mesure de son coût sur nos chemins au calme — décision
de l'orchestration, sur délégation de Lucie : une lecture de mémoire libérée atteignable
en service ne reste pas ouverte pour attendre un chiffre que le poste ne pouvait pas
donner ce soir-là (six sessions y travaillaient).
- Mesuré, dos à dos, sur la recherche vectorielle seule (dix mille vecteurs, `efs` 200,
  2 000 recherches par fil, médiane de cinq passes, charge du poste entre 6 et 7) :
  dimension 8, un fil 2,02 s → 2,07 s (+3 %), quatre fils 3,44 s → 4,03 s (+17 %) ;
  dimension 256, un fil 9,67 s → 10,29 s (+6 %), quatre fils 13,84 s → 16,79 s (+21 %).
- Mesuré sur nos chemins, sous charge, à lire sans conclure : `e2e_search` 3,73 s avec la
  garde (charge 9) contre 3,85 à 3,98 s avant ; une passe d'ingestion avant/après faite
  sous une charge de 20 est inexploitable (l'embarquement, que la garde ne touche pas, y
  passe de 94 s à 144 s).
- **Non vérifié** : le coût sur `e2e_search` et sur l'ingestion au calme. À faire une nuit
  ou sur l'autre poste ; scripts dans `~/.cache/rag3db-moteur-notes/a5bis/`
  (`mesure-appariee.sh`, et le brouillon `vector_search_cost_scratch_test.cpp.brouillon`).
- Essayé sans gain mesurable dans ce bruit, à ne pas refaire tel quel : le verrou exclusif
  du groupe (ferme la course mais double le temps de quatre recherches parallèles) ; une
  garde à bandes, une case par fil lecteur (donc le coût n'est pas la dispute d'une ligne
  de cache) ; la garde tenue par lot plutôt que par ligne (patch gardé,
  `variante-par-lot.patch`). Le poste n'a ni `perf` ni `valgrind` : on ne sait pas où part
  le temps.
- La piste : que le lecteur ne prenne rien, et que l'ancien tampon reste en vie tant qu'un
  lecteur peut le tenir. Elle touche `ColumnChunkData` et ses dérivés (code de l'amont) et
  la liste des blocs ; deux à trois jours, estimation non étayée. Si `e2e_search` ne bouge
  pas de façon visible au calme, elle attend derrière les verrous.
- Non examiné : la liste des groupes d'une table (`NodeGroupCollection`), lue sans verrou
  par `NodeTable::isVisibleNoLock` ; elle ne s'allonge que toutes les 131 072 lignes.

**Reprise après arrêt brutal, livrée le 3 octobre 2026 au soir** (session cœur C++, deux
commits en avance rapide, passe C++ complète et passe Rust à chacun) :
- `f5acca417` — un `DROP` d'index rejoué depuis le journal retire aussi l'index de la
  table (§6).
- `fcd9a7882` — **une base ne plante plus à l'ouverture** quand le journal écrit dans une
  table dont l'index n'est pas chargé (§6, garde 1). L'index est alors « en retard » et le
  dit par une erreur nommée, `is behind its table` ; rag3weaver doit le reconnaître à
  l'ouverture, le retirer et le rebâtir (à faire, session de l'arbre principal).

**Livré le 4 octobre 2026** (session cœur C++, `c8fdaf196`, avance rapide, passes C++ et
Rust complètes) : une transaction relit juste les relations qu'elle vient de créer après en
avoir supprimé — défaut du moteur d'origine, atteignable par Cypher, qui laissait aussi des
relations en trop ou en double (§6) ; et la mise à jour d'un vecteur garde ses anciens
voisins joignables dans l'index.

**Deux faux verts à connaître pour tout test de reprise** (trouvés ce soir, par le banc et
ici) : un test qui ferme la base avec son point de reprise final ne rejoue rien — exiger un
journal non vide juste avant de rouvrir ; et un processus qui a déjà chargé l'extension
vector ne voit pas les défauts du rejeu sans extension — rouvrir dans un processus neuf
(`test/transaction/vector_index_crash_reopen_test.cpp` le fait par `exec`).

**Suite de la session cœur C++**, dans l'ordre fixé par l'orchestration :
1. **La garde 2 de la reprise** : que le rejeu ait l'extension avant de rejouer, pour que
   l'index reste juste après une mort au lieu d'être à rebâtir. Le repérage est fait
   (relevé de connaissances de la session, §1) : la liste persistée avec la base demande
   soit de monter la version de stockage, soit un marqueur dans l'en-tête ; l'enregistrement
   réécrit en tête du journal ne touche aucun format mais change l'ouverture en lecture
   seule et une huitaine de tests. Dans le même lot : un journal qui porte un
   `LOAD EXTENSION` dont le fichier a disparu empêche aujourd'hui d'ouvrir la base. Ses cinq
   témoins sont rouges au banc.
2. **La mise à jour massive de vecteurs** (§6) : un contrôle en fin d'instruction sur tout
   ce qu'elle a touché, la dimension 768, les dix mille lignes ; et la ligne lointaine
   injoignable à la construction. Avant V1 seulement si l'invariant « toute ligne à vecteur
   est joignable » rougit en usage réel côté produit ; sinon après. Ladybug n'a ni mise à
   jour ni suppression dans son HNSW : rien à y reprendre.
3. **Les verrous**, tranchés le 3 octobre (§4) : V1 (gestionnaire générique, deux genres dès
   le départ : ligne par clé, index d'une table), A3′, A4′, V2 ; puis **la maintenance de
   l'index vectoriel au commit**, qui n'est plus un affinage mais une marche du plan —
   c'est elle qui rend parallèles les écritures sur une même table indexée.
4. **H4** : une fois les courses fermées, il reste rouge (35 sur 40) — une lecture de
   mémoire libérée au point de reprise (`ListChunkData::append` sous
   `NodeGroup::checkpointInMemAndOnDisk`, un seul fil à ce moment-là, donc un état déjà
   corrompu) et la base qui ne se rouvre pas (« Found duplicated primary key value » au
   rejeu, 21 à 23 fois sur 40). **Correction du 3 octobre au soir** : j'avais écrit que cet
   échec de réouverture n'arrivait que sur une table indexée, parce que C1 et C7 se
   rouvraient. C'était faux : la variante Crash du banc fermait la base avant de tuer le
   processus. Avec un vrai arrêt, une clé en double validée rend la base impossible à
   rouvrir, index ou pas (C1 déterministe, C7 vingt fois sur vingt). C'est le doublon seul ;
   A3′ l'empêchera de naître, et la reprise devra mettre de côté la transaction fautive au
   lieu de refuser d'ouvrir (témoin rouge au banc,
   `LockBench.RecoveryOfAJournalWithADuplicateKeyKeepsTheDatabaseOpen`).
5. Le port de la recherche par clé par ligne de Ladybug (§6, `UNWIND … MATCH`), et la
   marche A5 ter.

Les branches `a5-suppression-sure-entre-fils`, `-2` et `-3` sur `origin` sont des états
d'avant rebase : à supprimer par Lucie. Celles d'A5 bis et des correctifs de reprise sont
restées locales.

Le plan : `docs/2-octobre-2026-00h17/01-ecritures-paralleles-vela-et-le-chemin.md`
(§12, l'ordre des marches) ; côté crate :
`extension/rag3weaver/docs/2-octobre-2026-00h16/01-ce-que-rag3weaver-suppose-d-une-seule-base.md`.

`banc-de-concurrence` est fusionnée dans `master` le 3 octobre 2026 (dernière
branche `banc-de-concurrence-sur-7928974`, en avance rapide, après relecture par la
session cœur C++ et une passe `known_red` verte). **Depuis, `concurrence_test.known_red`
entre dans la liste de livraison de toute marche du moteur.** Les branches
`banc-de-concurrence` (`00b2a0263`) et `banc-de-concurrence-sur-7928974` sur `origin`
sont des états d'avant rebase, laissées sans force : à supprimer par Lucie. Les six
remarques de la relecture sont fusionnées le 3 octobre (`banc-remarques-de-relecture`,
en avance rapide, comparaison `known_red` verte contre le moteur de la marche 1, du
refus après un point de reprise échoué et du correctif HNSW : aucun rouge connu n'a
bougé). Depuis, `known_red.txt` épingle la raison de chaque rouge (étiquettes
`[check: …]`) : une marche qui corrige un cas retire sa ligne dans son commit. La passe
ThreadSanitizer du banc (`concurrence_tsan.signatures`) ne vit que dans un build TSan.
**La course de A5 fait planter le processus** (SIGSEGV dans `VersionInfo::isSelected`
et `isDeleted`, piles dans la spécification du banc, §10). Les cas sur une table indexée
par HNSW sont fusionnés le 3 octobre (`banc-hnsw`, spécification §11) : l'invariant de
l'index (une recherche exhaustive rend exactement les lignes vivantes), l'exécution isolée
dans un fils (un plantage ou une base qui ne se rouvre plus deviennent des rouges nommés),
le build du banc avec `-DBUILD_EXTENSIONS=vector`. H1 (deux insertions de vecteurs) est
vert depuis `b23309848` (A2) ; H3 (voisinages qui se recouvrent) et l'index construit sur
cent lignes qui perd un nœud sont rouges ; H4 est probabiliste (§6). Les témoins des verrous
(note du 3 octobre, §6) sont fusionnés le même soir (`banc-verrous`, spécification §12) :
`lock_bench_test.cpp`, rangés dans `known_red.txt` sous la marche qui doit les rendre
verts (V1, A3′, A4′, V2), avec un rouge distinct quand la fonction n'existe pas encore
(`acquire_locks`, `lock_timeout`) ; le nœud-carrefour est un garde-fou vert.

**Budget de reprise du lecteur en lecture seule : décidé le 3 octobre, il reste
à 250 ms** (`PATIENCE_OUVERTURE_MS`). Le pic à 567 ms mesuré à `20a8f6ee8` venait
de la course que la marche 1 corrige — ses refus étaient comptés sans lire leur
message — et `un_lecteur_qui_insiste_pendant_qu_on_ecrit`, joué dix fois après
la marche, n'a laissé aucun refus survivre au budget. Un refus sporadique qui y
survivrait un jour serait la famine que la marche 5 doit réduire, pas une
incohérence ; le test le dit.

La **marche 1** (le lecteur revérifie à l'ouverture) est fusionnée dans `master`
le 3 octobre 2026, en avance rapide depuis la branche locale `livraison-marche-1` :
le commit de test de `reprise-apres-panne-index-cle-primaire`, les quatre tests
rouges, puis `94f336ef5` et `cb8579761` fondus en un commit `feat(lecteur)`. Passe
de livraison : transaction_test 63/63, api_test 102/102, `LecteursConcurrents.*`
cinq fois vert (0 incohérence, 1 à 2 refus transitoires par passe),
`concurrence_test.known_red` vert, lib 1111, douze suites e2e, sept scripts du
backend. `un_lecteur_qui_insiste_pendant_qu_on_ecrit` (ignoré, non livré) joué dix
fois : 0 refus, 80 lectures sur 80, 0 incohérence à chaque passe. Les branches
`lecteur-reverifie-a-l-ouverture`, `lecteur-reverifie-a-l-ouverture-sur-7928974`
et `reprise-apres-panne-index-cle-primaire` sur `origin` sont à supprimer par Lucie.

`synchronisation-par-perimetre` est fusionnée dans `master` le 3 octobre 2026
(dernière branche `-5`, en avance rapide). Les branches `synchronisation-par-perimetre`,
`-2`, `-3` et `-4` sur `origin` sont des états d'avant rebase, laissées sans force : à
supprimer par Lucie, comme `origin/fin-de-journal-dechiree`.

`origin/fin-de-journal-dechiree` est périmée (son contenu est sur `master`
depuis `a66bb0b9d`, rebasé) : à supprimer par Lucie.

`correctif-wal-enregistrements-longs` a été fusionnée dans `master`
en avance rapide le 1er octobre 2026 (`955b1b136`, `84d783afc`), après
reconstruction de `build/lecteurs-csv` et des suites vertes.

`mtg-experiments` a été fusionnée dans `master` le 1er octobre 2026
(fusion `32d6e0b44`, `master` poussé à `d4f32f9e0`) ; la branche reste sur
`origin`, à supprimer quand Lucie le dira.

Les neuf branches des sessions du 18 septembre (`heuristique-taille`,
`retrait-monolithe-recherche`, `nettoyage-apres-monolithe`,
`doc-dernier-chemin-parallele`, `banc-ponderation`, `banc-ponderation-suite`,
`lifecycle`, `embarquements`, `fts-lucivy-v3`) sont **toutes fusionnées** dans
`master` : elles n'ont rien devant lui et peuvent être supprimées.

## 2. Ce qui est dans git, et ce qui n'y est pas (expérience MTG)

Le dépôt est public. La règle, posée par Lucie le 1er octobre 2026 : **on
pousse tout sauf les données**. Le code des branchements a sa place dans git
même s'il n'est réellement branché que sur le poste.

| Dans git | Hors de git, sur le poste seulement |
|---|---|
| Le moteur, le backend déclaratif, le harnais, le chat, les scripts de préparation et d'ingestion | `experiments/mtga/data/` : bases, snapshots, collection capturée, decks générés, journaux de conversation |
| Le manifeste `backend.json`, les schémas, les graphes, les règles Rhai, le gabarit de rendu | `backend/harness/cards.json` et `wildcards.json` : faits extraits, régénérés par `scripts/prepare_deck_harness.py` |
| Les branchements vers les sources locales : `mtga-reader`, `Player.log`, les SQLite du client Arena | `.venv`, `node_modules`, `rag3bridge/target`, les poids des modèles |
| Les docs des expériences, noms de cartes compris | Les identifiants : `.vault` |

Un chemin absolu du poste traîne dans `experiments/mtga/chat/chat.json`
(`backend_command`) : c'est un branchement local, pas une donnée ; à rendre
relatif le jour où quelqu'un d'autre lance le chat.

À ne pas confondre avec la question des droits, qui reste ouverte avant toute
diffusion d'un produit : `mtga-reader` est sous GPL-3.0 et les conditions de
Wizards ne sont pas clarifiées (`extension/rag3weaver/docs/20-09-2026/15-…`).

## 3. Reste à faire sur du travail déjà fusionné ou poussé

| Chantier | Où | Ce qui reste |
|---|---|---|
| Ligne de statut d'index dans l'application chat | décidé par l'orchestration le 4 octobre (après les rejeux T1 : aucun modèle ne relaie la ligne d'état, même en tête de fiche avec consigne) | Le chat lui-même montre l'état de l'index — une ligne de statut d'application lue par index_state_for, hors du texte de l'agent (« index : en cours, mots prêts, vecteurs 40 % »). Ce qu'un harnais sait, il le dit lui-même. Petite pièce, session recherche, après ses lots 2 et 3. |
| Bac à sable des commandes de l'agent de code | garde durcie le 3 octobre (`commande.rs` : la lecture libre confinée au domaine — trouvé par la passe Gemini, `cat ../backend.json` passait) | **Analyser une ligne de shell n'est pas une frontière de sécurité** : la garde borne l'honnête et l'opportuniste, pas un adversaire (un binaire du dépôt lancé par `cargo test` lit ce qu'il veut). Pour le mode `auto` et tout usage sans humain devant : un bac à sable du processus — la commande ne **voit** que le workspace. Recommandation Linux : Landlock (dans le noyau depuis 5.13, sans privilège, s'applique par le parent avant exec — premier choix), bubblewrap en repli si le noyau est trop vieux ; les deux par une variante d'`Atelier::executer`. À cadrer avant de l'ouvrir au cloud multi-locataire. |
| Repli des KB en entités dérivées | `master`, pas A et B faits | **Pas C** : poids de fusion par entité, pondération par genre dans `Scope`, gabarits de dérivées au catalogue. Décidé (§4) ; **la mesure des poids de fusion ne commence pas** avant la référence du banc `e2e_banc_etage` en granite-278m, avant / après les suppressions — avant = `a66bb0b9d`, après = `01791e347` (45 → 43 questions : les deux sur le parcours en largeur n'ont plus de cible) —, à jouer dès que les poids granite-278m sont sur ce poste. |
| Chemin de masse des lots de naissances | `fa70cf8f3`, désactivé (`RAG3WEAVER_COPY_NAISSANCES`) | Trouver pourquoi le `COPY` des chunks croît avec la table. Pistes : reconstruction de l'index vectoriel à chaque lot (`ajuster_l_index_pour_le_retard`), relecture `select_node_ids`. |
| Deck builder MTG (produit) | `experiments/mtga`, `master` | Descriptions d'outils propres à chaque entité (description d'entité dans le manifeste, au lieu du même texte pour tous les `search_*`) ; compter artefacts et créatures de mana comme sources de couleur dans le harnais (accordé, pas fait) ; barre de défilement du chat dont la taille ne suit pas la liste (capture attendue) ; option Gemini via Vertex. |
| Synchronisation par périmètre | **première étape sur `master` (3 octobre)** : `SnapshotConfig` (périmètre, `maxMissingRatio`, `onMissing: delete \| {transition}`), une session à la fois par périmètre et par cellule (`begin_snapshot` rend l'identifiant, `takeover`, `abort_snapshot`), marque `_snapshot`, fin en deux temps (`plan_snapshot_finish` / `apply_snapshot_finish`) et ses garde-fous, marque d'absence `_absent_since`, schéma v8 ; `tests/e2e_synchronisation.rs`, `scripts/test_backend_snapshot.py`, `scripts/test_migration_v8.py` | **Seconde étape faite (3 octobre)** : la mise de côté avant purge (`keepFor`, 7 jours par défaut, `0` pour la couper ; `_snapshot_aside`, le retour sans réembarquement, la purge bornée par SET) et l'annulation en bloc d'une fin (`undo_snapshot_finish`, outil `undo_snapshot`) — ce qui est codé et ses écarts : §8 de `extension/rag3weaver/docs/3-octobre-2026-15h04/01-mise-de-cote-avant-purge.md`. Le code l'utilise (3 octobre, `src/code_sync.rs`, deux grains emboîtés). Reste : PostgreSQL sans vecteurs mis de côté. |
| Champ `folds` des scopes | `1e5eea234` | Ré-ingérer le code pour le remplir. |
| Base MTG | poste | À reconstruire (environ 8 Go, dont 6 récupérables). |
| Récupération des lignes supprimées dans rag3db | proposé, pas fait | Les blobs d'index sont bornés par une purge côté rag3weaver (`d1aa7d296`) en attendant. |
| Un modèle d'embarquement requalifié en ancien = une transition d'état déclarée | promis le 18 septembre à la session optimiseur, **jamais confié** | Attendait que `Lifecycle` soit appliqué à l'écriture : c'est fait (`780acfd2d`). À cadrer avec les sessions optimiseur et lifecycle. |
| Écritures parallèles | **objectif décidé par Lucie le 2 octobre** : « on fait ce qu'il faut pour écritures parallèles, peu importe ce que ça coûte » | **Cible tranchée par Lucie : « oui jusque B, et A d'abord si dans même chemin »** — B, plusieurs processus écrivains sur le même fichier ; A, plusieurs transactions d'écriture dans le processus qui tient la base, en première étape seulement si elle est sur la route de B. **Lucie, 2 octobre : « l'écrivain doit attendre au lieu d'échouer, sur un lock par clé »** ; et son principe pour tout choix du moteur : **faire comme PostgreSQL et Neo4j, sauf quand on sait faire mieux** (ne lui rendre que les écarts) — les marches A3 et A4 se font donc par verrous (le second attend), pas par simple validation au commit ; note de conception demandée à la session cœur C++ avant tout code (ce qu'on verrouille, ce que fait celui qui a attendu une mise à jour, interblocages, où vit la table de verrous pour B). Plan demandé à la session cœur C++ (lecture seule) : ce que B réutilise de A, un ordre de marches testables une à une, et le lecteur d'un autre processus comme premier cas. Rien à coder avant que Lucie ait vu le plan. État d'aujourd'hui : Le second écrivain est **refusé**, pas mis en attente ; le checkpoint bloque les lecteurs. Ordre écrit le 6 septembre (`docs/6-septembre-2026-13h08/01-…`) : prendre Vela → mettre les écrivains en file → lots courts à l'ingestion → bien plus tard, deux processus écrivains. **Étude en lecture seule confiée à la session cœur C++ le 2 octobre** (Vela aujourd'hui, coût de fusion après nos deux correctifs du journal). Rien de décidé. |

### Dette de généricité : ce qui est câblé pour le code dans le cœur du crate

Relevé le 2 octobre 2026, à la demande de Lucie (« ça ne doit pas faire que
du code »), par lecture des noms de champs — peut-être pas exhaustif. Rien
n'est corrigé ; règle : une organisation se déclare dans `EntityConfig`,
jamais en dur.

- ~~`code_tools.rs`, `reingest_file`~~ — **fait le 3 octobre** : l'entité
  `Scope` déclare sa synchronisation (la source en grain large, le fichier en
  grain fin) et `File` la sienne ; une édition est une fin immédiate sur le
  grain du fichier, `code_sync::sync_source` synchronise une source entière
  et retire enfin les fichiers supprimés (`src/code_sync.rs`). Reste, hors de
  ce chantier : les arêtes sortantes d'un scope gardé ne sont jamais
  nettoyées (un appel retiré du corps laisse son `CONSUMES`).
- `work_domain.rs`, `Selector` : `sources` / `repos` / `languages` / `under`
  deviennent des filtres sur les champs `source`, `repo`, `language` et un
  champ de chemin. Le domaine de travail est un vocabulaire de code, pas un
  filtre sur des champs déclarés.
- `generic_search_nodes.rs` (application du domaine) : champ de chemin choisi
  par heuristique (`file_path`, sinon `path`).
- `render_nodes.rs` : une liste de champs consommés en dur par le rendu
  (`file_path`, `start_line`, `language`, `repo`, `revision`, `docstring`,
  `signature`, `scope_type`…), le choix du titre et la langue de l'extrait.
  `contentKind` et `sourceLines` (27 septembre) en déclarent une partie.
- `code.rs` : l'identité d'une source est un curseur de dépôt.

Conséquence pour le produit : pas d'entité « source » intégrée au moteur. Le
générique est une **session de synchronisation** (début, lots, fin) avec un
périmètre déclaré par l'entité ; un backend qui veut un « dépôt » ou un
« dossier » le déclare comme n'importe quelle entité.

## Audit des `CatalogEvent::Warning` — ce que le catalogue dit à personne

4 octobre 2026, demandé par l'orchestration après le défaut de l'index détaché
(`CATALOGUE=ok` pendant qu'une recherche vectorielle refusait). La classe est
nommée depuis la veille : **une information existe, et rien ne la consulte**.

**Le fait qui donne son poids au tableau : en production, personne ne lit un
`CatalogEvent::Warning`.** Le seul endroit du dépôt qui filtre cette variante
est un test (`src/catalog.rs:9019`), et `Catalog::subscribe()` n'est appelé que
par des tests (`catalog.rs:9045`, `:9070`, `tests/e2e_batch_observe.rs:270`).
Les `runtime.subscribe()` du fichier écoutent le **runtime dataflow**, pas ce
flux. Donc tout site ci-dessous qui masque un échec le masque **entièrement** :
il n'y a pas de journal où le retrouver.

26 sites dans `src/catalog.rs`, 3 dans `src/agent.rs`, aucun ailleurs. Trois
genres :

### A. Une déclaration acceptée qui ne fait rien

| Site | Ce qui est tu | Ce qu'il faudrait |
|---|---|---|
| `:710` | `boost` posé sur un champ — « accepté et jamais appliqué » | un **refus de `validate`**, comme celui d'un signal sémantique sans champ de contenu : le précédent existe dans `config.rs` |
| `:1985` | `special_ops` d'une KB — « désérialisé et jamais lu » | idem, ou le reçu d'ouverture |
| `:1995` | poids par champ d'une KB — « copiés dans les métadonnées et plus jamais relus » | idem |

Ce ne sont pas des échecs d'opération : ce sont des **déclarations sans effet**.
Quelqu'un écrit `boost` dans son schéma, croit peser son classement, et rien ne
le détrompe — jamais. Le message est déjà parfaitement écrit ; il n'a pas de
lecteur.

### B. Une capacité absente, et la durabilité avec elle

| Site | Ce qui est tu |
|---|---|
| `:974` | aucun magasin de checkpoints pour ce dialecte — **« la reprise après incident est indisponible »** |
| `:1006` | aucun magasin de blobs — **« les index FTS et sparse ne seront pas persistés »** |

Les deux plus graves du tableau. Une base peut tourner sans reprise après
incident et sans persistance d'index, et le seul endroit qui le dit est un flux
que personne n'écoute. Leur place est le **reçu d'ouverture**, celui que le lot
de l'index vectoriel vient de commencer à remplir.

### C. Une opération a échoué, et on continue

Par ordre de dégât décroissant — le critère étant « une lecture ultérieure
sera-t-elle fausse, incomplète, ou seulement lente ? »

| Site | Ce qui est tu | Dégât |
|---|---|---|
| `:2663` | marque de travail non publiée — **« un lecteur d'un autre processus croira la base à jour »** | **faux** : le message nomme lui-même sa victime |
| `:6762` | textes tronqués à la limite de jetons — « n'est donc pas trouvable par le vecteur, alors que la ligne est complète en base » | **incomplet et invisible** : la recherche rend moins, la base a tout |
| `:4495`, `:4547`, `:4576` | `split_unchanged` sauté : entité absente de la configuration, relecture impossible, 0 ligne relue | **faux, et vérifié** : les trois rendent `previous_states` **vide**, et `lifecycle_verdict` traite un `from: None` comme une **naissance** — donc n'importe quel état déclaré passe. La garde de cycle de vie est contournée en silence. Le code le sait déjà : « d'avant sort d'ici, un silence fait passer une transition interdite », juste au-dessus de `:4547` |
| `:4642` | chunks illisibles — « rien n'est court-circuité » | **lenteur seule** : celui-là rend `previous_states_de(…, &stored)` avec un `stored` bien relu. La garde tient. À ne pas confondre avec les trois ci-dessus, ce que le voisinage des messages invite à faire |
| `:3142`, `:6723` | retard d'embarquement non rattrapé — « des chunks restent sans vecteur » | **incomplet, mais partiellement consulté** : `index_state_for` et l'exigence « dense » le rattrapent |
| `:5621`, `:5680` | dette de rendu non posée, re-rendu en échec | **périmé durablement** : une entité dérivée reste vieille sans que rien ne le dise |
| `:2713`, `:6488` | marque non effacée, marque posée par sécurité | **lenteur** : des lecteurs `Strict` attendent pour rien |
| `:1347` | index secondaire non posé | **lenteur seule**, et son commentaire le dit déjà |
| `:3189`, `:3197`, `:3321`, `:3329`, `:5466` | relais d'avertissements venus d'un runtime | hérité — à juger avec leur émetteur |
| `:1679`, `:1703` | index vectoriel : refus non réparable, et le rebâti réussi | **traité** par le lot du 4 octobre |

### Ce que j'en fais, et ce que je ne fais pas

Je ne corrige rien ici sans le dire à la session qui tient le fichier. Deux
propositions, dans cet ordre :

1. **Un reçu d'ouverture** qui porte B en entier et la part de C qui change la
   durabilité. Il existe déjà en germe : le lot de l'index vectoriel y dit
   « index rebâti : *n* lignes, *d* ms ». Lui donner B, c'est une structure à
   remplir, pas un mécanisme à inventer.
2. **A devient des refus de `validate`.** C'est le moins coûteux des deux et le
   plus sûr : une déclaration qui ne fait rien doit être refusée à l'écriture
   du schéma, pas signalée à l'ouverture de la base.

Et une règle à opposer au prochain `Warning` qu'on voudra écrire : **un
avertissement est une erreur dont on a choisi de ne pas mourir.** Ce choix se
justifie par un lecteur nommé. Sans lecteur, ce n'est pas un avertissement,
c'est un silence avec du texte dedans.

## Les suites dont l'objet est la carte locale ne tournent pas de jour

Tranché par l'orchestration le 4 octobre 2026, après un gel d'écran : un démon
d'embarquement **local** tenait 2,7 Go de mémoire et 3,6 Go de mémoire vidéo sur
la carte qui porte l'écran de Lucie, pendant qu'elle travaillait.

**Pourquoi aucun régime doux ne sauve ces suites-là.** Quatre suites
court-circuitent **exprès** le service d'embarquement de l'autre poste —
`tests/common/mod.rs`, `fn par_le_service` rend `None` avant même de le
consulter quand `suite_locale()` est vraie :

```rust
const SUITES_LOCALES: &[&str] = &["e2e_burn_", "e2e_demon_embeddings",
                                  "e2e_mesure_ingestion_code", "e2e_banc_bge_m3"];
```

Et c'est juste : leur objet **est** l'embarqueur, le démon, ou la vitesse de ce
poste. Les envoyer au service leur ferait mesurer la carte d'un autre — un
chiffre juste pour une question qu'on ne leur pose pas, ce qui est pire qu'un
rouge. `RAG3WEAVER_EMBED_SERVICE` ne peut donc rien pour elles, par
construction et non par oubli.

**La règle** : tant que Lucie est devant l'écran, ces suites **ne tournent
pas**. `RAG3WEAVER_SANS_CARTE_LOCALE=1` les écarte **en le disant**, et c'est le
défaut des batteries de jour.

**Et la conséquence sur ce qu'on rend** : une batterie ainsi jouée se dit
**« complète hors carte locale »**, jamais « complète ». Elles se jouent quand
Lucie le dit — la nuit, ou sur demande —, et **une livraison qui touche
l'embarqueur, le démon ou le moteur burn les exige avant fusion** : il faut
alors le lui demander.

**Le second défaut, indépendant** : le démon survit exprès à qui l'a lancé
(`Fin::Laisser`), pour que le binaire de test suivant retrouve le modèle déjà
chargé — 2,2 Go de chargement économisés par binaire. La survie sert **pendant**
la passe et ne sert plus rien après. Donc `run_e2e.sh` doit relever `pidof
rag3weaver-embeddings` au début et arrêter en fin de passe **le démon qu'il a vu
naître pendant elle** : la même symétrie que sa somme du moteur, on relève
l'état avant et on le rétablit après.

Et l'arrêt se fait **par `pidof`, jamais par un motif** : `pgrep -f` attrape le
shell qui porte le motif.

## Méthode : huit façons de prendre son harnais pour un résultat

Relevé le 3 octobre 2026 au soir, en une heure, pendant `e2e_arret_brutal` ;
la cinquième est tombée la nuit suivante, la sixième le 4 octobre
(quatre fois le même jour), la septième et la huitième le 10. Elles se sont présentées à la suite,
chacune sous un visage neuf ; la quatrième a failli faire annoncer une fausse
régression à une autre session, et la cinquième montre que le remède de la
quatrième était à moitié écrit.

**1. Le sous-module reste en arrière après un rebase.** Le pointeur de
`codeparsers` avance avec le code ; le clone local d'un worktree ne suit pas,
et **`git status` ne montre rien** — on n'a rien modifié, c'est le contenu qui
est devenu obsolète tout seul. La lib compile alors contre un parseur de la
veille et l'erreur sort dans `code.rs`, un fichier qu'on n'a jamais ouvert.
Après chaque rebase qui amène du code : `git submodule update --init`, ou
`git config submodule.recurse true` dans le worktree.

**Et le remède échoue**, dans ce worktree-ci — mesuré le 4 octobre 2026, le
piège s'étant présenté pour de bon :

```
Impossible de rapatrier dans le chemin de sous-module 'extension/rag3weaver/codeparsers'
fatal : transport 'file' non permis
```

Son `origin` est le **chemin local** de l'arbre principal, et git refuse le
transport `file` pour un sous-module depuis la CVE-2022-39253. Donc
`git submodule update` ne peut pas faire son travail ici, et il le dit d'une
façon qui ne ressemble pas à « ton pointeur est en retard ».

Ce qui marche, et qui ne demande ni de desserrer `protocol.file.allow` ni de
cloner 2,6 Go : **un `fetch` ordinaire depuis l'arbre principal, puis le
commit exact**. La restriction porte sur le clone d'un sous-module, pas sur un
`fetch`.

```sh
cd extension/rag3weaver/codeparsers
git fetch origin <sha du pointeur>      # origin = l'arbre principal
git checkout -q <sha du pointeur>
```

Le `<sha>` se lit par `git ls-tree HEAD extension/rag3weaver/codeparsers`, et
`git -C … rev-parse HEAD` dit où l'on est : **les deux doivent coïncider**,
c'est la seule vérification qui attrape ce piège.

**Et cette recette échoue à son tour quand l'arbre principal est lui-même en
retard** — le 10 octobre 2026, le cas s'est présenté : le pointeur était passé
sur master par le push d'une autre session, sans transiter par l'arbre
principal, dont le clone de sous-module ne connaissait pas le commit. Un
`fetch origin <sha>` ne peut pas donner ce qu'il n'a pas, et l'erreur ressemble
à la précédente.

La sortie de secours est la **source déclarée**, qui ne dépend de personne :

```sh
cd extension/rag3weaver/codeparsers
git fetch git@github.com:L-Defraiteur/codeparsers.git <sha du pointeur>
git checkout -q --detach <sha du pointeur>
```

L'URL se lit dans `.gitmodules` — c'est elle qui est vraie, pas l'`origin` du
clone local, recâblé sur un chemin de fichier. Et cela **sans** desserrer
`protocol.file.allow` : affaiblir un garde-fou global pour contourner un remote
mal câblé est le mauvais échange, et c'est le genre de concession qu'on ne
reprend jamais.

**2. `git stash` est partagé entre les worktrees.** C'est une pile **par
dépôt**, pas par arbre de travail. Un `stash` qui n'empile rien suivi d'un
`pop` dépile **celui d'une autre session**, l'applique chez vous et le retire
de la liste. Pire que l'index partagé, qu'on connaissait : l'index se voit
dans un `git status`, un stash dépilé ne se voit **nulle part** chez son
propriétaire — il croit son travail rangé. Jamais de `git stash` ici ;
`git diff > fichier.patch` fait le même travail sans toucher à un état commun.

**3. Un rouge qui ne ressemble pas à la question posée est un rouge du
harnais.** Un test de perte de données qui échoue sur « ce chemin est un
répertoire » ne parle pas de perte de données. Règle utile, et insuffisante —
voir la quatrième.

**4. Un résultat qui ne dit pas contre quoi il a été obtenu n'est pas un
résultat.** Celle-là est la plus coûteuse, parce qu'elle **ressemble trait
pour trait à une vraie régression** : même signal, même code de sortie, même
base conservée à examiner. La garde 1 du rejeu était commitée à 23 h 11 ; la
bibliothèque liée datait de 21 h 33. Le SIGSEGV décrivait le moteur d'avant.
Seule la comparaison de deux horodatages l'a attrapé — la règle 3 ne l'aurait
pas vu.

Conséquence pour les suites qui lient le moteur : **imprimer l'âge de la
bibliothèque liée**, et refuser une bibliothèque plus vieille que les sources
C++. `e2e_arret_brutal` imprime déjà la ligne ; proposé à `run_e2e.sh`.

**5. Le moteur remplacé *pendant* la passe.** La suite de la quatrième, trouvée
une heure plus tard, et elle montre que le remède de la quatrième était à
moitié écrit. `run_e2e.sh` imprime l'âge de la bibliothèque **en tête de
passe** : ça dit contre quoi la passe a *commencé*, pas contre quoi elle a
*tourné*. Le 4 octobre à 00 h 59 min 43, la session de l'arbre principal a
rebâti `librag3db.so` pendant une batterie complète. Les binaires de test
démarrés avant cette seconde tiennent l'ancienne bibliothèque, déjà projetée en
mémoire ; ceux d'après la neuve. **Un seul résultat, deux moteurs, et rien dans
le journal ne le dit** — la ligne d'âge du début est parfaitement exacte et
parfaitement trompeuse.

Ce qui l'a attrapé n'est pas le harnais : c'est que l'autre session a **annoncé
qu'elle n'avait pas attendu**. Sans son message, j'aurais rendu un verdict sur
un mélange.

Conséquence, **livrée** par la session de l'arbre principal dans
`run_e2e.sh` : une **somme du contenu** (`cksum`) des deux bibliothèques, au
début et à la fin, et la passe sort en échec si elle a changé — « le moteur a
été remplacé pendant la passe : ce résultat ne vaut rien, rejoue ». Éprouvé
contre une copie privée modifiée en cours de passe.

J'avais proposé la date et la taille ; la somme est meilleure, et pour une
raison mesurée plutôt que raisonnée : **deux rebâtis de cette nuit avaient la
même taille à l'octet**, et un `touch` change la date sans rebâtir. Ma version
aurait donc raté un vrai échange et crié au faux. À retenir pour la forme : un
contrôle d'identité se fait sur le **contenu**, et les métadonnées qui
l'approchent (date, taille) sont des indices, pas des preuves.

Et la règle de voisinage qui va avec : **annoncer, attendre les « libre » des
passes en vol, puis rebâtir** — l'annonce seule ne suffit pas, puisque la passe
en vol ne peut pas l'entendre.

**6. Le chemin du moteur dérivé du chemin du code.** Quatre fois le 4 octobre
2026, dans quatre endroits différents, et à chaque fois depuis un **worktree** :
la bibliothèque et l'extension du moteur sont bâties dans l'arbre principal, et
un harnais qui dérive leur chemin de `CARGO_MANIFEST_DIR` ou du chemin de son
propre script les cherche là où vit le **code**.

| Où | Ce que ça donnait |
|---|---|
| `e2e_arret_brutal` (le mien) | trois rouges nommés — `RAG3DB_ROOT` oublié à l'appel ; la suite, elle, le lisait |
| `e2e_mesure_sync_source` | injouable depuis un worktree, `RAG3DB_ROOT` pas lu du tout |
| `e2e_code_sync` (un seul site, deux tests) | idem |
| **`run_e2e.sh`** | **code 1 sans un seul message** |

Le dernier est le plus instructif, et c'est un contrôle que j'avais demandé
moi-même : `empreinte_du_moteur` faisait `[ -f "$f" ] && cksum "$f"` dans une
boucle, donc rendait **1** quand le dernier fichier manquait ; l'affectation
`MOTEUR_AU_DEBUT="$(…)"` propageait ce 1, et `set -e` tuait la passe en
silence. **Un contrôle qui tue en silence est pire que pas de contrôle** : il ne
laisse même pas la trace d'un refus. Une fonction de ce genre se termine par
`return 0`, et dit « absent … » pour ce qu'elle n'a pas trouvé.

La règle, donc : **le chemin du moteur ne se dérive jamais du chemin du code.**
`RAG3DB_ROOT` d'abord, le manifeste en repli, et l'absence **dite**.

**Et `RAG3DB_ROOT` désigne l'arbre où le moteur est bâti, pas « l'arbre
principal ».** La nuance a de l'importance et elle se trompe facilement : un
worktree qui a lancé son propre `cmake` a sa `libvector` à lui, et c'est **lui**
qu'il faut désigner ; un worktree qui n'a jamais bâti n'a pas de
`extension/vector/build/` du tout, et doit désigner l'arbre principal. Les deux
cas se sont présentés le même jour, dans deux sessions, avec des réponses
opposées — et chacune avait raison chez elle.

Et la façon de chercher ce motif, qui a tranché là où une liste de fichiers ne
suffisait pas : un `grep` sur `extension/vector/build` dans `tests/` — par le
**chemin**, pas par la liste des fichiers déjà en cause. **À rejouer après
chaque rebase** : un second site du même motif est arrivé sur master le jour
même, et un grep joué avant le rebase ne pouvait pas le voir.

**7. Le binaire de test plus vieux que le moteur qu'il charge.** 10 octobre
2026 : trois rouges d'un coup, `undefined symbol: setForceCheckpoint`, dans une
suite qui passait la veille. Le symbole existe bien dans la bibliothèque du
moteur du jour — le binaire de test, lui, datait du **4 octobre**, et cargo ne
l'avait pas rebâti parce qu'aucune de **ses** sources n'avait changé. J'ai failli
l'imputer au moteur.

C'est le sixième piège d'un cran plus loin. **Correction du même jour, 18 h**,
parce que la première version de cette note accusait un trou qui n'existe pas :
j'avais écrit que « personne ne surveille l'âge de la bibliothèque » autrement
qu'à l'horloge. C'est faux, et `run_e2e.sh` me l'a prouvé en me **refusant** une
passe :

```
✗ librag3db.so est plus vieux que le dernier commit de ses sources
  (src : d10b92306 du 10/10 17:53)
✗ Rebâtir : cmake --build … , ou RAG3WEAVER_MOTEUR_ANCIEN=1 en le sachant.
```

La garde compare l'artefact au dernier commit de **ses sources**, ce qui est le
bon critère, et elle nomme son remède et sa dérogation. Je ne l'avais jamais vue
parce que je ne l'avais jamais déclenchée — et annoncer un garde-fou manquant
qui existe coûte autant que d'en manquer un : un journal auquel on ne peut pas
se fier ne sert plus à rien.

**Ce qui reste vrai, et c'est le piège** : cette garde protège la
**bibliothèque**, pas le **binaire de test** qui la charge. Rien ne compare
l'âge de `deps/e2e_…` à celui du `.so`, et c'est le seul maillon que les deux
gardes laissent sans surveillance.

Ce qui le rend vicieux : rien n'est périmé au sens de cargo. Le binaire est à
jour par rapport à ses sources, la bibliothèque est à jour par rapport aux
siennes, et les deux sont pourtant incompatibles parce qu'elles ne se lient
qu'au **chargement**. Un `cargo test` qui ne recompile rien n'est donc pas une
bonne nouvelle quand le moteur a bougé sous lui : c'est exactement le cas où il
faut forcer. `touch` sur une source du crate, ou `--tests --no-run` après tout
rebâti du moteur.

Et la leçon de forme, la même que les six autres : la condition de validité
manquante n'était ni la bibliothèque ni les sources, mais **la date du binaire
mesuré lui-même**. Une mesure commence par prouver que l'artefact chargé vient
du code qu'on croit mesurer — pas seulement que ses sources sont à jour.

**8. Le diff qui montre vos propres fichiers comme supprimés.** 10 octobre
2026, avant une fusion : `git diff --stat HEAD..origin/master` pour savoir ce
qu'un rebase allait apporter, et la sortie annonce `src/gabarits.rs | 163 -----`
— mon fichier, celui que je venais d'écrire, en voie de disparition. J'ai
failli chercher qui le retirait.

Il ne manquait rien : `HEAD..origin/master` compare **deux têtes**, donc tout
ce que ma branche a et que master n'a pas encore paraît comme une suppression.
La question « qu'est-ce que master apporte ? » se pose depuis la base de
fusion, jamais depuis ma tête :

```sh
base=$(git merge-base HEAD origin/master)
git log  --oneline $base..origin/master
git diff --stat      $base..origin/master
```

Ce qui le rend dangereux n'est pas la subtilité, c'est la **crédibilité** : la
sortie est bien formée, le chiffre est juste, le fichier existe, et la seule
chose fausse est la question qu'on croyait poser. Un rouge franc se corrige ;
une réponse exacte à une autre question se croit.

Et le détail qui achève de convaincre : la **même** sortie m'était passée sous
les yeux deux heures plus tôt, sur un rebase précédent, et je l'avais
reconnue — « c'est attendu, master n'a pas encore mes commits ». Savoir
pourquoi une sortie trompe ne protège pas de s'y laisser prendre la fois
suivante, quand elle nomme un fichier auquel on tient. C'est la commande qu'il
faut changer, pas la vigilance.

**Et la forme commune aux huit**, qui est aussi celle des défauts qu'on
corrige dans le produit : une information existe, et rien ne la consulte. Le
pointeur du sous-module, la pile de stash, la provenance d'un rouge, l'âge
d'une bibliothèque, la date du binaire qui la charge — et, pour la cinquième, son âge **à la fin**. Un banc, un
test ou un rapport doivent **porter leur condition de validité à côté de leur
verdict**, sinon le verdict se lit tout seul et on le croit.

La cinquième ajoute une nuance aux quatre autres : une condition de validité
relevée **avant** le travail ne prouve rien de ce qui s'est passé pendant. Ce
qui peut changer sous les pieds d'une mesure se vérifie **aux deux bouts**.

### Mémoire longue : l'ordre des lots change (3 octobre 2026, au soir)

Proposition et banc : `extension/rag3weaver/docs/3-octobre-2026-20h41/01-…`.
Livré : le gabarit `templates/backends/memory` (Memory, Subject, les deux
relations, la machine à états), le banc `scripts/test_banc_memoire.py` avec ses
72 paires de contrôle versionnées, et l'ingestion qui émet `EntitiesChanged`.

**Le nœud de décision à modèle descend derrière le réacteur, le crochet après
outil et le jardinier.** La raison, et elle est mesurée sur trois jeux :

- **ranger** un texte parmi des sujets existants est un problème d'**ordre**,
  et le cosinus le résout aussi bien que le modèle de décision (14/16, 19/32,
  22/30). Le premier appel de `remember` n'a donc besoin d'aucun modèle ;
- **décider qu'il faut créer** ne marche par **aucune** piste essayée, sur
  aucun jeu : ni l'option « aucun de ces sujets », ni la faiblesse du meilleur
  score, ni un seuil transportable. Rien de mesuré ne fait mieux que
  « demander toujours », l'appelant tranchant, la création étant réparable ;
- sur les mémoires, le verdict s'améliore nettement avec le pourquoi en plus
  du titre (AUC 0,80 → 0,92), mais sur peu de cas.

Donc le nœud garde son étage 2 sur « demander toujours », aucun modèle
branché, et le critère comme la liste de choix restent des **données du
graphe** : la découpe de la décision est tenue ouverte par Lucie — « ça reste
une expérimentation, faut pas courir avec avant d'avoir trouvé si une bonne
manière de s'en servir existe ».

**Une correction à porter quand le lot des sujets viendra** : `Subject` doit
être une **entité dérivée**. La mesure dit que le bon classement vient du
*texte complet* du sujet — sa description **et ce qu'il porte** ; avec les noms
seuls, 8/7/15. Une description écrite une fois ne contient pas ce qu'on range
dessous. La pièce existe déjà (`DerivedConfig`, `gather`, `DeriveNode`, et
l'invalidation par `_render_hash`), donc un sujet grossirait tout seul et son
vecteur suivrait, sans nœud de plus.

**Et une forme à retenir pour le nœud** : le texte du juge n'est pas le texte
du vecteur. Le classement veut le titre court (cosinus 0,99 sur les titres,
0,83 avec le pourquoi) ; le verdict veut le titre **et** le pourquoi (0,80 →
0,92). Deux étages, deux textes.

### Le produit, en parallèle des écritures parallèles (Lucie, 3 octobre 2026)

« Si nous faut tout ça pour être compétitifs, mais oui ok pour avancer en
parallèle sur les deux produits code. » Les écritures parallèles continuent
(session cœur C++) ; en même temps, les deux produits — un agent de code en
cloud ou dans le chat, qui télécharge un dépôt git ; un agent en ligne de
commande, avec accès au disque, qui ingère dépôts, dossiers et documents — se
préparent comme **un seul moteur et deux politiques d'ingestion** :

- session de l'arbre principal : les deux grains de synchronisation (option A,
  emboîtés, jamais ouverts ensemble — partie dessus sans le mot exprès de
  Lucie, à lui confirmer), `reingest_file` sur la synchronisation déclarée, la
  synchronisation d'un dépôt entier ; puis l'assemblage du backend de code ;
- session embarquements : l'entrée « indexer ce dépôt » (estimation, choix du
  modèle, avancement, seuil de confirmation) — proposition d'abord ;
- session recherche : la surface d'outils de l'agent de code sur le backend
  déclaratif — proposition d'abord.

## 4. Décisions en attente de Lucie

Posées le 18 septembre 2026, **tranchées par Lucie le 1er octobre 2026** :

1. La forme du pas C : **oui** — fusion pesée par entité, pondération par
   valeur de champ (un poids, pas un filtre), gabarits de dérivées. **Avec une
   exigence de Lucie : ces pondérations se règlent dans les graphes de
   recherche**, comme les poids de fusion (`FuseResultsNode(weights=…)`), pas
   seulement dans la config d'entité. Reste à dessiner : quand l'entité
   déclare une fusion, le gabarit ne la retouche pas aujourd'hui
   (`base_de_fusion`) — dire si un graphe peut forcer les siens.
   **Tranché le 2 octobre : préséance C** (`docs/2-octobre-2026-00h43/01-…`) —
   un graphe distingue un choix (`weights`, prime sur l'entité) d'un défaut
   (`default_weights`, ne vaut que si personne ne déclare) ; ordre : appelant >
   choix du graphe > entité > défaut du gabarit > moteur, pour la fusion comme
   pour la pondération par champ. En cours : branche `pas-c-ponderations`,
   arbre `../rag3db-pas-c`, session recherche.
2. Les poids de fusion par défaut : **FERMÉ le 4 octobre, par la mesure**
   (91c44a96c ; session recherche). Banc étagé, config produit granite
   dense + creux bge-m3 par le service : le creux AIDE (H phrases
   0,340 → 0,369/11/23 à 0,45/0,55 + creux 0,4, le meilleur point de la
   série ; identifiants 0,850 → 0,900) ; bge dense ne remplace pas
   granite (0,342 contre 0,405) ; le couple 0,45/0,55 prend son sens une
   fois le creux là. Adopté aux deux gabarits :
   `default_weights='bm25:0.45,vector:0.55,sparse:0.4'` — étage passé de
   « choix » à « défaut », que l'entité et l'appelant priment. Reste la
   pièce de câblage livrée (workspace.index_signals, 1afd8bfbe). **Et la
   position de Lucie, 4 octobre** : le creux est une OPTION documentée,
   pas le défaut — un creux appris sur du texte faiblit sur les jetons
   hors vocabulaire (identifiants), hors domaine il peut faire pire que
   BM25 ; « la parité avec ce que propose Qdrant », des expériences avant
   de préconiser (liste à venir de la session optimiseur, jouée au banc
   par la session recherche). Le manifeste d'exemple ne l'active plus
   (10bdda04d) ; les deux clés et la mesure restent au README ; le poids
   sparse:0.4 des default_weights reste, il ne pèse rien sans signal.
3. Le texte embarqué (le nom avec le corps) : **plus tard**.
4. `fuse_results` à trois listes et ses onze tests : **supprimer** — fait
   le 2 octobre (`01791e347`) ; les onze tests, seuls tests unitaires de la
   fusion vivante, portés sur `fuse_signals`.
5. La grappe d'exploration après recherche (`search_with_explore`,
   `explore_bfs`) : **supprimer** — fait (`01791e347`).
6. `search_with_strategy` : **supprimer** — fait (`01791e347`) ; sa garde
   `max_rounds` vit dans `build_dataflow_graph`, qui rend un `Result`.
   `sparse-vector` alignée en 4.3.0 avant (`7c653f66c`, sources identiques).

Encore en attente :

7. ~~Le budget de reprise du lecteur~~ — **tranchée le 3 octobre 2026** : il
   reste à 250 ms (§1, après la marche 1).
8. ~~Fusionner `mtg-experiments` dans `master`~~ — **tranchée et faite le
   1er octobre 2026** : correctifs C++ extraits seuls (`9edf6f3b4`, avec le
   test de `ParsedParameterExpression::copy` prouvé rouge sans le correctif),
   fusion `32d6e0b44`, trois corrections de tests et du chat par-dessus, `master`
   vert puis poussé (`d4f32f9e0`).
Tranchée le 1er octobre 2026 : les trailers d'attribution à une IA
(`Co-Authored-By: Claude…`) ont été retirés des 17 commits de
`mtg-experiments` qui en portaient, par réécriture des messages et push en
force ; arbres, auteurs et dates inchangés, **hash changés** à partir de
`64a12b05b` (le commit de Codex `ab95c3a2d` garde le sien). Sur `master`,
quatre commits anciens en portent encore — un de février 2026, trois de
l'amont Kuzu de 2025 — et ne sont pas réécrits : cela changerait tous les
hash du dépôt public.

**2 et 3 octobre 2026, synchronisation par périmètre** : le périmètre se
déclare dans l'entité, des champs de la ligne (aucun nom en dur) ; les
relations partent avec la ligne et sont comptées ; `onMissing` nomme une
**transition** (pas un état) et une absente d'où elle ne part pas est gardée
et nommée ; une fin est une unité ; proportion maximale réglable, la moitié par
défaut ; **une session à la fois par périmètre**, identifiant rendu par le
moteur, reprise explicite (`takeover`), jamais d'expiration — comme le verrou
d'état de Terraform et la table de verrou de Flyway ; **marque d'absence**
`_absent_since` ; **mise de côté de 7 jours** avant purge, avec l'annulation en
bloc, en seconde étape. Un périmètre est **borné à la cellule** courante.

**3 octobre 2026** : étapes 1 et 2 du pas C livrées et fusionnées — l'échelle
de préséance sur la fusion (`weights` choix / `default_weights` défaut,
`search_base` migré) et `FieldWeightNode` (pondération par valeur de champ,
posé d'office, neutre sans déclaration — prouvé au banc par contre-épreuve,
référence du corpus 5 278 : tel quel 0,333, G 0,409). Mesures de pondération
et gabarits de dérivées en cours ; rien posé dans `Scope`.

### Décisions du 3 octobre 2026 au soir (Lucie, relevées par l'orchestration)

Relevé de ce qui s'est décidé en conversation ce soir-là, pour que rien ne
vive seulement dans des messages. Chaque ligne nomme qui porte la suite.

**Règles de travail.**
- Un lot fini et vert se fusionne dans master sans attendre, dans rag3db comme
  dans le sous-module codeparsers : « y a que nous dessus et tout est
  expérimental ». Restent : pas de push en force, rien hors de ses dépôts sans
  son mot.
- Les choix réversibles se tranchent sans lui remonter : « toujours prendre le
  meilleur chemin possible et laisser la voie aux autres options ».
- L'adresse gmail est son adresse personnelle, à utiliser partout ; seule la
  professionnelle est interdite.
- `LUCIVY_SCHEDULER_THREADS=8` sur toute passe qui ouvre un catalogue tant
  qu'elle travaille sur le poste : l'ordonnanceur de lucivy démarre sinon sur
  tous les cœurs.

**Les verrous** (`docs/3-octobre-2026-15h47/01-…`, les trois écarts, tranchés
par délégation) : l'annonce en tête de transaction, oui ; hors annonce, on
reste en instantané par transaction, avec une erreur nommée à rejouer ; créer
une relation prend un verrou partagé sur ses extrémités. Le nom de l'erreur
est stable et le mode du verrou est un paramètre, pour laisser les autres
options ouvertes. Porte : cœur C++, après A5 bis, l'insertion des relations
et H4.

**A5 bis** : la garde simple se livre malgré son coût (jusqu'à 20 % sur des
recherches vectorielles parallèles pures, mesuré sous charge) ; la garde fine
devient une marche à part, décidée sur une mesure au calme. Porte : cœur C++.

**Indexer un dépôt.**
- Mesuré : ce dépôt entier, 6 735 fichiers et 418 761 relations, 1 798 s pour
  être cherchable par mots, 710 s de vecteurs ; la cause est l'insertion des
  relations, qui croît avec la base.
- Le premier index d'une source se fait **en masse** pour les relations
  (question de Lucie : « gros dossier, on fait tout d'un coup ») : les paquets
  posent nœuds et texte, les relations attendent, un seul chargement à la fin,
  puis la résolution des symboles. L'incrémental reste pour les éditions et
  les sources déjà indexées, **et s'optimise aussi** (seuil du chargement par
  `COPY` à mesurer plus bas ; correctif du moteur). Portent : arbre principal
  (masse, seuil), cœur C++ (moteur).
- **Une édition pendant l'indexation** ne la ralentit jamais : fichier pas
  encore passé, il est lu à son paquet dans son état édité ; fichier déjà
  passé, il entre dans une file et se reprend en incrémental à la toute fin.
  Porte : arbre principal. **Livré** (branche `edition-pendant-l-indexation`).
  Le registre « à reprendre » vit hors du catalogue (`code_sync::note_change`) :
  l'outil d'édition (`edit_file_shared`) n'attend jamais le verrou qu'une
  indexation tient. Il écrit, s'inscrit et le dit dans son rendu. La
  justesse tient à un ordre : la synchronisation inscrit un chemin comme
  « lu » *avant* de le lire, et l'édition écrit *avant* de consulter.
  La reprise passe par `reingest_file` une fois les sessions closes, ou par
  `remove_file` pour un fichier supprimé. La file se vide sous le même verrou
  que la désinscription, pour qu'aucune édition ne reste orpheline.
  `reingest_file` retire d'abord les arêtes sortantes des scopes du fichier,
  sauf `CONSUMED_BY`, et le miroir `CONSUMED_BY` de ses `CONSUMES` : un scope
  gardé qui n'appelle plus `f` gardait sinon son arête, ce qu'une mutation
  du test a montré. Le COPY garde déjà la sémantique de MERGE, donc rien ne
  double. Quatre tests (`e2e_code_sync`), dans les deux modes de relations,
  comparent l'index final à un index bâti à neuf, arête par arête avec leur
  multiplicité. Le libellé du journal d'indexation est proposé à la session
  embarquements (`run_index`).
- **La recherche choisit seule son mode** selon l'état de l'index : balayage
  des fichiers quand rien n'est indexé, plein texte quand les mots sont prêts,
  fusion quand tout l'est ; une ligne d'état le dit ; un paramètre permet de
  forcer. Porte : session recherche.
- `WorkingTree` respecte les règles d'exclusion du dossier et garde les
  fichiers cachés non ignorés ; les secrets probables sont écartés par une
  règle nommée.
- Point ouvert : pas de recherche dans le même processus pendant la phase du
  texte.

**Le backend de code** : deux manifestes (poste, cloud), un schéma **nommé**
au manifeste (`"index": "code"`, rien d'enregistré sans la clé), une politique
par outil. Question ouverte : descriptions d'outils en français, rendus du
moteur en anglais.

**Codeparsers** (sous-module) : le genre d'usage et la ligne sortent sur
chaque relation ; il sera rangé en **propriétés sur l'arête existante**, pas
en relation « appelle » neuve. La résolution entre homonymes est rendue
déterministe ; 1 552 fausses arêtes retirées sur `src/dataflow`
(`extension/rag3weaver/docs/3-octobre-2026-20h30/`). Les références du banc
de recherche d'avant ne sont plus comparables : refaire une référence.

**Visions** (`extension/rag3weaver/visions/2026-10-03-20h16-explorer-les-relations.md` et
`extension/rag3weaver/visions/`) : explorer les relations (usages, chemins entre résultats, poids
par proximité, voisinage, structure) ; sept idées pour le produit code, toutes
retenues, avec le crochet après outil (idée de Lucie pour « le même motif
existe ailleurs ») et des notes accrochées au code, datées et versionnées ;
les propriétés d'arête, dont la provenance.

**Mémoire longue** (`extension/rag3weaver/visions/2026-10-03-20h37-memoire-longue.md`, proposition `…-20h41/01`) : produit ouvert
en parallèle. Choix pris par délégation, chacun un réglage du manifeste :
premier usage l'agent de code, avec un scénario sans dépôt au banc ; tout
agent écrit, l'origine marquée, une déduction n'écrase jamais une parole ; la
personne voit une page à la demande et une ligne au rendu ; dans le doute, le
nœud de décision crée et demande après, jamais de fusion silencieuse. Les
sujets abstraits sont une entité de rendez-vous (`Subject`). Porte : session
mémoire (ex-lifecycle).

**Modèle de décision** : un nœud générique, modèle branchable ; critère ferme
de Lucie : rien que nous devions entraîner nous-mêmes. Candidats vérifiés à la
source (`extension/rag3weaver/docs/optimiseur/3-octobre-2026-20h55/`), mesure
en cours contre ce que nous avons déjà (cosinus granite, reranker). Porte :
session optimiseur.

**Toujours en attente de Lucie** : « envoie »
pour tracel-ai (dans la fenêtre de l'optimiseur) ; le réglage rafale/pause
devant l'écran ; le seuil de confirmation d'`index` et la politique cloud ; la
demande au support GitHub ; le ménage des branches distantes ; dans la vision
des relations, la cohésion active par défaut ou non.

### Un seul résolveur entre fichiers : le graphe ne dépend plus de la taille du paquet (4 octobre, minuit)

Décidé par l'orchestration, sur les mesures de l'arbre principal et de la
session codeparsers. **C'est un changement de ce que l'index contient**, pas
un réglage.

**Livré le 4 octobre vers 1 h** (`63d154730`, codeparsers `fichier-seul` @
`d567e7c`). Les mêmes arêtes par paquets de 1, de 64 et d'un seul tenant,
trois passes sur trois. Banc des relations, master → livré :
- qui appelle : 1,00/0,88 → 1,00/0,88 ;
- dépend de : 1,00/0,64 → 1,00/0,85 ;
- relie : 4/4 → 4/4 ;
- tests qui traversent : 0,96/0,92 → 0,96/0,93.

Les pièges trouvés en route :
- un nom de bibliothèque était pris sur tout le paquet ;
- l'ordre des fichiers venait d'un `HashMap` ;
- le mot NULL des COPY était une constante du code, devenue un symbole ;
- le correctif des `use` Rust internes était resté hors d'un commit de
  codeparsers.

**Le fait.** Le même dépôt compte 819 808 relations indexé d'un seul tenant,
contre 438 104 par paquets de 64. Sur `src/` de rag3weaver (129 fichiers) :
59 152 contre 72 721. Seuls les liens entre fichiers diffèrent. codeparsers
résout lui-même les noms entre les fichiers qu'il voit ensemble, et choisit
parmi les homonymes : même fichier d'abord, puis le type lu, puis le premier
homonyme dans un ordre fixe. Son choix change donc avec le paquet. La voie des
`Symbol` (rendez-vous `DEFINES`/`MENTIONS`), elle, s'abstient sur un nom
ambigu.

**La justesse.** Deux échantillons indépendants de 50 arêtes, tirées dans
l'écart et relues (arbre principal sur `src/` de rag3weaver, codeparsers sur
le dépôt entier), donnent tous deux 15 justes et 35 fausses.
- **Fausses** : appels de méthode sur un receveur sans type (`.len()`,
  `.count()`, `.get()`), accès de champ (`self.dialect` relié au module
  `dialect`), autre langage (`std::fs` Rust vers une variable de `.cjs`),
  autre crate du même nom.
- **Justes** : types importés (`use crate::estimate::Rate`), chemins de
  module.

**La décision.**
1. codeparsers ne résout plus que **dans le fichier** (option `cross_file`).
   La résolution devient parallèle par fichier.
2. Tout lien entre fichiers passe par la voie des `Symbol`, qui s'abstient
   sur un nom ambigu. La mention porte, pour départager, le type lu (déjà
   fait) et **le chemin d'import** (à faire).
3. L'analyseur reçoit la **liste des chemins du projet** : sans elle, par
   paquets, un import interne devient une « bibliothèque ».
   `USES_LIBRARY` : 15 994 par paquets contre 5 975 d'un seul tenant.
4. Un test : la même source par paquets de 1, de 64 et d'un seul tenant
   donne le même graphe, arête par arête. La justesse se mesure avant et
   après, au banc des relations et sur `e2e_code` ; tout seuil qui bouge se
   justifie par la liste des arêtes retirées.

**À faire à la mise à jour.** Une base existante garde ses arêtes devinées :
une resynchronisation ne les retire pas (`ingest_code_jusqu_a` ne nettoie
pas les arêtes ; seule une édition, par `reingest_file`, nettoie celles du
fichier édité). Il faut réindexer la source de zéro, ou faire un nettoyage
unique : retirer les arêtes d'usage entre scopes de la source, puis
rejouer analyse et résolution.

**Ce que ça ouvre.** La taille du paquet ne change plus que le temps. Le
premier index peut prendre de gros paquets, bornés en octets plutôt qu'en
fichiers. Mesure de la session embarquements : le dépôt entier en un
paquet fait 132 s (352 par paquets de 64), pour 16 Go au pic.

**Dégelé le 4 octobre à midi** (mesure de la session embarquements, base
**sur disque**) : paquets de 512, 823 s contre 120 s en mémoire, dont 346 s
de poussée des blobs en 56 appels. Sur disque, c'est de nouveau le premier
poste, à toute taille de paquet. Le gel ci-dessous ne valait que pour une
base en mémoire.

**Gelé (base en mémoire seulement) : le report de la poussée des blobs d'index.** C'était 119 s sur 352 par
paquets de 64, à 0,29 s pour chacun des 404 appels d'`ingest_entities`. En un
seul paquet, la même poussée ne coûte plus que 2 s, en 4 appels (mesure de la
session embarquements). Le report, avec sa marque durable « plein texte en
retard », est donc sans objet pour le premier index par gros paquets. Pour
l'incrémental, ce n'est pas un gros poste. Ne pas le rouvrir sans une mesure
qui le redemande.

**Livré côté codeparsers (4 octobre, nuit).** Branche `fichier-seul` du dépôt
codeparsers, `2c17314`, en attente du pointeur unique chez l'arbre principal.
- `resolve_cross_file: Some(false)` : aucune relation ne traverse un fichier ;
  ni les cibles, ni les qualificatifs connus, ni les types lus d'un champ ou
  d'un retour ne dépendent plus du paquet. Sur le dépôt entier, un appel et
  85 paquets rendent les mêmes 371 289 relations.
- `project_files` : seulement pour Python. Ailleurs, la forme de l'import dit
  ce qui est local. Un dossier `std` ou `node` du dépôt effaçait sinon la
  bibliothèque du même nom.
- `IdentifierReference.import_origin { source, imported, via_qualifier }` :
  l'import qui amène le nom, ou son qualificatif, tel qu'écrit.

**Ce que la règle retire de l'index d'aujourd'hui.** 50 retraits relus par
rapport aux paquets de 64, pas par rapport à l'appel unique : **32 justes, 18
fausses**. Un paquet regroupe des fichiers voisins, donc le premier homonyme y
tombe plus souvent juste.
- 10 des justes étaient un bogue de la branche, corrigé avant livraison.
- Environ 16 se rattrapent par les rendez-vous : nom unique, ou import qui le
  désigne.
- Environ 5 sont des **paires déclaration `.h` / définition `.cpp`**
  (`get_time_`, `getLine`, `BrotliCreateBackwardReferences`). Deux définitions
  du même nom, que les rendez-vous ne trancheront pas.
- La fusion attend donc le banc des relations avant et après, rendez-vous
  enrichis compris. Le rappel ne doit pas baisser.

**Prochain lot côté justesse : les paires `.h` / `.cpp`.** Ce dépôt est pour
moitié du C++. La règle à mesurer : même nom qualifié, même signature,
fichiers de même base (`x.h` et `x.cpp`, ou `-inl.h`). La déclaration et la
définition sont alors une seule chose pour un rendez-vous. Elle se mesure
comme les autres : relations gagnées, et 50 relues.

**L'analyse était quadratique, et sa sortie pas déterministe** (même
branche, `8d6d212`, `2c17314`).
- La ligne de contexte de chaque référence se lisait en découpant le fichier
  depuis le début.
- La doc de chaque scope se cherchait en découpant le fichier entier.
- Gains : roaring.c 12,1 → 0,86 s ; dépôt entier en un appel 19,9 → 8,7 s ;
  par paquets de 64, 34,7 → 12,8 s.
- Références et imports sortaient dans un ordre propre à chaque fil :
  jusqu'à 28 ordres pour 64 analyses du même fichier. C'était la paire qui
  clignotait dans le test d'égalité des graphes.
- Tout est trié à la sortie. Preuve par empreinte canonique de chaque
  fichier : 0 différence avant et après, 0 entre deux passes.

**Les géants générés** (analyseurs ANTLR, `generated/*_onnx.rs`, tables
Unicode) ne coûtent plus de temps, mais n'apprennent rien à un agent. C'est à
la politique (`code::verdict`) de les écarter, avec leur raison : marqueur
« generated », dossier `generated/`, ou grande taille pour très peu de
scopes. Confié à la session embarquements, pour l'estimation.

**Mis de côté : la résolution parallèle par fichier.** En fichier seul, chaque
fichier se résout sans les autres, donc la résolution se parallélise. Elle ne
fait plus que 2,2 s en un appel sur le dépôt entier : le gain est petit.
L'orchestration l'a écartée pour l'instant.

**Pour la session cœur C++.** `SHORTEST` sur `CONSUMES` fait 11 à 18 ms par
paire sur 5 000 scopes, sans produit cartésien. Mais son plan lit les
propriétés des nœuds du chemin par un **`SCAN_NODE_TABLE`**, qui croîtra avec
la table. À regarder avant d'en faire une section de chaque recherche :
`extension/rag3weaver/docs/3-octobre-2026-20h30/08`.

### `SET` refuse un champ nul partout qu'un `CREATE` accepte (cœur C++, 3 octobre)

`UNWIND $rows AS r MATCH (t:T {id: r.id}) SET t.v = r.v`, avec `v` nul dans toutes les
lignes, est refusé : « Expression STRUCT_EXTRACT(r,v) has data type STRING but expected
INT64. Implicit cast is not supported. » La même liste par `CREATE (:T {id: r.id, v:
r.v})` passe. C'est ce que rag3weaver a rencontré dans ses annulations (`0709e3cba` le
contourne : la colonne nulle partout sort de la liste et est remise à NULL à part).

Cause, lue et exécutée : rien ne donne de type à un champ nul partout, et `UNWIND`
remplace tout type inconnu par STRING (`bind_unwind.cpp`, `purgeAny`). Ensuite `CREATE`
amène la valeur au type de la colonne par une conversion explicite
(`bind_graph_pattern.cpp`, `forceCast`), alors que `SET` n'accepte qu'une conversion
implicite (`bind_updating_clause.cpp`, `bindSetItem`) — et il n'en existe pas de STRING
vers INT64.

Trois options :

1. **Ne rien changer au moteur, garder le contournement de rag3weaver.** C'est le
   comportement de PostgreSQL pour une expression typée texte : il la refuse dans un
   `UPDATE` sans conversion explicite. Coût : nul. Le piège reste pour le prochain qui
   écrira un `UNWIND $rows … SET`.
2. **Aligner `SET` sur `CREATE`.** Essayé, une ligne, les deux tests passent — et
   retiré : `SET a.age = '12'` devient accepté, et deux tests amont (`BinderError`,
   `ListFunctionException`) montrent que la rigueur de `SET` est voulue. Le patch et ses
   tests sont gardés hors dépôt (`~/.cache/rag3db-moteur-notes/set-comme-create-et-ses-tests.patch`).
3. **Faire porter le type inconnu jusqu'au bout** : qu'un champ nul partout reste « sans
   type » à travers `UNWIND`, et ne devienne STRING que si rien ne le type ensuite. C'est
   la vraie correction, et celle qui garde `SET` strict ; elle touche la liaison et
   l'exécution d'`UNWIND`. Estimation : deux jours.

**Tranché le 3 octobre par l'orchestration, sans le faire trancher à Lucie** (c'est le
comportement des moteurs établis) : **option 1 maintenant, option 3 après le chantier des
verrous.** `SET` reste strict ; le contournement de rag3weaver reste en place. L'option 2
échangerait un piège contre un contrat plus lâche, et c'est `CREATE` qui est l'anomalie,
pas `SET`.

## 5. Hors de ce dépôt

À confirmer par qui s'en souvient : ces lignes viennent de la mémoire des
sessions, pas d'une vérification.

| Chantier | Où | État connu |
|---|---|---|
| Forks burn, cubecl, cubek | `github.com/L-Defraiteur/{burn,cubecl,cubek}`, branche `rag3weaver/pre.3` | Utilisés tels quels (burn `21674205` depuis le 2 octobre). |
| Adresse professionnelle sur les forks | `L-Defraiteur/{burn,cubecl,cubek}`, branche `rag3weaver/pre.3` | **Corrigé le 2 octobre** sur décision de Lucie : les neuf commits qui portaient son adresse professionnelle sont recréés à l'adresse personnelle (mêmes diffs), branches poussées avec bail, `pre.2` supprimées, épinglage remonté sur `master` (`04b5052ec` : burn `21674205`, cubecl `bdf6b77a`, cubek `7ba8affd`). **Reste ouvert** : GitHub sert encore les anciens commits par leur hash ; un retrait certain demande le support GitHub ou la suppression et recréation des trois forks — au choix de Lucie. Reste aussi : bâtir une fois sur les nouvelles révisions. Tout commit de `rag3db` antérieur à `04b5052ec` ne se bâtira plus sur un poste neuf une fois ces anciens commits purgés. |
| PR amont burn / cubek | à ouvrir | Sept préparées par la session optimiseur au 18 septembre ; **aucune envoyée**. Lucie, 2 octobre : d'accord pour les proposer, **après vérification que l'amont n'a pas déjà corrigé** — vérifié le 2 octobre : **aucune des sept n'est corrigée en amont**, textes et correctifs prêts (`extension/rag3weaver/docs/optimiseur/2-octobre-2026-00h15/01-les-sept-pr-amont.md`, avec `patches/`). Reste : compiler contre leur branche principale, puis le mot de Lucie sur l'envoi (compte `L-Defraiteur`, adresse personnelle ; la 6 avant la 5). **Lucie, 3 octobre : oui à trois envois avec une issue d'abord, « mais attention à pas paraître trop robotique »** — étudier d'abord leurs PR et issues, et leur position sur les contributions aidées par une IA ; textes réécrits dans leur forme, relus par elle avant tout envoi. |
| lucivy | crates.io | 4.3.0 utilisée par `mtg-experiments`. |
| Amont Vela | remote `vela`, branche `storage/concurrent-checkpoint-recovery` (27 septembre) | En cours de relecture par la session cœur C++ (2 octobre), voir « Un seul écrivain à la fois » au §3. Le WAL illisible, lui, venait de notre bug d'écriture, corrigé (§6). |

## 6. Bugs connus, non corrigés

- ~~Un backend qui déclare un signal creux n'a pas d'embarqueur creux~~ —
  **corrigé le 3 octobre 2026** (lot 3 de « un modèle, en service ou en
  local »). `models.sparse` se déclare comme les autres ; le backend branche
  le creux, et le dual quand c'est le même modèle des deux côtés. Un signal
  `sparse` déclaré sans `models.sparse` est refusé au démarrage en disant quoi
  écrire. Preuve sur un vrai backend : `scripts/test_backend_sparse.py`.
  **Reste** : les graphes de recherche des gabarits de backend
  (`search_structured.mmd`) n'ont pas de branche `sparse` — le signal est
  branché, mais il faut un graphe qui l'interroge (`search_base.mmd` l'a).
- ~~Un backend n'a jamais de relecteur ni d'OCR~~ — **corrigé le 3 octobre
  2026** (lot 4). `models.rerank` et `models.ocr` se déclarent comme les
  autres ; leur seule forme aujourd'hui est `local` (burn, dans le processus),
  un démon ou un tiers demandé est refusé en le disant. Preuve sur un vrai
  backend : `scripts/test_backend_models_local.py` (une recherche relue,
  l'OCR chargé). Depuis le lot 5 (même jour), le démon d'embarquement
  porte aussi leurs routes (`/rerank`, `/ocr`) : `provider: service` les
  trouve par le modèle déclaré, parmi les adresses de l'embarquement.
- **Seul l'embarquement sait dire « je calcule ailleurs »** : le relecteur,
  l'OCR et le LLM n'ont pas d'équivalent de `distant()`. Le régulateur de
  rafale, la classe de carte et l'estimation en ont besoin ; `model_source`
  rend désormais l'origine du calcul avec le client (lot 1, fait pour
  l'embarquement dense).

- **Une base plantait à l'ouverture après la mort d'un processus qui écrivait dans une
  table à index vectoriel : garde 1 livrée le 3 octobre 2026 (`fcd9a7882`), garde 2 à
  faire.** Mode en service, un seul écrivain. Trouvé par le banc de concurrence, confirmé
  sur le chemin réel de rag3weaver par l'arrêt brutal d'un catalogue (`e2e_arret_brutal`,
  session mémoire). **La condition exacte** : le rejeu du journal a lieu dans le
  constructeur de `Database`, avant tout `LOAD EXTENSION` de l'appelant ; il n'a
  l'extension que si le journal porte encore l'enregistrement `LOAD EXTENSION` de la session
  morte. Un point de reprise vide le journal, cet enregistrement compris — et
  `CREATE_VECTOR_INDEX` comme `COPY` en écrivent un d'eux-mêmes. Après lui, le rejeu d'une
  écriture rencontrait un index connu de la table et pas chargé : pointeur nul dans
  `NodeTable::initInsertState`. Vérifié sur quatre bases conservées : le chemin de
  l'extension présent dans le journal ⟺ la base s'ouvre. Défaut d'origine (c'est l'amont
  qui journalise le chargement) ; Ladybug a gardé les chemins d'écriture (`d2db8acb4`,
  `1ba0cc540`), gardes reprises ici. **Ce que fait la garde 1** : au rejeu, l'index non
  chargé est détaché de la table — la ligne entre, l'index n'est plus écrit au point de
  reprise, seule son entrée au catalogue reste. Cet état est « à rebâtir » :
  `QUERY_VECTOR_INDEX` et `CREATE_VECTOR_INDEX` (même `skip_if_exists`) refusent par
  l'erreur nommée **`is behind its table`** (`HNSWIndexUtils::INDEX_BEHIND_ITS_TABLE`),
  `DROP_VECTOR_INDEX` le retire, après quoi on le recrée. Chez Ladybug l'index sauté se
  recharge tel quel, faux sans le dire. Dix cas par SIGKILL et processus neuf
  (`test/transaction/vector_index_crash_reopen_test.cpp`), sept témoins verts au banc.
  **Ce qui reste** : rag3weaver doit reconnaître le nom à l'ouverture et rebâtir de
  lui-même (session de l'arbre principal) ; et la garde 2, sans laquelle chaque mort de ce
  genre coûte un index entier à rebâtir.
- **Le rejeu d'une mise à jour de nœud n'informait aucun index** : corrigé avec la garde 1
  (`fcd9a7882`). `WALReplayer::replayNodeUpdateRecord` appelait `update` sans
  `initUpdateState` ; un index chargé gardait l'ancienne position du vecteur.
- **Une transaction relisait de travers ses propres relations, et écrivait faux à partir
  de là : corrigé le 4 octobre 2026 (`c8fdaf196`).** Défaut du moteur d'origine, sans
  rapport avec l'index vectoriel, atteignable par Cypher. Une transaction supprime des
  relations d'un nœud qui sont **sur disque** (écrites par un point de reprise), en crée de
  nouvelles depuis ce nœud, puis relit : elle recevait les survivantes, puis la même relation
  locale répétée à la place des nouvelles (`{ 1, 3, …, 59, 100, 100, 100, … }`). **Ce
  n'était pas qu'une lecture fausse** : dans la même transaction, un `DELETE` des relations
  qu'elle venait de créer en laissait, et un `MERGE` des mêmes en créait en double — la base
  restait fausse après le `COMMIT`. **Une base existante peut donc porter des relations en
  trop ou en double** si une transaction explicite a supprimé, créé puis réécrit les
  relations d'un même nœud ; aucun outil ne le détecte aujourd'hui. **Ce qui n'est pas
  touché** : des relations validées encore en mémoire ; une transaction sans suppression —
  un `MERGE` par lot depuis un même nœud, sans `DELETE`, est juste ; rag3weaver, qui valide
  chaque instruction seule, n'y passe pas, sauf par l'index vectoriel, qui supprime et
  recrée ses arêtes à chaque mise à jour d'un vecteur. La cause : le balayage des relations
  sur disque laisse le vecteur de sélection en mode filtré quand la transaction en a
  supprimé, et `LocalRelTable::scan` n'en changeait que la taille. Ladybug avait posé la
  même ligne (`ffe855873`, 23 avril 2026) ; Vela non. Témoin :
  `test/transaction/uncommitted_rels_scan_test.cpp`, six cas, et la suite
  `UncommittedRelations` du banc. **Leçon du témoin** : la première version était verte
  avant comme après, parce que ses relations étaient encore en mémoire — il manquait un
  `CHECKPOINT`.
- **La mise à jour d'un vecteur perdait des lignes dans l'index : deux causes corrigées le
  4 octobre 2026 (`c8fdaf196`), une troisième ouverte.** En service, sans arrêt ni rejeu.
  Mesure d'origine sur master (1000 lignes de dimension 4, recherche exhaustive depuis une
  sonde, lignes joignables) : `SET` depuis NULL, 1000 ; `SET` vers un autre vecteur pour les
  mille lignes, 969 ligne à ligne et 532 par lots de 512 ; vingt `SET` vers le même vecteur
  en une instruction, 998. **Ces comptes varient d'une passe à l'autre** (le tirage des
  niveaux de l'index : la session du banc a mesuré 779, 593, 772 pour la même variante) :
  ordre de grandeur, pas un seuil. Le chemin principal de rag3weaver — les morceaux insérés
  sans vecteur, puis les vecteurs posés — n'a jamais été touché ; le réembarquement d'une
  ligne gardée l'est, rare chez nous. **Corrigé** : `OnDiskHNSWIndex::update` garde
  joignables les anciens voisins de la ligne (la suppression le faisait depuis `13284a0fe`) ;
  et l'index relit juste ses arêtes dans une instruction à plusieurs lignes (l'entrée
  précédente). Au passage disparaît un échec des grosses transactions de mises à jour,
  « bitset::set: __position (which is 2049) >= _Nb (which is 2048) ». **Ouvert** : quand
  presque toutes les lignes d'une table sont mises à jour, il reste des lignes injoignables
  (865 et 491 dans une passe, 1000 et 964 dans une autre) ; l'élagage des voisins retire des
  arêtes entrantes à des nœuds que personne ne recontrôle. Il faut un passage en fin
  d'instruction — un `finalize` de la mise à jour, comme pour la suppression ; l'état de mise
  à jour de l'index est aujourd'hui recréé à chaque ligne. **Contournement sûr** : supprimer
  puis réinsérer la ligne ; en masse, retirer l'index, poser les vecteurs, le recréer.
  Repasser par NULL n'est pas sûr par lots (599 sur 1000 avant correction). Notre greffe
  (`98e35566a`) ; Ladybug n'a ni mise à jour ni suppression dans son HNSW.
  **Témoins au banc (4 octobre, `e8d646716`)** :
  `test/transaction/concurrence/vector_index_update_test.cpp`, un seul écrivain. Chaque
  cas est répété sur des tables neuves. L'invariant est double : toute ligne est joignable
  par une recherche exhaustive, et chaque ligne sort première sur son propre vecteur. Le
  second contrôle voit des pertes que le premier ne voit pas, y compris sur des lignes non
  mises à jour.
  - Après `c8fdaf196`, environ un essai sur deux perd encore des lignes, sous la marche
    « mise à jour de l'index : contrôle en fin d'instruction » de `known_red.txt` : lots de
    512, vingt `SET` distincts ligne à ligne, une même ligne cinquante fois, mise à jour
    puis suppression. Le `SET` ligne à ligne est probabiliste.
  - Une ligne lointaine dans un nuage serré, index bâti d'un coup sans aucune mise à jour,
    manque un essai sur trois.
  - Le chemin du produit est vert : depuis NULL, et le remplacement qui repasse par NULL
    ligne à ligne.
  - **`c8fdaf196` a beaucoup ralenti la mise à jour** : dix mille lignes par lots de 512
    passent de 25 s à 27 min, le ligne à ligne est multiplié par cinq. Signalé au cœur
    C++ ; ce cas ne tourne que sur demande (`CONCURRENCE_VECTOR_HEAVY=1`).
  - La comparaison `known_red` dure désormais 14 min 30 s.
- **`e2e_idempotent_registration` rougit environ une fois sur dix à la réouverture**
  (`register_entity_persists_and_reloads`) : « Runtime exception: Reading past the end of
  the file …/test.db.wal with size 0 at offset 0 ». Mesuré le 3 octobre : 2 rouges sur 20
  sur la bibliothèque de master d'avant `f5acca417`, 2 sur 20 avec ; 1 sur 9 chez la
  session de l'arbre principal. Le test ferme un catalogue puis rouvre la même base dans le
  même processus. Hypothèses non vérifiées : un journal de taille nulle laissé par la
  fermeture, que l'ouverture lit au lieu de l'ignorer ; ou deux `Database` sur le même
  fichier un instant, le point de reprise de fermeture de l'une vidant le journal pendant
  que l'autre l'ouvre. À regarder des deux côtés : le test ferme-t-il vraiment avant de
  rouvrir (arbre principal), et que fait l'ouverture d'un journal vide (cœur C++).
- **Quand `CREATE_VECTOR_INDEX` échoue en plein milieu, la connexion en auto-commit reste
  dans un bloc en échec** (« The transaction was rolled back… Send ROLLBACK to close it »)
  alors que le client n'a jamais envoyé `BEGIN` : la fonction ouvre sa propre transaction,
  et la marche T0 la laisse en échec. Un client qui n'a pas ouvert le bloc ne doit pas
  avoir à le fermer. Vu le 3 octobre, non examiné ; une suite de T0, à reprendre avec les
  verrous.
- **Les amonts : on les lit, on ne leur transmet rien, on ne copie pas** (décision de
  Lucie, 4 octobre 2026 : « Vela, ils ont pas l'air très sérieux, et Kuzu un peu mort, donc
  non on leur transmet rien » ; tout reste sous LRSL). Ladybug est sous MIT (« Kùzu Inc.,
  Ladybug Memory Inc. », lu dans son `LICENSE` au tag `ladybug-main-2026-08-31`) ; à ce
  jour aucun code de Ladybug n'est copié chez nous — des idées lues puis réécrites (les
  gardes des index non chargés), et une ligne trouvée indépendamment (la sélection des
  relations locales). La règle : chaque commit qui doit quelque chose à un amont dit ce qui
  vient d'où, « lu puis réécrit » avec le commit cité ; copier un bloc se demande avant, et
  c'est Lucie qui tranche l'ajout à `NOTICE`. Quatre correctifs qui nous manquaient y ont
  été trouvés (`e92346c97` et suite, `d2db8acb4`, `1ba0cc540`, `ffe855873`) : la session du
  banc fait un passage en lecture seule sur ses commits de stockage, avec un témoin rouge
  par correctif atteignable en service. Son histoire est séparée de la nôtre (espace de
  noms `lbug`, même arborescence) ; celle de Vela nous est commune jusqu'au 10 octobre 2025.
  **Fait le 4 octobre** : vingt correctifs manquants confirmés chez nous, chacun par un
  témoin rouge au banc (`UpstreamFixes`, `a10ee1c51`, marche « correctifs de l'amont à
  reprendre »). Le tableau complet est dans
  `extension/rag3weaver/docs/3-octobre-2026-23h31/banc-de-concurrence/03-revue-des-amonts.md`.
  Par gravité :
  - **une perte** : un point de reprise efface les relations des régions intactes d'un
    groupe de nœuds (Ladybug `0be6fa597`). La condition : une table de plus de 1 024
    nœuds, et toutes les relations d'un bloc de 1 024 supprimées entre deux points de
    reprise. Signature : les deux sens ne s'accordent plus.
  - **huit plantages**, dont :
    - le point de reprise impossible, à jamais, après `ALTER … DROP` ;
    - la lecture après le point de reprise d'une relation créée puis supprimée, qui
      concerne rag3weaver.
  - **deux blocages** : l'UUID nul, et le lecteur retenu par un `CHECKPOINT`.
  - **dix réponses fausses**, dont aucune forme n'est émise par rag3weaver (vérifié par
    l'arbre principal).
  Par ailleurs, deux recettes ne se reproduisent pas chez nous. Dix défauts ne sont pas
  atteignables sans faute d'E/S ou coupure ; parmi eux, la publication avant le journal et
  le répertoire jamais synchronisé.
  **Changement de comportement, 4 octobre** : un champ CSV `""` (entre guillemets) se lit
  désormais comme la chaîne vide, et non plus NULL. NULL reste le champ vide non cité, ou
  le mot déclaré par `null_strings`. Décision de Lucie ; `escaped_newlines.test` a changé
  d'attendu dans le même commit. Qui écrit du CSV sans `null_strings` et comptait sur
  `""` → NULL doit le savoir. rag3weaver n'est pas touché, ses COPY déclarent leur mot.
- **`UNWIND $items AS item MATCH (n {_uuid: item.champ})` balaie la table entière**
  (planificateur, trouvé le 3 octobre par la session cœur C++, non corrigé ; un
  contournement existe). Toute écriture par lot qui retrouve ses nœuds par la clé primaire
  à travers le champ d'une structure déroulée — `batch_set`, `batch_link_labeled` sous le
  seuil du `COPY` — coûte en proportion du nombre de **nœuds** de la table, pas du nombre
  de lignes du lot ni du nombre de relations. Le plan (`EXPLAIN`) : `SCAN_NODE_TABLE` de
  la table entière, `CROSS_PRODUCT` avec la liste, puis `FILTER` ; pour un lien, deux
  balayages et deux produits cartésiens. Mesuré en C++ seul (Release, base de test en
  mémoire, lot de 300, à 2 000 / 20 000 / 200 000 nœuds par table) :

  | Forme | Plan | Temps par lot |
  |---|---|---|
  | une clé par appel, `MATCH (n {_uuid: $u})` (300 appels) | recherche par l'index | 8 / 11 / 12 ms |
  | `UNWIND $items AS item MATCH (n {_uuid: item._uuid}) SET …` | `CROSS_PRODUCT` + `FILTER` | 2,3 / 16 / 169 ms |
  | `UNWIND $uuids AS u MATCH (n {_uuid: u})` | `HASH_JOIN` | 0,3 / 0,8 / 5,8 ms |
  | `MATCH (n) WHERE n._uuid IN $uuids` | balayage filtré | 1,7 / 17 ms |
  | **`UNWIND $items AS item WITH item._uuid AS u MATCH (n {_uuid: u}) SET …`** | `HASH_JOIN` | 1,1 / 1,7 / 6,5 ms |
  | lien, forme d'aujourd'hui (structure, `MERGE`) | deux `CROSS_PRODUCT` | 24 / 348 ms, échec à 200 000 |
  | **lien : `… WITH item.from_uuid AS f, item.to_uuid AS t MATCH (a {_uuid: f}) WITH a, t MATCH (b {_uuid: t}) CREATE (a)-[:REL]->(b)`** | `HASH_JOIN` | 2,0 / 2,8 ms, échec à 200 000 |

  La table de relations, elle, peut grossir sans rien changer (plat de 0 à 120 000
  relations) : la croissance mesurée par la session de l'arbre principal venait des tables
  de nœuds qui grandissaient pendant la passe. Les échecs sont « Buffer manager exception:
  … The buffer pool is full » dans une base de test dont le tampon fait 73 Mo : le plafond
  est celui du test, mais ces plans matérialisent une quantité qui croît avec la table.
  **La cause** : la recherche par l'index n'existe que pour une clé constante (littéral,
  paramètre : `isConstantExpression`, `src/optimizer/filter_push_down_optimizer.cpp`) ; un
  prédicat qui dépend de la ligne extérieure est planifié à part, joint par produit
  cartésien puis filtré (`planRegularMatch`, `src/planner/plan/plan_subquery.cpp`) ; et le
  rattrapage en jointure de hachage (`visitCrossProductReplace`) ne reconnaît qu'une
  colonne nommée, pas `STRUCT_EXTRACT(item, …)`. **Le contournement** : sortir les champs
  en variables simples par un `WITH` avant le `MATCH` (lignes en gras) ; pour un lien,
  enchaîner les deux `MATCH` par `WITH a, t` — côte à côte, le plan retombe sur le produit
  cartésien des deux tables. **Mieux, trouvé par la session de l'arbre principal
  (`b3db4244c`, `dialect::unwind_par_cle`)** : la structure `item` voyage, et chaque clé est
  extraite dans le `WITH` qui précède immédiatement son `MATCH` ou son `MERGE` ; le `MERGE`
  d'arête, la suppression d'arête et les formes sans étiquette passent alors tous par
  jointure de hachage. Le refus que j'avais noté ici comme un second défaut (« Cannot
  evaluate expression with type VARIABLE » avec `MERGE`) venait d'une clé extraite trop tôt,
  à travers deux `WITH` : pas un `MERGE` impossible, au plus un défaut de portée des
  variables, non examiné. Vérifier par `EXPLAIN` qu'on lit `HASH_JOIN` et non
  `CROSS_PRODUCT`. **Le vrai
  correctif** : une recherche par clé par ligne. Ladybug l'a faite (`e92346c97`, « Row-Driven
  Primary-Key Lookup for MATCH », sous le tag `ladybug-main-2026-08-31`, puis sept
  correctifs) ; son histoire est séparée de la nôtre, c'est un port et non un
  cherry-pick, et il ne couvre qu'un motif à un seul nœud. Quatre à six jours, estimation
  non étayée ; derrière les verrous. Brouillons de mesure :
  `~/.cache/rag3db-moteur-notes/croissance-relations/`.
- **Au rejeu du journal, un `DROP_VECTOR_INDEX` retirait l'index du catalogue mais pas de
  la table : corrigé le 3 octobre 2026 (`f5acca417`)** (moteur, cas minimal de la session
  de l'arbre principal ; code hérité de l'amont). Le rejeu retire maintenant les deux ; le
  symétrique n'existe pas, la création d'un index écrit elle-même un point de reprise. Un index écrit par un point de reprise,
  puis retiré, puis un arrêt sans fermeture : à la réouverture `SHOW_INDEXES` ne le montre
  plus, mais `CREATE_VECTOR_INDEX` lève « Index … is not loaded yet ». C'est le rouge
  intermittent d'`an_interrupted_bulk_load_is_repaired_when_the_catalog_reopens`, et
  rag3weaver retire puis recrée ses index autour d'un chargement en masse.
  `WALReplayer::replayDropCatalogEntryRecord` (`src/storage/wal/wal_replayer.cpp`) ne
  rejoue que l'entrée du catalogue ; le retrait de la table n'est fait qu'à l'exécution,
  par la fonction de l'extension. Cas déterministe au banc de concurrence :
  `IndexReopen.DropAfterCheckpointThenCrashLeavesAnUnloadedIndex` (rouge, dans
  `known_red.txt` ; il reproduit le défaut seulement si le processus meurt base ouverte).
- **Aucun test Rust de rag3weaver ne fait « écrire, mourir par SIGKILL, rouvrir dans un
  autre processus »** (relevé par la session de l'arbre principal, 3 octobre). Ses tests
  de reprise sont de trois familles :
  - **A. Réouverture dans le même processus.** L'extension y est déjà chargée, et le
    défaut du rejeu sans `LOAD EXTENSION` y est invisible. Ce sont
    `e2e_code::a_bulk_load_interrupted_by_a_caught_panic_is_repaired_on_reopen`
    (renommé le 4 octobre, ex-`an_interrupted_bulk_load_is_repaired_when_the_catalog_reopens` ; la mort
    y est une panique rattrapée, suivie d'une fermeture propre), `e2e_rouvrir` (deux
    tests), `e2e_idempotent_registration` (trois tests), `e2e_search::phase6_sparse_mmap_persistence`
    et `e2e_entites_derivees::la_dette_de_rendu_se_voit_en_base_et_se_rattrape`. Ils
    prouvent la persistance du schéma et la réparation après une panique rattrapée,
    **pas** la reprise après une mort.
  - **B. Processus neuf, mais après une fermeture propre** : les scripts
    `test_backend_*` et `test_chat_must_reopen`. Leurs `kill()` ne servent qu'au délai
    dépassé.
  - **C. Vrai processus tué** : `e2e_prise_atomique` et
    `e2e_rag3daemon::deux_processus_partagent_la_base_par_le_demon`. Ils éprouvent le
    partage entre processus, pas la reprise d'une base.
  **Suite décidée** : pas de seconde suite. Le scénario du chargement en masse
  interrompu (index retirés avant le COPY, mort au milieu, restauration attendue à
  l'ouverture) entre comme cas dans `e2e_arret_brutal`, la suite de la session mémoire.
  Le test du code sera renommé pour dire ce qu'il prouve : une panique rattrapée.
- **Une colonne ajoutée par `ALTER … DEFAULT NULL` fait refuser tout COPY qui l'omet**
  (moteur, cas minimal de la session de l'arbre principal, 3 octobre ; contourné côté
  rag3weaver, pas corrigé dans le moteur). Une table, puis `ALTER TABLE T ADD v <type>
  DEFAULT NULL` (FLOAT[4], STRING ou INT64), puis `COPY T (colonnes sans v)` : « Trying to
  a create a vector with ANY type. This should not happen. Data type is expected to be
  resolved during binding ». Sans `DEFAULT`, ou avec `DEFAULT ''`, le COPY passe. Chez
  nous, les morceaux d'une première indexation en plein texte omettent leur colonne de
  vecteurs : leur COPY retombait sur le MERGE ligne à ligne, sans un mot. Le dialecte
  rag3db n'écrit plus de `DEFAULT NULL`, puisque c'est déjà le défaut. Les bases déjà
  créées gardent la clause sur les colonnes existantes : le COPY ne s'y tente que sur une
  table vide, ce qui rend le cas rare. Les replis sont désormais comptés
  (`Catalog::take_bulk_load_refusals`, `SourceSyncReport.bulk_load_refused`), et
  `e2e_code_sync::une_premiere_indexation_ne_se_replie_pas_en_silence` échoue s'il s'en
  produit un.
- **L'index vectoriel peut laisser une ligne injoignable dès sa construction**
  (extension vector, trouvé le 3 octobre par la session cœur C++, non corrigé). Une
  ligne indexée qu'aucune recherche n'atteint : un trou de rappel silencieux, sans
  aucune suppression. Reproduction, déterministe : les cent premières lignes de
  `dataset/embeddings/embeddings-8-1k.csv` chargées par `COPY`, puis
  `CALL CREATE_VECTOR_INDEX('embeddings', 'e_hnsw_index', 'vec', metric := 'l2')`,
  paramètres par défaut ; une recherche exhaustive (`k = 100`, `efs := 500`) rend 99
  lignes, et chercher la ligne 13 par son propre vecteur rend trois autres lignes.
  **Ce n'est pas une proportion du corpus, c'est un cas limite** : le trou apparaît à 98
  et à 100 lignes, pas à 50, 90, 95, 99, 101, 105, 110, 120, 150, 200, 300, 500, 1 000 ;
  sur des jeux aléatoires, 1 puis 4 puis 0 ligne perdue à 100 lignes selon la graine,
  aucune à 1 000, 3 000 et 10 000. Indépendant du nombre de fils et de `pu` ; dépend de
  `ml` (`ml := 20` : rien ne manque) et d'`alpha` (`alpha := 1.0` : quatre lignes
  manquent). Le même trou apparaît quand l'index existe avant le `COPY` (insertion sur
  disque, en lot), pas quand les lignes sont insérées une à une : **c'est l'élagage
  différé des voisins**, commun aux deux chemins en lot, qui retire à un nœud toutes ses
  arêtes entrantes. Cause exacte non élucidée ; code de l'amont. Une vérification
  d'atteignabilité en fin de construction coûterait une recherche par nœud élagué — le
  correctif de la suppression (`13284a0fe`, `keepNodeReachable`) fait exactement cela
  pour les voisins d'un nœud supprimé et peut servir de modèle. Le banc en a fait un
  cas rouge (`MinimalReproduction.HnswBuiltOnTheFirstHundredRowsLosesANode`, branche
  `banc-hnsw`). C'est aussi pourquoi le test Rust des suppressions partielles ne doit
  pas exiger que chaque survivante se retrouve elle-même sur un jeu de cent lignes : il
  compare désormais à un index bâti à neuf (session de l'arbre principal).
  **Reproduction plus simple, 3 octobre au soir** : 500 lignes de dimension 4
  (`[i, 2, 3, 4]`), une ligne mise à `[9000, 2, 3, 4]`, `CREATE_VECTOR_INDEX` ; chercher ce
  vecteur rend la ligne 499, à distance 8 501. Une ligne très loin des autres est
  injoignable dans un index bâti d'un coup ; avec `[250.5, 2, 3, 4]` elle est trouvée.
- **`QUERY_VECTOR_INDEX … RETURN count(*)` rend toujours `k`** (extension vector,
  3 octobre), même quand l'index a moins de `k` lignes : le résultat est complété
  jusqu'à `k` avant la jointure avec la table. `count(node.id)` est juste.
- **L'élagage immédiat des voisins plante dans le chemin de suppression** (extension
  vector, 3 octobre) : appeler `createRels` depuis `finalizeDelete` pour ajouter une
  arête à un nœud déjà plein mène à un SIGSEGV dans `shrinkForNode`. Non élucidé ; le
  correctif `13284a0fe` l'évite en écrivant les arêtes directement.
- **Les points d'entrée de l'index vectoriel vivent hors transaction** (extension
  vector) : c'est la cause du défaut du `ROLLBACK` corrigé le 3 octobre par un
  contournement (`e64839489`), et de la course que décrit
  `docs/3-octobre-2026-15h47/02-hnsw-sous-plusieurs-ecrivains.md`. La vraie correction
  est la maintenance de l'index au commit.
- **L'extension de plein texte range des offsets provisoires** dans ses tables internes
  (identifiants de document en entiers) : la marche A2 ne les remappe pas. Elle n'est
  dans aucun build et rag3weaver ne s'en sert pas.
- **Une colonne nulle sur toutes les lignes d'une liste de paramètres est
  lue comme STRING** (moteur, trouvé le 3 octobre par `e2e_undo`).
  Reproduction : `CREATE NODE TABLE T(id STRING PRIMARY KEY, n INT64)`,
  `CREATE (:T {id: 'a'})`, puis `UNWIND $items AS i MATCH (t:T {id: i.id})
  SET t.n = i.n` avec `$items = [{id: 'a', n: NULL}]` : « Binder exception:
  … STRUCT_EXTRACT(i, n) has data type STRING but expected INT64 ». Latent
  pour tout champ typé nullable. Contourné dans les annulations de
  `DeleteRecordNode` et `UpdateRecordNode` (`0709e3cba`) ; la correction est
  au moteur. Rejouée le 3 octobre par la session cœur C++ : la cause est
  qu'`UNWIND` remplace tout type inconnu par STRING et que `SET`, à la différence
  de `CREATE`, n'accepte qu'une conversion implicite (détail et décision au §4).
  Le cas voisin des littéraux — `UNWIND [{v: 5}, {v: NULL}]`, refusé à la
  liaison — est corrigé (`17c1ae41d`) ; celui des paramètres par `SET` ne l'est pas.
  Le « 0 au lieu de NULL » vu par la migration v8 n'est **pas** un défaut du
  moteur : un `ALTER TABLE … ADD n INT64` nu rend bien NULL (vérifié) ; c'est
  `SchemaDialect::alter_add_column` qui pose la valeur par défaut du type.
- **L'identité d'une ligne ne dépend pas de la cellule** (crate) : `uuid_for`
  hache les champs `hashsafe` seuls. La même clé ingérée dans deux cellules
  donne **la même ligne**, dont la seconde ingestion réécrit `_org` /
  `_project`. `e2e_scope` l'évite par des noms distincts par cellule. La
  synchronisation, bornée à la cellule, n'y ajoute rien ; mais deux cellules
  ne peuvent pas porter deux lignes de même identité. À trancher si un produit
  le demande.

- **Les défauts du mode multi-écrivains et des transactions, prouvés par le banc**
  (`test/transaction/concurrence/`, fusionné le 3 octobre) : la liste à jour est
  `known_red.txt`, les rouges déterministes, et `probabilistic.txt`, les rouges
  qui dépendent de l'ordonnancement. On y trouve : la clé primaire en double (C1),
  la relation pendante (C2), les relations rattachées aux mauvais nœuds (C3), la
  suppression et la mise à jour d'une même ligne validées toutes deux (C6, attendu
  en suspens jusqu'à la note sur les verrous), la double suppression sans conflit
  (C5, course, marche A5) et la transaction en échec qui repasse en auto-commit.
  Les courses vues par ThreadSanitizer sont dans
  `docs/2-octobre-2026-00h36/02-threadsanitizer-premiere-passe.txt`. Les virements
  qui ne conservaient pas la somme (étape 2) étaient un défaut **du banc**, pas du
  moteur : corrigé.
- **MODE EN SERVICE — une base dont un index HNSW a été créé dans la session qui meurt
  fait planter le processus qui l'ouvre** (banc, famille « arrêt brutal, un seul
  écrivain », 3 octobre au soir ; déterministe). Un index créé dans la session, même
  suivi d'un CHECKPOINT, puis une insertion ou une suppression dans la table, puis la
  mort : à la réouverture, SIGSEGV dans `NodeTable::initInsertState`, au rejeu du
  journal, dans le constructeur de `Database`, donc avant tout `LOAD EXTENSION`. Un index
  créé dans une session précédente ne le provoque pas. Pour rag3weaver, c'est le premier
  chargement : il crée l'index puis continue d'insérer, et une mort avant la fermeture
  propre laisse une base qui ne s'ouvre plus. Il n'est visible qu'en ouvrant la base dans
  un processus neuf : un processus qui a déjà chargé l'extension la garde. Cas minimal et
  pile chez la session cœur C++. Les autres correctifs de reprise tiennent sous un vrai
  arrêt : enregistrements de plus de 4 Ko, fin déchirée, mort à cinq instants d'un point
  de reprise, point de reprise échoué, COPY tout ou rien, suppressions d'A5 (spécification
  du banc, §13). **Condition élargie** (session cœur C++) : ce n'est pas la création
  de l'index dans la session morte, c'est un journal qui ne porte plus le `LOAD
  EXTENSION` — n'importe quel point de reprise après son chargement, puis une insertion
  ou une suppression dans la table indexée, puis la mort. La mise à jour d'un vecteur,
  elle, se reprend juste. Témoins au banc, avec une seconde mort et les attendus des deux
  gardes (« à rebâtir » nommé, puis index juste sans rebâtir), sous la marche à part
  « reprise avec index d'extension » de `known_red.txt`.
- **Une clé primaire en double validée sous le mode multi-écrivains rend la base
  impossible à rouvrir après un arrêt brutal** (banc, C1, variante Crash, 3 octobre au
  soir). C'est déterministe : le rejeu refuse le doublon (« Found duplicated primary key
  value 7 »). Cela **corrige** ce que le banc disait depuis l'étape 2 (« le rejeu
  réinsère le doublon sans erreur ») : la variante Crash fermait la base proprement avant
  de tuer le fils, et ne rejouait aucun journal. C'est corrigé, avec un témoin
  (`journal-to-replay`). **Cela ne se produit qu'en mode multi-écrivains**, éteint hors
  du banc : avec un seul écrivain, une clé en double ne peut pas être validée. C'est
  rangé sous A3′ : le verrou de clé empêche le doublon de naître (C1, et C7 en
  variante Crash, rouge 20 fois sur 20). Un second témoin,
  `LockBench.RecoveryOfAJournalWithADuplicateKeyKeepsTheDatabaseOpen` (rouge), demande
  à la reprise de refuser la transaction fautive en nommant la clé, plutôt que de rendre
  la base inouvrable. Audit du 3 octobre au soir, toutes les variantes Crash rejouées
  avec un vrai arrêt (spécification du banc, §12) :
  - C1 passe de « doublon visible » à « base perdue » ;
  - C7 passe de probabiliste à rouge certain ;
  - C2 en DETACH, relation validée d'abord, voit le rejeu réparer la relation pendante ;
  - C5 est vert depuis A5, pas à cause de l'arrêt ;
  - le reste est inchangé.
  La phrase « C1 et C7, sans index, valident des clés en double mais se rouvrent » (plus
  haut, cœur C++) reposait sur la fausse variante.
- **Sous le mode multi-écrivains, une table indexée par HNSW peut rendre la base
  inutilisable** (banc de concurrence, cas H4, 3 octobre). Quatre écrivains mélangent
  insertions, suppressions et nouveaux vecteurs sur une table qui porte un index HNSW.
  Sur 40 exécutions (20 à chaud, 20 avec arrêt brutal), le processus écrivain meurt 34
  fois : 30 SIGSEGV et 4 SIGABRT, dont une sur « corrupted double-linked list »
  (corruption du tas). Surtout, **19 bases ne se rouvrent plus** : le rejeu du journal
  bute sur « Found duplicated primary key value », une clé en double validée sous
  concurrence (la corruption de C1). Un doublon n'est donc pas seulement une ligne de
  trop : il peut rendre une base impossible à rouvrir. Graine 20261002, rejouable
  depuis le worktree du banc après `cmake -S . -B build/release -DBUILD_EXTENSIONS=vector`
  et le build de `concurrence_test` :
  `CONCURRENCE_GRAINE=20261002 ./build/release/test/transaction/concurrence/concurrence_test --gtest_filter='*H4_IndexedRandomMix/Thread_*' --gtest_repeat=20`.
  Le cas est probabiliste (`probabilistic.txt`). Les marches A3 (unicité au commit) et
  A5 (chemin de suppression), et la maintenance de l'index au commit, sont celles qui le
  concernent.
- **Après un point de reprise échoué, le processus qui continue perd des clés
  en silence puis plante** (mesuré sur `master`, 2 octobre ; patch
  d'expérience dans `…/2-octobre-2026-01h07/moteur-concurrence/`). **Corrigé
  côté moteur le 3 octobre** : le gestionnaire de transactions retient
  l'échec, et tout début de transaction ou point de reprise — celui de la
  fermeture compris — est refusé sous le nom
  `TransactionManager::REOPEN_AFTER_FAILED_CHECKPOINT` jusqu'à la réouverture,
  comme PostgreSQL (PANIC puis reprise par le journal). Les lectures fausses,
  la corruption durable (301 lignes pour 276 clés) et le gel à la fermeture ne
  se produisent plus ; un `COPY` dont le point de reprise échoue est atomique.
  Livré depuis `refus-apres-point-de-reprise-echoue` (`6d0c540f5`, sans son
  premier commit, déjà sur `master`) : transaction_test 65/65, api_test
  102/102, copy_tests 19/19, les huit suites de stockage, `known_red` vert ;
  Rust ciblé — lib 1111, `e2e_prise_atomique`, `e2e_checkpoint`,
  `e2e_chemin_de_masse` ; le reste non rejoué, le changement ne joue que sur
  le chemin d'un point de reprise en échec. **Le côté rag3weaver est fait
  aussi** (3 octobre, option B) : le nom est reconnu en un seul point
  (`Rag3dbConnection`), la base empoisonnée pour toutes ses connexions, et le
  catalogue refuse chaque verbe par `CatalogError::MustReopen` en comptant la
  file non drainée qu'il perd. **Les hôtes sortent avec 75** (`EX_TEMPFAIL`,
  `connection::EXIT_MUST_REOPEN`) après avoir répondu l'erreur :
  `rag3weaver-backend` (`mustReopen: true` dans sa réponse) et `rag3daemon`,
  dont le client reconnaît le nom à travers le fil. **`rag3weaver-chat`
  relance son backend une fois**, le dit dans le résultat de l'outil, ne
  rejoue pas l'appel, et ne relance pas une seconde fois si la panne se
  répète ; `chat_app.py` lance le chat, rien à y changer. Rien n'est retenté
  ni rejoué par le crate : un COPY non validé se relance par son appelant,
  l'ingestion étant idempotente. Éprouvé par crochet de test (la vraie panne
  l'est côté C++) : `e2e_rouvrir`, `e2e_rag3daemon`,
  `test_backend_must_reopen.py`, `test_chat_must_reopen.py`.
- **`les_lots_stables_comptent_des_puissances_de_deux` (lib) échoue sous le
  régime doux** (3 octobre) : il appelle `lot_budget`, qui lit
  `RAG3WEAVER_EMBED_CHAR_BUDGET`, que toute passe pose à 4096 ; seul, il
  échoue à chaque fois avec la variable et passe sans. Les passes d'avant
  étaient vertes par chance d'ordre : un test voisin retire la variable en
  parallèle. Défaut d'isolement du test, pas du code ; d'ici sa correction,
  la suite lib se joue sans cette variable (elle n'embarque rien sur la
  carte).
- **Une fin de synchronisation retirait une ligne sortie de son périmètre
  entre le plan et l'application** (trouvé le 3 octobre en répondant à la
  revue de la session lifecycle) : l'identité d'une ligne ne dépend pas du
  périmètre, la session unique par périmètre ne la protégeait donc pas.
  **Corrigé** : l'application relit le périmètre et la cellule, sur les deux
  chemins. **La classe des écritures hors session est fermée aussi** (une
  ligne réécrite ou recréée dans le périmètre, ou écrite pendant la session
  avant le plan, gardait une marque qui n'était pas celle de la session et
  était retirée) : une écriture dans un périmètre en session prend la marque
  de cette session (`{session}+w`), comptée à part (`written`), une marque
  qui ne descend jamais, lue par `mark_verdict` seul ; relu par la session
  lifecycle. **Le prix, assumé** : un écrivain qui réécrit périodiquement une
  ligne que la source n'a plus empêche son retrait, sans bruit. Une écriture
  en Cypher brut ne marque pas.
- **Les relations d'une première indexation, en masse ; le seuil du COPY des
  liens à 200 ; un COPY de liens qui nomme ses colonnes** (3 octobre, soir).
  *Décidé, et pourquoi* : Lucie — « gros dossier, on fait tout d'un coup » —,
  parce que les relations posées paquet par paquet partaient par MERGE, dont
  le coût grandit avec la base (sur `src/` du moteur, 2 à 5 s par paquet d'un
  tiers à l'autre). `sync_source` pose désormais, pour une source sans rien en
  base (`RelationsMode::Bulk`, le défaut alors), les nœuds et le plein texte
  par paquets et toutes les relations à la fin, en une fois, avec une seule
  résolution par Symbol ; la mémoire est bornée (la file est posée par COPY
  au-delà de 200 000 liens) ; une marque `relations_pending:{cellule}:{source}`
  dit, dès le début, que les relations ne sont pas encore là. **La mesure a
  trouvé mieux en chemin** : le seuil du COPY des liens à 200 au lieu de
  2 000 rend l'incrémental presque aussi rapide (48 s contre 92 s, durée par
  paquet presque plate) — c'est lui qui sert la synchronisation d'une source
  déjà indexée ; mesures dos à dos sous la charge du banc d'optimisation, à
  lire en ordre de grandeur. **Le COPY des liens nomme ses colonnes** : sans
  elles, deux colonnes texte d'une relation s'échangeaient sans erreur (relevé
  par la session codeparsers, test `e2e_copy_liens` rouge avant). Reste,
  au moteur : l'insertion par MERGE qui croît (session cœur C++). Vu une fois,
  sur une bibliothèque d'avant A5 : `an_interrupted_bulk_load_is_repaired_when_the_catalog_reopens`
  (e2e_code) rouge sur « Index … is not loaded yet » à la réouverture, puis
  vert trois fois et à la batterie suivante, sur la bibliothèque reconstruite
  — à surveiller. Ce n'est pas une forme de ce que les marches A5 ont corrigé
  (session cœur C++) : le message vient de `NodeTable::getIndex`
  (`src/storage/table/node_table.cpp`), l'index est au catalogue mais son
  contenu n'est pas chargé ; il l'est quand l'extension vector se charge
  (`extension/vector/src/main/vector_extension.cpp`), pour les index déjà au
  catalogue à ce moment-là. Hypothèse non vérifiée : une requête a touché
  l'index avant la fin de ce chargement, ou la réparation du chargement
  interrompu a retrouvé un index après lui. Non reproduit ; pas de correctif
  avant reproduction. Cas à ajouter au banc : rouvrir puis interroger l'index
  tout de suite, répété.
- **La suppression ne retirait jamais les vecteurs creux de l'index
  lucistore : corrigé le 3 octobre**, confirmé par exécution avec bge-m3
  avant (3 entrées sur 3 après un retrait ; 4 pour 3 chunks vivants après une
  mise à jour qui redécoupe). `DeleteRecordNode` et `RechunkDeleteNode`
  retirent désormais du handle les offsets des chunks avant de les supprimer
  (`retirer_le_creux_des_chunks`) ; test
  `le_creux_d_une_ligne_retiree_quitte_l_index`, éprouvé par mutation.
- **Après une table vidée puis repeuplée par le chemin de masse, la recherche
  dense ne rendait rien : corrigé le 3 octobre**, dans **notre** greffe de
  suppression dans l'index HNSW (l'amont n'y supprime rien), par la session
  cœur C++ (`hnsw-point-d-entree-apres-suppression`). Trois défauts : le
  nettoyage de fin d'instruction ne tournait jamais (`finalize()` appelé sur
  l'opérateur d'origine, pas sur la copie qui s'exécute) ; le point d'entrée
  de l'index remplacé par un nœud mort, d'où la recherche vide ; le graphe
  coupé en morceaux après des suppressions partielles (899 sur 1 000 : 62 des
  101 survivants atteignables, les plus proches rendus faux, sans erreur) —
  le chemin ordinaire de toute réingestion qui supprime. Non-régression :
  `e2e_recherche_dense_apres_suppressions` (table vidée puis repeuplée ; cent
  notes dont quatre-vingt-dix supprimées, chaque survivante la plus proche
  d'elle-même, pas de résultat fantôme). **Ce que le correctif ne règle pas**
  (session cœur C++) :
  - `QUERY_VECTOR_INDEX … RETURN count(*)` rend toujours `k`, même avec moins
    de `k` résultats ; `count(node.id)` est juste. **Lu côté rag3weaver** :
    aucune requête ne compte par `count(*)` — toutes rendent
    `node._uuid, distance` et comptent les lignes en Rust
    (`rag3db_search_backend.rs`, `search.rs`), sans filtrer un uuid vide ;
    **exécuté** : `k=20` sur 10 survivantes rend 10 lignes, toutes
    identifiées. Pas de fantôme observé.
  - Un `DELETE` annulé par `ROLLBACK` laisse le point d'entrée et le compteur
    de l'index modifiés (lu, non exécuté). **rag3weaver n'ouvre aucune
    transaction explicite** (ni `BEGIN` ni `ROLLBACK` dans le crate) ; son
    annulation passe par des instructions compensatoires. Il y reste exposé
    par une instruction `DELETE` en auto-commit qui échoue, que le moteur
    annule de lui-même.
  - Un nœud qui pointait vers un supprimé sans réciproque garde une arête
    morte.
- **Les poids ne dépendent plus d'aucun disque** (3 octobre 2026). Les
  granite d'origine du 6 septembre, rapportés de l'ancien poste par ssh, sont
  installés (suite granite 11 sur 11) et publiés à la place des régénérés dans
  les deux dépôts Hugging Face, passés en public le même jour, empreintes distantes conformes ; les
  régénérés restent dans `~/.cache/rag3weaver/regeneres-2-octobre/`. L'OCR
  `ppocrv6-tiny` était déjà publié, en public, contrairement à ce que disait
  la notice (corrigée) ; les sept autres modèles aussi.
- **Le mode multi-écrivains corrompt en silence — prouvé le 2 octobre 2026**
  par l'étape 1 du banc de concurrence (branche `banc-de-concurrence`,
  `37a44e351`, 20 passes sur 20). Sous `debug_enable_multi_writes` : deux ou
  trois écrivains insèrent la même clé primaire et tous valident (la clé
  existe deux et trois fois) ; une suppression de nœud et la création d'une
  relation vers lui valident toutes deux (relation pendante, comptée par
  `count(r)` mais invisible à toute requête qui lit une propriété de
  l'extrémité) ; deux écrivains qui créent des nœuds puis des relations entre
  les leurs voient les relations du second pointer vers les nœuds du premier.
  Aucune erreur dans aucun cas. **Ce mode n'est pas allumé en production** :
  ne pas l'allumer avant les marches A2, A3, A4 du plan
  (`docs/2-octobre-2026-00h17/01-…`, §7), qui corrigent ces trois cas contre
  ce banc.
- **Les poids granite ne sont ni sur ce poste ni publiés** (vu le 2 octobre) :
  `~/.cache/rag3weaver/` n'a que bge-m3, minilm et multilingual-minilm ;
  granite-278m (le modèle par défaut), granite-107m et l'OCR ppocrv6-tiny
  n'ont jamais été mis sur Hugging Face. Conséquence : le banc de recherche
  (`e2e_banc_etage`) ne donne aucune référence, et la mesure des poids de
  fusion attend. En cours : régénération depuis les ONNX d'IBM par la session
  optimiseur, avec preuve par écart absolu contre l'ONNX, puis publication en
  dépôts privés (accord de Lucie). Les originaux sont peut-être sur l'ancien
  poste : une note sur la clé USB de Lucie dit quoi y copier ; s'ils
  reviennent, ce sont eux qu'on garde.
- **`LecteursConcurrents.CeQueLeLecteurVoitEstCoherent` : corrigé à
  l'ouverture par la marche 1** (3 octobre). Une ouverture en lecture seule
  qu'un point de reprise a traversée est refusée par
  `WALReplayer::CHECKPOINT_CROSSED_READ_ONLY_OPEN`, refus transitoire qui se
  retente ; le test est vert cinq fois sur cinq. **Reste ouvert** : un lecteur
  qui *reste* ouvert pendant qu'un point de reprise passe, et deux points de
  reprise complets dans une seule ouverture — il y faut une époque écrite
  dans le fichier, c'est la marche 5. Historique :
  *était rouge sur `master`* (`api_test`, vu le 2 octobre) : le lecteur en lecture seule est
  refusé avec « Found duplicated primary key value » au lieu du seul refus
  attendu. Présent avec et sans les deux correctifs du journal, à chaque
  passe : il leur est antérieur, et aucun contrôle de livraison ne jouait
  `api_test` (il y entre désormais). **Rouge aussi à `20a8f6ee8`** (18
  septembre), 5 fois sur 5 sur cette machine : la course a toujours existé,
  c'est le Strix Halo qui la montre. Accepté comme antérieur et nommé à la
  livraison de la fin déchirée ; rien n'est corrigé, le test n'est ni
  désactivé ni relâché.
  **Cause, par lecture du code (session cœur C++, 2 octobre)** : un lecteur
  en lecture seule rejoue le journal d'un écrivain vivant (`database.cpp:136`
  → `StorageManager::recover`, sans condition sur `readOnly`) par trois
  lectures non atomiques — `dryReplay` décide, `readCheckpoint` lit le
  fichier de données, puis le rejeu. Si l'écrivain fait son checkpoint entre
  les deux premières, le lecteur rejoue des transactions déjà dans le
  fichier. Déduit et **non vérifié par exécution** : avec des relations (pas
  de clé primaire) le doublon serait silencieux, dans la mémoire du lecteur
  seulement. Vela a la même forme. Indépendant de la fin déchirée, qui se
  livre en nommant ce rouge ; le correctif est un choix de conception et
  entre dans le plan des écritures parallèles (§3).
- **Reprise après un point de reprise interrompu : corrigée** (2 octobre,
  `6bf46150b`, trouvée par la session cœur C++). Sur une table qui avait déjà
  connu des points de reprise, un point de reprise interrompu laissait l'index
  de clé primaire réécrit en place, et la base ne se rouvrait plus (« Found
  duplicated primary key »). Défaut hérité de Kuzu, que Vela et Ladybug portent
  encore. Livré après reconstruction de `build/lecteurs-csv` : transaction_test
  56/56, api_test 102/102 (la course des lecteurs concurrents ne s'est pas
  produite à cette passe), lib 1101, huit suites e2e, trois scripts du backend.
- **WAL illisible : la vraie cause est un bug d'écriture**, pas l'arrêt
  brutal (trouvé le 1er octobre 2026). `resizeBufferIfNeeded`
  (`src/storage/wal/checksum_writer.cpp`, même défaut dans
  `checksum_reader.cpp`) remplace le tampon de 4096 octets sans recopier ce
  qui y était : tout enregistrement de plus de 4 Kio perd son début, et sa
  somme de contrôle, calculée sur le tampon faux, ne voit rien. Un arrêt
  propre supprime le journal ; un arrêt brutal force le rejeu d'un journal
  déjà faux (`wal_record.cpp:79` ou `:76`). Hérité de l'amont (#5940), non
  corrigé chez Vela. **Le bug d'écriture est corrigé sur `master`** (`955b1b136`, étape D :
  recopie des deux côtés, message « journal corrompu », doc
  `docs/1-octobre-2026-23h37/01-…`). **La fin déchirée** (étape E,
  décidée par Lucie le 1er octobre) est sur `master` depuis le 2 octobre :
  rouvrir au dernier COMMIT, toute troncature copiée dans
  `<journal>.ecarte-<ms>` (par blocs) ; en lecture seule, rien n'est écrit ni
  dit. **Limite** : sans longueur par enregistrement, une
  longueur abîmée au milieu se lit comme une fin déchirée et fait écarter des
  transactions validées (copiées à l'octet près, rien n'est supprimé) ; seul
  un changement de format la lèverait. En attendant : arrêt par SIGTERM ou
  EOF, copie reflink avant une longue écriture, un seul processus par base.
  **La base MTG actuelle a un `.wal` déjà corrompu (30 enregistrements
  illisibles) : ne pas l'ouvrir, et jamais avec
  `throw_on_wal_replay_failure=false`** — le rejeu tronquerait le journal.
  Avec l'étape E, la raison s'ajoute : ses enregistrements abîmés peuvent se
  lire comme une fin de fichier, et l'ouverture écarterait des transactions
  validées. Lucie, 1er octobre : « on la laisse tranquille pour le moment ».
- **Persistance des `abilities` imbriquées** : après réouverture, des textes
  rattachés au mauvais élément. Bloquant pour les filtres sur ce champ.
- **SIGSEGV avec un buffer pool de 1 Gio** ; contournement : 8 à 15 Gio.
- La synchronisation MTG ne fait que des upserts : pas de suppression des
  cartes absentes d'un nouveau snapshot (chantier au §3).
- `list_filter.cpp:113` lit `inputVector.isNull(i)` au lieu de `pos`.

## 7. Ménage

- **Sur cette machine (ROG Flow Z13)**, les poids minilm et multilingual-minilm
  sont installés depuis le 1er octobre dans `~/.cache/rag3weaver/` (procédure
  de `extension/rag3weaver/generated/README.md`, sha256 vérifiés) : sans eux,
  16 tests e2e ne tournent pas et ne doivent pas être comptés verts.

- `git worktree prune` : trois worktrees dont les dossiers n'existent plus
  (`rag3db-embarquements`, `rag3db-recherche`, `rag3db-lifecycle`).
- Fichiers non suivis à la racine, à supprimer : `follows.csv`, `user.csv`,
  `user.parquet` (restes d'une démo du 7 septembre), `build-lecteurs-csv.log`,
  `build-rag3weaver.log`.
