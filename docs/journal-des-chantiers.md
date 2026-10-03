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

**Suite de la session cœur C++** : A5 d'abord — la course du chemin de suppression fait
planter le processus (§10 de la spécification du banc), et elle ne dépend pas des écarts
de la note sur les verrous. Puis, après la réponse de Lucie sur ces trois écarts
(`docs/3-octobre-2026-15h47/01-note-de-conception-les-verrous.md`) : le gestionnaire de
verrous, A3′, A4′, l'annonce en tête de transaction ; la maintenance de l'index vectoriel
au commit avant A6.

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
cent lignes qui perd un nœud sont rouges ; H4 est probabiliste (§6).

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
| Repli des KB en entités dérivées | `master`, pas A et B faits | **Pas C** : poids de fusion par entité, pondération par genre dans `Scope`, gabarits de dérivées au catalogue. Décidé (§4) ; **la mesure des poids de fusion ne commence pas** avant la référence du banc `e2e_banc_etage` en granite-278m, avant / après les suppressions — avant = `a66bb0b9d`, après = `01791e347` (45 → 43 questions : les deux sur le parcours en largeur n'ont plus de cible) —, à jouer dès que les poids granite-278m sont sur ce poste. |
| Chemin de masse des lots de naissances | `fa70cf8f3`, désactivé (`RAG3WEAVER_COPY_NAISSANCES`) | Trouver pourquoi le `COPY` des chunks croît avec la table. Pistes : reconstruction de l'index vectoriel à chaque lot (`ajuster_l_index_pour_le_retard`), relecture `select_node_ids`. |
| Deck builder MTG (produit) | `experiments/mtga`, `master` | Descriptions d'outils propres à chaque entité (description d'entité dans le manifeste, au lieu du même texte pour tous les `search_*`) ; compter artefacts et créatures de mana comme sources de couleur dans le harnais (accordé, pas fait) ; barre de défilement du chat dont la taille ne suit pas la liste (capture attendue) ; option Gemini via Vertex. |
| Synchronisation par périmètre | **première étape sur `master` (3 octobre)** : `SnapshotConfig` (périmètre, `maxMissingRatio`, `onMissing: delete \| {transition}`), une session à la fois par périmètre et par cellule (`begin_snapshot` rend l'identifiant, `takeover`, `abort_snapshot`), marque `_snapshot`, fin en deux temps (`plan_snapshot_finish` / `apply_snapshot_finish`) et ses garde-fous, marque d'absence `_absent_since`, schéma v8 ; `tests/e2e_synchronisation.rs`, `scripts/test_backend_snapshot.py`, `scripts/test_migration_v8.py` | **Seconde étape faite (3 octobre)** : la mise de côté avant purge (`keepFor`, 7 jours par défaut, `0` pour la couper ; `_snapshot_aside`, le retour sans réembarquement, la purge bornée par SET) et l'annulation en bloc d'une fin (`undo_snapshot_finish`, outil `undo_snapshot`) — ce qui est codé et ses écarts : §8 de `extension/rag3weaver/docs/3-octobre-2026-15h04/01-mise-de-cote-avant-purge.md`. Reste : rebrancher `reingest_file` (dette de généricité ci-dessous) ; PostgreSQL sans vecteurs mis de côté. |
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

- `code_tools.rs`, `reingest_file` : le seul chemin qui supprime aujourd'hui
  les lignes disparues ; entité `SCOPE` et champs `file_path`, `key`,
  `source` en dur. À remplacer par la synchronisation par périmètre déclaré
  (ci-dessus, « supprimer les lignes disparues »).
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
2. Les poids de fusion par défaut : **mesurer avant de choisir**, au banc, aux
   deux réglages (0,6 / 0,4 du gabarit, 0,3 / 0,7 d'avant) — **et avec le
   signal sparse**, que le banc n'a jamais mesuré et que le gabarit laisse au
   défaut du moteur (0,2). Avant de mesurer : `sparse-vector` est figée en
   4.0.1 alors que lucivy est en 4.3.0 et que `sparse-vector` 4.3.0 est
   publiée depuis le 13 septembre ; aligner d'abord.
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
