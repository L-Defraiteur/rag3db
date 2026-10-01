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

Au 2 octobre 2026, deux branches en cours (`sparse-et-suppressions` fusionnée
le 2 octobre, `7c653f66c` et `01791e347`), chacune dans son arbre, avec la
session qui la porte (les noms `rag3db-xx` changent à chaque relance : se fier
au sujet) :

| Branche | Arbre | Session (sujet) | Ce que c'est | Où elle s'arrête |
|---|---|---|---|---|
| `lecteur-reverifie-a-l-ouverture` | `../rag3db-moteur` | cœur C++ | Marche 1 du plan des écritures parallèles : le lecteur d'un autre processus revérifie l'identité du journal et de l'en-tête après avoir lu le fichier de données ; quatre tests déterministes par un crochet de test. | Branche poussée, `api_test` vert en entier ; la fusion et les suites Rust attendent le contrôle de l'orchestration. |
| `banc-de-concurrence` | `../rag3db-banc` | banc de concurrence | Marche 2 : banc de concurrence et vérificateur d'intégrité (spécification dans la branche, `docs/…/01-specification-du-banc-de-concurrence.md`). Étape 1 : cas C0 à C3, qui tranchent les corruptions seulement déduites. | Après l'étape 1, compte rendu avant de continuer. Ses cas rouges restent rouges, sous le label `concurrence-rouge-connu`. |

Le plan : `docs/2-octobre-2026-00h17/01-ecritures-paralleles-vela-et-le-chemin.md`
(§12, l'ordre des marches) ; côté crate :
`extension/rag3weaver/docs/2-octobre-2026-00h16/01-ce-que-rag3weaver-suppose-d-une-seule-base.md`.

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
| Synchronisation par identifiant : supprimer les lignes disparues d'un snapshot | moteur (`ingest_snapshot` / `EntityBatchNode`) | Décidé avec Lucie le 1er octobre, après les étapes D et E du WAL. Générique ; aujourd'hui seuls les fichiers de code le font (`reingest_file`). |
| Champ `folds` des scopes | `1e5eea234` | Ré-ingérer le code pour le remplir. |
| Base MTG | poste | À reconstruire (environ 8 Go, dont 6 récupérables). |
| Récupération des lignes supprimées dans rag3db | proposé, pas fait | Les blobs d'index sont bornés par une purge côté rag3weaver (`d1aa7d296`) en attendant. |
| Budget de reprise du lecteur en lecture seule | `master`, mesuré (`20a8f6ee8`) | 250 ms de budget contre un pic à 567 ms sous charge : relever le budget, ou tester l'invariant par `read_only_patient`. Attend une décision. |
| Un modèle d'embarquement requalifié en ancien = une transition d'état déclarée | promis le 18 septembre à la session optimiseur, **jamais confié** | Attendait que `Lifecycle` soit appliqué à l'écriture : c'est fait (`780acfd2d`). À cadrer avec les sessions optimiseur et lifecycle. |
| Écritures parallèles | **objectif décidé par Lucie le 2 octobre** : « on fait ce qu'il faut pour écritures parallèles, peu importe ce que ça coûte » | **Cible tranchée par Lucie : « oui jusque B, et A d'abord si dans même chemin »** — B, plusieurs processus écrivains sur le même fichier ; A, plusieurs transactions d'écriture dans le processus qui tient la base, en première étape seulement si elle est sur la route de B. Plan demandé à la session cœur C++ (lecture seule) : ce que B réutilise de A, un ordre de marches testables une à une, et le lecteur d'un autre processus comme premier cas. Rien à coder avant que Lucie ait vu le plan. État d'aujourd'hui : Le second écrivain est **refusé**, pas mis en attente ; le checkpoint bloque les lecteurs. Ordre écrit le 6 septembre (`docs/6-septembre-2026-13h08/01-…`) : prendre Vela → mettre les écrivains en file → lots courts à l'ingestion → bien plus tard, deux processus écrivains. **Étude en lecture seule confiée à la session cœur C++ le 2 octobre** (Vela aujourd'hui, coût de fusion après nos deux correctifs du journal). Rien de décidé. |

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

## 4. Décisions en attente de Lucie

Posées le 18 septembre 2026, **tranchées par Lucie le 1er octobre 2026** :

1. La forme du pas C : **oui** — fusion pesée par entité, pondération par
   valeur de champ (un poids, pas un filtre), gabarits de dérivées. **Avec une
   exigence de Lucie : ces pondérations se règlent dans les graphes de
   recherche**, comme les poids de fusion (`FuseResultsNode(weights=…)`), pas
   seulement dans la config d'entité. Reste à dessiner : quand l'entité
   déclare une fusion, le gabarit ne la retouche pas aujourd'hui
   (`base_de_fusion`) — dire si un graphe peut forcer les siens.
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

7. Le budget de reprise du lecteur (§3).
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

## 5. Hors de ce dépôt

À confirmer par qui s'en souvient : ces lignes viennent de la mémoire des
sessions, pas d'une vérification.

| Chantier | Où | État connu |
|---|---|---|
| Forks burn, cubecl, cubek | `github.com/L-Defraiteur/{burn,cubecl,cubek}`, branche `rag3weaver/pre.3` | Utilisés tels quels (burn `21674205` depuis le 2 octobre). |
| Adresse professionnelle sur les forks | `L-Defraiteur/{burn,cubecl,cubek}`, branche `rag3weaver/pre.3` | **Corrigé le 2 octobre** sur décision de Lucie : les neuf commits qui portaient son adresse professionnelle sont recréés à l'adresse personnelle (mêmes diffs), branches poussées avec bail, `pre.2` supprimées, épinglage remonté sur `master` (`04b5052ec` : burn `21674205`, cubecl `bdf6b77a`, cubek `7ba8affd`). **Reste ouvert** : GitHub sert encore les anciens commits par leur hash ; un retrait certain demande le support GitHub ou la suppression et recréation des trois forks — au choix de Lucie. Reste aussi : bâtir une fois sur les nouvelles révisions. Tout commit de `rag3db` antérieur à `04b5052ec` ne se bâtira plus sur un poste neuf une fois ces anciens commits purgés. |
| PR amont burn / cubek | à ouvrir | Sept préparées par la session optimiseur au 18 septembre ; **aucune envoyée**. Lucie, 2 octobre : d'accord pour les proposer, **après vérification que l'amont n'a pas déjà corrigé** — vérifié le 2 octobre : **aucune des sept n'est corrigée en amont**, textes et correctifs prêts (`extension/rag3weaver/docs/optimiseur/2-octobre-2026-00h15/01-les-sept-pr-amont.md`, avec `patches/`). Reste : compiler contre leur branche principale, puis le mot de Lucie sur l'envoi (compte `L-Defraiteur`, adresse personnelle ; la 6 avant la 5). |
| lucivy | crates.io | 4.3.0 utilisée par `mtg-experiments`. |
| Amont Vela | remote `vela`, branche `storage/concurrent-checkpoint-recovery` (27 septembre) | En cours de relecture par la session cœur C++ (2 octobre), voir « Un seul écrivain à la fois » au §3. Le WAL illisible, lui, venait de notre bug d'écriture, corrigé (§6). |

## 6. Bugs connus, non corrigés

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
- **`LecteursConcurrents.CeQueLeLecteurVoitEstCoherent` est rouge sur
  `master`** (`api_test`, vu le 2 octobre) : le lecteur en lecture seule est
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
