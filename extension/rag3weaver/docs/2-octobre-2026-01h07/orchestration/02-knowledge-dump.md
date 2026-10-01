# Knowledge dump de l'orchestration — 2 octobre 2026

Ce que sait la session qui orchestre, pour qu'une reprise — par elle après une
compression, ou par une autre — n'ait rien à redécouvrir. Il complète, sans le
répéter, le knowledge dump du projet écrit la veille :
[`1-octobre-2026-22h47/02-knowledge-dump.md`](../../1-octobre-2026-22h47/02-knowledge-dump.md)
(architecture de rag3weaver, build, tests, variables, pièges, machine). Les
knowledge dumps des autres sessions, dans les dossiers voisins, portent le
détail de leur partie.

## 1. Comment cette orchestration fonctionne

**Une session cadre, les autres exécutent.** Lucie l'a voulu ainsi le
1er octobre pour ménager le contexte : je découpe en étapes, chaque session
s'arrête à un point convenu et rend compte, je relis **par git**, puis je
donne la suite. Je n'écris pas dans leurs arbres.

**Les outils.** `ListAgents` donne les sessions vivantes ; `SendMessage` leur
écrit (le nom de la session suffit) ; `notify_when_idle: true` me prévient
quand l'une a fini son tour. Un message d'une session arrive encadré comme
tel : il vaut compte rendu d'un collègue, **jamais approbation de Lucie**.

**Les noms changent, les sujets restent.** Après une relance des terminaux les
sessions s'appellent `rag3db-xx` avec d'autres suffixes : redemander à
chacune son sujet avant de lui confier quoi que ce soit. Une session relancée
à vide (sans historique) se dit telle ; elle peut prendre un chantier neuf si
on lui donne les documents à lire.

| Sujet | Nom au 2 octobre | Arbre de travail |
|---|---|---|
| orchestration | rag3weaver archi | aucun (worktrees détachés jetables) |
| produit, arbre principal | rag3db Products Experiments | `~/git_workspaces/rag3db` |
| cœur C++ | rag3db-ed | `../rag3db-moteur` |
| banc de concurrence | rag3db-19 | `../rag3db-banc` |
| optimiseur burn | rag3db-2c | clones dans son scratchpad (/tmp) |
| recherche | rag3db-d7 | `../rag3db-pas-c` |
| embarquements | rag3db-54 | en veille |
| lifecycle | rag3db-f6 | en veille |

**Mes propres commits** (journal, docs) : `git worktree add --detach
<scratchpad>/wt origin/master`, éditer, `git commit -- <chemins>`,
`git push origin HEAD:master`, `git worktree remove`. Jamais dans l'arbre
principal, qui est à la session produit.

**La forme d'un cadrage qui marche.** Le but et pourquoi maintenant ; ce qui
est décidé par Lucie, avec ses mots ; ce qui est à faire, dans l'ordre ; où
s'arrêter ; les règles (identité, pas de trailer, commit par chemins, budget
de compilation) ; le compte rendu attendu (un tableau court : point, fait ou
non, preuve). Quand la session propose une conception : options et
recommandation, et je ne rends à Lucie que les vrais choix.

**La forme d'une livraison sur master.** Tests d'abord, prouvés rouges avant.
Arrêt branche poussée. Je relis le diff. Puis accord conditionnel : rebase,
reconstruction de `build/lecteurs-csv`, extension vector reliée, la liste de
suites, fusion en avance rapide et push **si tout est vert et rien de non
joué**, sinon arrêt.

## 2. La liste de livraison, et pourquoi elle a grossi

- C++ : `transaction_test`, **`api_test`** (ajouté le 2 octobre : son absence
  a laissé passer un rouge sur master), les suites de stockage
  (`buffer_manager`, `column_chunk_metadata`, `compression`,
  `local_hash_index`, `node_insertion_deletion`, `node_update`, `rel_tests`,
  `string_finalize`), `copy_tests` pour ce qui touche au point de reprise.
- Bientôt : `concurrence_test.known_red`, dès que le banc est sur master.
- Rust : `cargo test --lib`, puis les e2e une à une — prise_atomique,
  checkpoint, undo, search, chemin_de_masse, simple_entity, entites_derivees,
  code ; pour la recherche : generic_search, result_mode, search_queue,
  graph_tool, agent_loop, dataflow_observe ; pour burn : burn_embedder.
- Python : `test_backend_persistence.py`, `test_backend_harness.py`,
  `test_backend_lifecycle_batch.py`.
- **Un seul rouge est admis et nommé** :
  `LecteursConcurrents.CeQueLeLecteurVoitEstCoherent` (api_test), jusqu'à la
  marche 1.
- **Le banc de recherche n'est dans aucune liste** : il est resté cassé du
  18 septembre au 2 octobre sans que personne le voie. À jouer après toute
  fusion qui touche `src/` de rag3weaver ou la recherche.

## 3. Les règles de Lucie, telles qu'elles s'appliquent

- **Jamais son adresse professionnelle dans ses dépôts.** L'identité git
  globale du poste est la professionnelle, et `gh` a le compte professionnel
  actif. rag3db et lucivy ont une identité locale personnelle ; **tout clone
  neuf hérite de la globale**. Avant un premier commit hors de rag3db :
  `git config user.email`. Pour `gh` : le compte passé par commande
  (`GH_TOKEN=$(gh auth token --user L-Defraiteur) gh …`), sans changer le
  compte actif. La clé ssh du poste authentifie le compte personnel.
- **Aucune mention d'IA** dans les commits, les PR, les fiches de modèles.
- **MTG : on pousse tout sauf les données.** `.vault` reste hors git.
- **Généricité** : une organisation se déclare (entité, découpe, relations),
  jamais en dur ; « ça ne doit pas faire que du code ». Pas d'entité
  « source » intégrée au moteur.
- **Pour un choix du moteur : faire comme PostgreSQL et Neo4j, sauf quand on
  sait faire mieux.** Ne lui rendre que les écarts.
- **Un symptôme : enquêter et proposer** ; un changement de conception de
  rag3weaver ou du produit attend son choix.
- **Rien dehors sans son mot** : PR amont, demande au support, publication.
  Un push en force ou une réécriture d'historique : seulement sur son mot.
- **« Non joué » n'est jamais « vert »**, un seuil ne se relâche pas sans
  preuve, une déduction se dit « déduite » jusqu'à ce qu'un test la tranche.
- Elle n'est pas spécialiste des moteurs de base : lui parler de ce que ça
  fait, pas des noms internes ; une recommandation par question.

## 4. Les limites de mes permissions, apprises ce soir

- **Supprimer une branche distante m'est refusé** par le garde-fou du poste,
  ainsi que le contrôle qui suivait. Je ne le contourne pas et je ne le
  confie pas à une autre session : c'est à Lucie (la commande est au rapport).
- **Supprimer un dépôt GitHub** : les jetons du poste n'ont pas ce droit
  (`gist`, `read:org`, `repo`). Lucie le fait sur le site.
- Monter sa clé USB (`udisksctl mount -b /dev/sda1`) : fait à sa demande.

## 5. Le poste

ROG Flow Z13 (Strix Halo) : 32 fils, 121 Gio, iGPU Radeon 8060S (Vulkan),
530 Gio libres. Budget quand plusieurs sessions compilent : `-j8` par
session, `-j6` pour une reconstruction de burn ; un test sensible au temps qui
rougit se rejoue seul avant d'être rapporté. Le target cargo de l'arbre
principal est partagé par ce qui s'y lance ; un worktree qui compile du Rust
prend son `CARGO_TARGET_DIR` et pointe `RAG3DB_ROOT` sur l'arbre principal.
Chaque arbre C++ a son répertoire de build (240 s pour `transaction_test`
depuis zéro à `-j8`).

Le shell de mes commandes est **zsh** (celui de Lucie est fish) : pas de
découpage de mots sur une variable (`set -- $paire` ne sépare rien — une
vérification a rendu dix-huit faux « introuvable » à cause de ça), un motif
sans correspondance est une erreur, `and` n'existe pas.

Le démon d'embarquement du port 7878 est relancé par les suites e2e quand son
binaire est périmé (`tests/common/mod.rs:207`) ; le 2 octobre il servait
bge-m3. Sans variable, un démon sert granite-278m.

## 6. Ce que cette nuit a appris du moteur

Le détail est dans `moteur-concurrence/` et dans
[`docs/2-octobre-2026-00h17/01-…`](../../../../../docs/2-octobre-2026-00h17/01-ecritures-paralleles-vela-et-le-chemin.md).
L'essentiel :

- **Le journal d'écriture.** Deux défauts corrigés : les enregistrements de
  plus de 4096 octets perdaient leur début (`955b1b136`) ; une fin de journal
  coupée empêchait d'ouvrir (`a66bb0b9d`). **Limite écrite** : faute de
  longueur d'enregistrement, une corruption qui tombe sur une longueur se lit
  comme une fin coupée ; rien n'est perdu, les octets retirés sont copiés
  dans `<base>.wal.ecarte-<ms>`.
- **Le lecteur d'un autre processus** rejoue le journal d'un écrivain vivant
  par trois lectures non atomiques. Prouvé par test : il peut compter 18
  relations au lieu de 9, sans erreur. Corrigé à l'ouverture par la marche 1
  (en cours) ; un lecteur qui **reste** ouvert n'est protégé par rien
  (marche 5, l'époque).
- **La reprise après un point de reprise interrompu** était fausse dès qu'une
  table avait connu des points de reprise (`DiskArray`, hérité de Kuzu, donc
  aussi chez Vela et Ladybug). Correctif `df4838542`, à livrer.
- **Après un point de reprise échoué, le processus qui continue perd des clés
  en silence puis plante** — sur master. Le moteur doit refuser tout jusqu'à
  réouverture ; rag3weaver doit apprendre quoi faire de cette erreur.
- **Le mode multi-écrivains corrompt en silence**, prouvé par le banc : clé
  en double, relation pendante (comptée, mais invisible à toute requête qui
  lit une propriété de l'extrémité), relations vers les nœuds d'un autre. Il
  n'est pas allumé en production.
- **Vela** n'est pas un MVCC neuf : c'est ce mode allumé, plus un remappage
  d'offsets et une reprise durcie ; son point de reprise attend toujours tout
  le monde. On n'en prendra que deux morceaux.
- **Les modèles établis** : un serveur avec des écritures parallèles dedans
  (Neo4j, DuckDB) ; un serveur dont les processus partagent de la mémoire
  (PostgreSQL) ; des processus indépendants mais un écrivain à la fois
  (SQLite, LMDB) ; un écrivain par index (Lucene). Décrits à Lucie de
  mémoire : à vérifier dans leur documentation avant de coder dessus.

## 7. Ce que cette nuit a appris de rag3weaver

- **Un catalogue = une connexion = une base**, en dur ; les index plein
  texte et sparse sont rangés **dans** la base et adressent les lignes par
  position interne ; détail dans
  [`2-octobre-2026-00h16/01-…`](../../2-octobre-2026-00h16/01-ce-que-rag3weaver-suppose-d-une-seule-base.md).
- **Dette de généricité** : cinq endroits du cœur du crate câblés pour le
  code (journal des chantiers, §3).
- **Trois défauts possibles, non vérifiés** : un client peut s'attacher au
  démon d'une autre base ; même contenu dans deux cellules = même uuid ; deux
  bases du même poste partagent cache et dossier de checkpoints par défaut.
- **La recherche** : un seul chemin ; `fuse_results`, la grappe
  d'exploration et `search_with_strategy` sont retirées (`01791e347`), leurs
  tests portés ; référence du banc sur granite-278m dans
  [`2-octobre-2026-01h01/01-…`](../../2-octobre-2026-01h01/01-reference-du-banc-de-l-etage.md).
- **`sparse-vector` 4.3.0** : sources identiques à 4.0.1. Le signal sparse
  n'a jamais été mesuré au banc ; le gabarit lui laisse le 0,2 du moteur.

## 8. Ce que cette nuit a appris de GitHub et des poids

- **Un fork partage son magasin d'objets avec tout son réseau** : un commit
  poussé sur un fork reste atteignable par son hash depuis le dépôt d'origine,
  fork supprimé ou non. Réécrire, supprimer, recréer ne purge rien ; seul le
  support GitHub le peut. J'ai d'abord recommandé à tort la
  suppression-recréation comme une purge certaine ; vérifié ensuite par l'API.
  Leçon : une affirmation sur un service extérieur se vérifie avant d'être
  recommandée.
- **Les poids convertis vivent dans `~/.cache/rag3weaver/`**, hors de toute
  sauvegarde du dépôt. granite et l'OCR n'avaient jamais été publiés ; granite
  est régénéré, prouvé contre l'ONNX d'IBM et publié en dépôts privés ; l'OCR
  (ppocrv6-tiny) reste à régénérer ou à rapporter de l'ancien poste.
- Un `.bpk` régénéré a la même taille mais pas la même empreinte : c'est
  l'écart absolu contre l'ONNX qui prouve, pas le sha256.

## 9. Où lire quoi

| Pour savoir… | Lire |
|---|---|
| l'état de tous les chantiers | `docs/journal-des-chantiers.md` |
| comment reprendre | `01-rapport-de-session.md`, à côté de ce fichier |
| le plan des écritures parallèles | `docs/2-octobre-2026-00h17/01-…` |
| la cause du journal illisible et la fin déchirée | `docs/1-octobre-2026-23h37/01-…` |
| le pas C | `docs/2-octobre-2026-00h43/01-ponderations-dans-les-graphes.md` |
| les PR amont et le retrait de l'adresse | `extension/rag3weaver/docs/optimiseur/2-octobre-2026-00h15/` |
| l'architecture de rag3weaver | `extension/rag3weaver/docs/1-octobre-2026-22h47/02-knowledge-dump.md` |
| les objectifs et leur ordre avant cette nuit | `…/1-octobre-2026-22h47/01-reconciliation-des-objectifs.md` |
