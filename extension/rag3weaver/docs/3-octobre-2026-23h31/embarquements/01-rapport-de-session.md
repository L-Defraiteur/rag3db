# Embarquements — rapport de session

Session « embarquements » : worktree `/home/lucied/git_workspaces/rag3db-embarquements`
(crate `extension/rag3weaver`), target à elle dans ce worktree. Elle tient le
service de modèles sur l'autre poste, le régulateur d'écran, l'estimation et
l'indexation en fond, l'état d'avancement, la déclaration commune des modèles,
et les mesures du dépôt entier. Mis à jour le 4 octobre 2026, vers 11 h 45.

## Fait aujourd'hui, sur master

Dans l'ordre de fusion.

| Commit | Lot | Pourquoi |
|---|---|---|
| `f1b3845b9` | **Régulateur de rafale** (`src/burst.rs`) et **`RAG3WEAVER_EMBED_SERVICE`** | L'écran de Lucie figeait pendant les passes : sur un poste dont la seule carte porte l'affichage, on vise une durée de rafale et une pause. Et une variable pour s'attacher à un service d'embarquement déjà en place, tests compris. |
| `f0a97ff16` | **`estimate`** (`src/estimate.rs`, `EstimateNode`) et **l'avancement** (`Catalog::index_progress`) | Dire ce qu'une indexation coûtera avant de la lancer ; rendre lisible la dette de vecteurs, jusque-là privée. |
| `79b200ceb` | **`index`** (`IndexNode`, `spawn_index`) | Indexer en fond : le reçu est un journal au contrat de `run_bg`, que `wait` sait attendre. |
| `ad3966f83`, `92ecaa656` | **`index_state`**, puis **`index_state_for(entity)`** | La recherche choisit son mode sur l'état de l'index, en une lecture de méta. L'état global était faux dans le chat réel (le journal de conversation vit dans la même base) : il se lit par entité. |
| `aaa652948` | **Modèles déclarés, lot 1** (`src/model_source.rs`) | Demande de Lucie : un modèle se déclare en service ou en local, pour toute capacité. La déclaration, sa résolution, l'embarquement dense dessus. |
| `91d761268` | **Lot 2 : la décision** (`src/decider.rs`) | Premier client neuf : un texte déjà composé, des options, une probabilité par option ; fournisseur `llama_server`. |
| `bfbd57d1a` | **Lot 3 : le creux et le dual** branchés dans un backend | Un backend comptait le signal creux pour exiger un service, puis ne branchait jamais d'embarqueur creux. |
| `0202a2ee6` | **Lot 4 : relecteur et OCR** déclarés en local | Les nœuds existaient, le backend ne leur donnait aucun service. |
| `4cb2d04cc` | **Lot 5 : le démon porte le relecteur et l'OCR** | Un seul démon à relancer sur le poste qui sert. |
| `0330578b6`, `3645e0799` | **Lot 6 : le modèle de langage** dans la déclaration commune (`models.llm`), posé dans le chat par la session du chat | Dernière capacité hors de la déclaration ; éprouvé de bout en bout avec Gemini par Vertex. |
| `45c88e3a5` | **L'invariant des vecteurs** (`tests/e2e_invariant_des_vecteurs.rs`), la dette périmée, `write_vectors` | Lot de l'orchestration : le moteur perd des lignes de l'index HNSW quand un `SET` remplace un vecteur. La suite a surtout trouvé un défaut à nous — voir plus bas. |
| `3da4bb05a` | **Fichiers générés écartés avec leur raison** (`src/generated.rs`, `workspace.generated`) | Lot de l'orchestration : 95 fichiers et 5,4 Mo sur ce dépôt n'apprennent rien à un agent. Deux signaux sur trois ; détail dans la page 03. |
| `867ce5560` | **Le signal creux dans `IndexState`** (`sparse` : niveau et pourcentage, à part du dense) | Demande de la session du chat : la dette creuse était comptée mais invisible à la ligne d'état et au mode auto. |
| `1d3319f65`, `a7b241faa` | **Profil d'ingestion complet** (`src/ingest_profile.rs`, `[sync-profile]`, `[ingest-total]`) | 140 s de l'indexation n'étaient dans aucune ligne de profil. |

Les pages : `docs/3-octobre-2026-14h26/01` (une seule carte partagée avec
l'affichage), `02` (les services de l'autre poste : lancer, arrêter, tunnels,
relance), `03` (« indexer ce dépôt » : proposition, puis toutes les mesures) ;
`docs/3-octobre-2026-21h22/01` (un modèle, en service ou en local).

## Décidé, et pourquoi

- **Le service d'embarquement vit sur l'autre poste** (Lucie). Ce poste n'a
  qu'une carte, qui porte l'écran ; `luciepc` a une carte libre de 32 Gio. On
  s'y attache par tunnel ssh ; rien n'écoute hors de la boucle locale ; rien
  n'y est commité ni poussé (son identité git globale est professionnelle).
- **Une variable d'attache ne lance ni n'arrête rien.** `RAG3WEAVER_SERVICE_EMBED`
  (alias `RAG3WEAVER_EMBED_SERVICE`) désigne un service qu'on n'a pas lancé :
  le client s'y attache et c'est tout. Sans service pour le modèle demandé,
  c'est un refus qui dit ce que chaque adresse sert — jamais un repli
  silencieux sur la carte de l'écran.
- **Le repli se déclare** (`fallback: local`), défaut `refuse`.
- **La frontière de l'abstraction passe au trait** : chaque capacité garde ses
  entrées et sorties ; ce qui est commun est où vit le modèle, à quelle
  adresse, et quoi faire s'il ne répond pas.
- **La décision ne sait rien de la formulation** (Lucie : « ça reste une
  expérimentation »). Le prompt est composé par le graphe ; la calibration se
  donne par appel (`decide_at`), parce qu'elle change d'un critère à l'autre.
- **L'état d'index se lit par entité**, et un « jamais » noté est recompté :
  sinon un outil refuserait pour toujours un index rempli par un autre chemin.
- **On instrumente avant de corriger.** Mon diagnostic « 140 s dans les points
  de reprise » était faux : c'étaient les blobs de l'index plein texte. Les
  chronomètres l'ont dit, pas le raisonnement.
- **Les défauts provisoires sont écrits comme tels** : rafale 50 ms / pause
  150 ms (`BurstSettings::DEFAULT`), seuil de confirmation d'`index` à cinq
  minutes (`estimate::CONFIRM_ABOVE`).

## L'invariant des vecteurs : ce que la suite a dit (4 octobre, 0 h 30)

- **Un défaut à nous, corrigé.** Après une édition posée sans embarquement,
  300 vecteurs sur 600 restaient ceux de l'ancien texte : l'édition ne vide
  que `_embed_hash`, pas le marqueur du modèle courant, et la dette ne
  comptait que les marqueurs vides. La dette compte maintenant un marqueur
  différent de `_text_hash`.
- **Le défaut du moteur nous touche peu.** Le produit pose ses vecteurs par
  lots de 32 ; le moteur perd surtout par gros lots (522 à 549 joignables sur
  1000 par lots de 512, 988 une fois par lots de 64, 1000 par lots de 32 et
  ligne à ligne ; une fois 169 sur 600 en dimension 32). Jouée une fois sans
  contournement, la réédition de mille lignes gardait 1000 joignables : la
  suite garde le chemin, elle ne démontre pas que le contournement sert.
- **Le contournement** est dans une seule fonction, `write_vectors`
  (`dataflow/record_nodes.rs`) : une ligne qui porte déjà un vecteur repasse
  par NULL, seule dans son instruction. À enlever là quand le moteur sera
  corrigé. « Sûr ligne à ligne » repose sur les mesures du cœur C++.
- Les cas sont donnés à la session du banc pour ses témoins du moteur.

## En cours

- **Fichiers générés, le reste** : ce que la règle retire en relations et
  en temps (à la prochaine passe du dépôt entier), `.gitattributes
  linguist-generated`, et le troisième signal (grosse taille, peu de scopes).

- **Le démon bge-m3 de luciepc sert aussi le relecteur et l'OCR** depuis
  23 h 45 (worktree de service à `4cb2d04cc`, relancé dans un creux des
  autres sessions) : `bge-reranker-v2-m3` et `ppocrv6-tiny`, vérifiés par le
  tunnel (relecture de trois passages en 189 ms). Les deux démons granite
  tournent encore le binaire de l'après-midi : sans conséquence, ils ne
  servent que l'embarquement.
- **Le budget pour 90 s** (cible de Lucie, 3 octobre au soir, pour le premier
  index de ce dépôt). La session de l'arbre principal a rendu un budget par
  poste pour un chargement de bout en bout ; deux mesures me reviennent :
  `analyze_with` sur le dépôt entier en un appel (durée, pic de mémoire), et
  le plein texte bâti d'un coup (durée, taille des blobs).
- **La remesure du dépôt entier est faite** (4 octobre, 11 h 30, sur master
  avec le résolveur unique) : 289 s par paquets de 64, 120 s à 512, 111 s à
  2 048, **89 s d'un tenant** ; pics de 10,9 à 16,6 Go. Tableau et postes dans
  la page 03. Restent inexpliqués : le pic de mémoire le plus haut à 64, et
  474 relations d'écart entre 64 et 512.

## Ce qui attend quelqu'un

| Quoi | Qui |
|---|---|
| Le défaut (rafale, pause) du régulateur : deux ou trois couples à mesurer avec elle devant l'écran | Lucie |
| Le seuil de confirmation d'`index` (cinq minutes proposées) | Lucie |
| Les trois noms tranchés par défaut : clé `models`, variables `RAG3WEAVER_SERVICE_<CAPACITÉ>`, repli `refuse` | Lucie, si l'un ne convient pas |
| L'ordre des vecteurs, la politique « cloud » des adresses git (page 03, § 6) | Lucie |
| Le report des blobs d'index, avec la marque « plein texte en retard » | session de l'arbre principal |
| Une branche `sparse` dans les graphes de recherche des gabarits de backend | session recherche |
| Lot 6 : `models.llm` (la section `llm` du chat en alias) | avec la session recherche, après sa faille de `run_command` |

## Comment reprendre

- **Lire d'abord** : `docs/journal-des-chantiers.md`, puis la page `03` (les
  mesures) et le `02-knowledge-dump.md` à côté de ce fichier.
- **Le worktree** : `rag3db-embarquements`. Y vérifier `git config user.email`
  (gmail). Les branches de travail sont locales et jetables ; tout ce qui
  compte est sur master. Les branches poussées `regime-carte-partagee-2/-3`,
  `estimation-et-avancement(-2)`, `index-en-fond`, `profil-ingestion` sont à
  ranger par Lucie.
- **Le cycle d'un lot** : branche depuis `origin/master`, tests d'abord, lib
  verte, les suites touchées, rebase, lib de nouveau, puis
  `git push origin HEAD:master` en avance rapide. Jamais de force. Un lot fini
  et vert se fusionne sans demander (consigne de Lucie, 3 octobre) ; une ligne
  à l'orchestration après coup.
- **Les commandes** : voir le knowledge dump, § « Lancer les tests ».
- **Après un redémarrage de ce poste** : refaire les tunnels (page `02`, § 4,
  § 7 bis). **De luciepc** : relancer les démons (§ 3, § 7 bis, § 7 ter).
- **Pièges** : ne jamais pointer `RAG3WEAVER_EMBEDDINGS_ADDR` sur un tunnel
  (les suites enverraient `/quitter` au service) ; donner à chaque arbre son
  port local (`RAG3WEAVER_EMBEDDINGS_ADDR=127.0.0.1:7890`) ; le shell de
  luciepc est fish (passer par `ssh … bash -s < script`) ; une passe du dépôt
  entier prend six à dix minutes et se lance détachée (`setsid nohup`), on la
  surveille en lisant son fichier de sortie.
