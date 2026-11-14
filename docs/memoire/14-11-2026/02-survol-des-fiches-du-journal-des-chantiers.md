# Survol des fiches proposées depuis le journal des chantiers

*14 novembre 2026, session mémoire (96), pour Lucie. L'archiviste a rangé `docs/journal-des-chantiers.md`, découpé en un
chantier par section, sur une **copie** de ta mémoire (l'état de 14 h 50, 116 fiches) : rien n'est encore écrit chez toi.
Les fiches sont toutes `proposed`. La vraie passe viendra ensuite ; cette page sera alors refaite avec les uuid des fiches.*

**Ce qu'on te demande** : pour chaque fiche thématique, « garde » (une fiche pour toutes ces sections) ou « scinde »
(une fiche par chantier) ; pour les autres, un coup d'œil suffit — « retire » si une fiche n'a pas lieu d'être.

**Les nombres de l'essai** : 156 sections décidées, 0 erreur ; 152 rangées, 4 jugées « none » ; **56 fiches créées** (19 thématiques, 37 à un chantier) ; 2 sections rejoignent une fiche existante ; 0 nom daté refusé, 0 description de moins de six mots.

**Une correction déjà décidée** : les quatre sections d'en-tête du journal (son mode d'emploi : « À quoi il sert », « Les deux
règles », « La partie qui se recalcule », « À lire avec ce journal ») ne seront pas dans la vraie passe ; elles sont marquées
*(en-tête, écarté)* ci-dessous.

## 1. Les 19 fiches thématiques — garde ou scinde

### Bonnes pratiques du harnais de développement — 19 sections

*Recommandations et pièges à éviter lors de l'utilisation du harnais et des dépôts de travail.*

- 6. Le chemin du moteur dérivé du chemin du code
- 8. Le diff qui montre vos propres fichiers comme supprimés
- 7. Le binaire de test plus vieux que le moteur qu'il charge
- 10. Le script qui écrit le code avale la continuation de ligne
- Et la forme commune aux neuf
- 5. Le moteur remplacé pendant la passe
- relire le fichier, pas le script
- bibliothèque
- 9. Le « bâti à part » qui n'est pas à part
- 7 bis. L'extension et la bibliothèque d'âges différents, et rien ne le voit
- chemin
- run_e2e.sh
- 1. Le sous-module reste en arrière après un rebase
- Précision du même soir, et elle corrige la première version de cette note
- e2e_mesure_sync_source
- Et le remède échoue
- Ce qui reste vrai, et c'est le piège
- e2e_code_sync (un seul site, deux tests)
- 2. git stash est partagé entre les worktrees

### Sous-module codeparsers — 16 sections

*Évolution de l'extraction des relations, résolution des homonymes et graphe de code dans codeparsers.*

- Dégelé le 4 octobre à midi
- sur disque
- Justes
- Livré le 4 octobre vers 1 h
- Le fait
- La justesse
- Ce que la règle retire de l'index d'aujourd'hui
- Livré côté codeparsers (4 octobre, nuit)
- Mis de côté : la résolution parallèle par fichier
- La décision
- Codeparsers
- À faire à la mise à jour
- Fausses
- Ce que ça ouvre
- L'analyse était quadratique, et sa sortie pas déterministe
- Prochain lot côté justesse : les paires .h / .cpp

### Suivi des chantiers ouverts — 12 sections

*Ce fichier recense les travaux non fusionnés ou en cours entre sessions pour éviter les oublis.*

- Toujours en attente de Lucie
- La partie qui se recalcule *(en-tête, écarté)*
- c_api_test : deux tests faux depuis le renommage du fork (le moteur était juste)
- codeparsers-genre-d-usage (sous-module codeparsers, dépôt L-Defraiteur/codeparsers)
- Index vectoriel : un DELETE annulé par ROLLBACK ne le laisse plus muet ; un survivant vers lequel ne menaient que des nœuds supprimés reste
- pas-c-ponderations (130984f61)
- Arrêt du 2 octobre 2026, 01 h 25
- Marche T0 : après une erreur dans BEGIN … COMMIT, tout est refusé jusqu'au ROLLBACK (TransactionContext::MANUAL_TRANSACTION_FAILED), erreur
- Les deux règles *(en-tête, écarté)*
- Marche A2 : les offsets provisoires des nœuds d'une transaction sont remappés au commit ; les insertions sont journalisées au commit ; au co
- Un champ NULL d'une liste de structures littérales prend le type des autres éléments
- À quoi il sert *(en-tête, écarté)*

### Audit des erreurs tues — 10 sections

*Inventaire et qualification de l'impact des erreurs étouffées ou non propagées dans le système.*

- 2713, :6488
- 6762
- 4495, :4547, :4576
- 5621, :5680
- 3189, :3197, :3321, :3329, :5466
- 4642
- 3142, :6723
- 1679, :1703
- 2663
- 1347

### Mode multi-écrivains du cœur C++ — 9 sections

*Travaux d'ordonnancement, de verrous et de concurrence pour le mode multi-écrivains dans le cœur C++.*

- La course de A5 fait planter le processus
- Les verrous
- Le mode multi-écrivains corrompt en silence — prouvé le 2 octobre 2026
- Marche A5, livrée le 3 octobre 2026
- Un faux vert à éviter dans la liste de livraison
- Écritures parallèles
- Marche A5 bis, livrée le 3 octobre 2026
- 10 octobre 2026, 20 h 30 — cœur C++, marche A3′ sur master
- Les défauts du mode multi-écrivains et des transactions, prouvés par le banc

### Chantier du cœur C++ — 7 sections

*Suivi des livraisons, des versions de stockage et des conditions de la stèle pour le cœur C++ du moteur.*

- Suite de la session cœur C++
- c_api_test entre dans la liste de livraison de toute marche du moteur
- Livraison du cœur C++, 10 octobre 2026 — la condition 2 de la stèle est fermée
- Pour la session cœur C++
- Livré le 4 octobre 2026
- Livraison du cœur C++, 3 octobre 2026
- Le rejeu d'une mise à jour de nœud n'informait aucun index

### Expérience MTGA — 7 sections

*Suivi des développements, données locales et composants de l'expérimentation MTGA.*

- Les docs des expériences, noms de cartes compris
- Deck builder MTG (produit)
- Le moteur, le backend déclaratif, le harnais, le chat, les scripts de préparation et d'ingestion
- Persistance des abilities imbriquées
- Base MTG
- Le manifeste backend.json, les schémas, les graphes, les règles Rhai, le gabarit de rendu
- Les branchements vers les sources locales : mtga-reader, Player.log, les SQLite du client Arena

### Suites de tests sur carte locale — 5 sections

*Règles et conditions d'exécution des suites de tests sollicitant la carte graphique locale.*

- Et la conséquence sur ce qu'on rend
- Le second défaut, indépendant
- « complète hors carte locale »
- La règle
- Sur cette machine (ROG Flow Z13)

### Refus des déclarations de configuration inopérantes — 4 sections

*Cette fiche traite du rejet par validation des options de configuration acceptées mais non appliquées par le moteur.*

- 974
- 1985
- 710
- 1995

### Forks de burn, cubecl et cubek — 3 sections

*Cette fiche suit l'état et l'utilisation des versions dérivées de burn, cubecl et cubek.*

- PR amont burn / cubek
- Forks burn, cubecl, cubek
- Adresse professionnelle sur les forks

### Rangement en mémoire longue — 3 sections

*Méthodes et critères pour classer un texte parmi les sujets existants de la mémoire longue.*

- ranger
- décider qu'il faut créer
- Et une forme à retenir pour le nœud

### Régime carte partagée et embarquements — 3 sections

*Régulation des rafales d'ingestion et déport des services d'embarquement pour ménager la carte graphique.*

- (services sur luciepc)
- Pourquoi aucun régime doux ne sauve ces suites-là
- regime-carte-partagee-2

### Synchronisation par périmètre — 3 sections

*Travaux et mécanismes de synchronisation par périmètre et gestion des instantanés.*

- Synchronisation par périmètre
- transition
- 2 et 3 octobre 2026, synchronisation par périmètre

### Déclaration unifiée des modèles — 2 sections

*Déclaration commune et résolution des capacités de modèles sous forme de services distants ou locaux.*

- Seul l'embarquement sait dire « je calcule ailleurs »
- modeles-declares (fusionnée)

### Indexation en tâche de fond — 2 sections

*Conception et suivi de l'indexation d'un dépôt en tâche de fond et de la concurrence avec la recherche.*

- Indexer un dépôt
- index-en-fond (fusionnée)

### Marche A5 bis — 2 sections

*Cette fiche regroupe les travaux et les mesures de coût associés à la marche A5 bis.*

- A5 bis
- Non vérifié

### Publication et hébergement des poids de modèles — 2 sections

*Cette fiche suit l'hébergement, la publication sur Hugging Face et la gestion des poids des modèles utilisés par le système.*

- Les poids ne dépendent plus d'aucun disque
- Les poids granite ne sont ni sur ce poste ni publiés

### Repli des KB en entités dérivées — 2 sections

*Travaux de repli des bases de connaissances en entités dérivées et étalonnage des poids de fusion.*

- 3 octobre 2026
- Repli des KB en entités dérivées

### Tests de reprise après incident — 2 sections

*Règles et pièges à éviter lors de l'écriture des tests de reprise après crash.*

- Deux faux verts à connaître pour tout test de reprise
- e2e_arret_brutal (le mien)

## 2. Les 37 fiches à un chantier

| fiche | ce qu'elle couvre | la section du journal |
|---|---|---|
| Anomalie de count(*) sur QUERY_VECTOR_INDEX | Cette fiche traite de l'anomalie renvoyant systématiquement k lors d'un count(*) sur un index vectoriel. | QUERY_VECTOR_INDEX … RETURN count() rend toujours k |
| Bac à sable de l'agent de code | Confinement et sécurisation de l'exécution des commandes lancées par l'agent de code. | Bac à sable des commandes de l'agent de code |
| Balayage de table sur UNWIND MATCH | Diagnostic et contournement du balayage complet de table généré par le planificateur lors d'un UNWIND MATCH par clé primaire. | UNWIND $items AS item MATCH (n {_uuid: item.champ}) balaie la table entière |
| Bibliothèque lucivy | Suivi des versions et de l'usage de la crate lucivy. | lucivy |
| Chantier amont Vela | Suivi des développements et des intégrations de la branche de stockage amont Vela. | Amont Vela |
| Chantier de la mémoire longue | Conception générale, principes directeurs et architecture du produit de mémoire longue. | Mémoire longue |
| Chantier everything declarative | Prototype et implémentation du moteur tout déclaratif et de ses composants scriptés. | 10 octobre 2026, 23 h — chantier I « everything declarative », branche proto-declaratif |
| Chantier T0 | Modifications du comportement transactionnel et des contrats clients apportées par la branche T0. | Ce que T0 change pour un client |
| Chemin de masse des lots de naissances | Optimisation et diagnostic de performance lors de l'ingestion en masse par COPY des lots de naissances. | Chemin de masse des lots de naissances |
| Choix automatique du mode de recherche | Sélection dynamique du mode de recherche selon l'état d'avancement de l'indexation. | La recherche choisit seule son mode |
| Cohérence des champs nuls entre CREATE et SET | Règles et arbitrages sur le rejet des champs nuls dans les instructions SET et CREATE du cœur. | Tranché le 3 octobre par l'orchestration, sans le faire trancher à Lucie |
| Corruption du journal WAL à l'écriture | Cette fiche traite de la corruption du journal WAL provoquée par le redimensionnement défectueux du tampon d'écriture. | WAL illisible : la vraie cause est un bug d'écriture |
| Exclusion des fichiers générés | Critères et règles de politique permettant d'écarter les fichiers volumineux générés de l'indexation. | Les géants générés |
| Folds des scopes | Suivi de l'intégration du champ folds dans les scopes et de la ré-ingestion du code. | Champ folds des scopes |
| Identité de ligne indépendante de la cellule | Cette fiche documente l'absence de prise en compte de la cellule dans le calcul de l'identité des lignes. | L'identité d'une ligne ne dépend pas de la cellule |
| Indexer nos propres dépôts | Travaux et suivi de la couverture d'indexation de nos propres dépôts avec codeparsers. | 10 octobre 2026, soir — codeparsers, chantier « indexer nos propres dépôts » (B4, B5, B2/B1, B3), master de codeparsers en avance rapide |
| Maintenance transactionnelle de l'index vectoriel | Cette fiche documente l'absence d'isolation transactionnelle des points d'entrée de l'index vectoriel et sa future correction au commit. | Les points d'entrée de l'index vectoriel vivent hors transaction |
| Marche A5 ter | Travaux d'intégration d'une garde sans surcoût contre les lectures de mémoire libérée. | Marche A5 ter, ouverte, non commencée : une garde qui ne coûte rien au lecteur |
| Modèle de décision de l'optimiseur | Choix et intégration d'un modèle de décision branchable sans réentraînement pour l'optimiseur. | Modèle de décision |
| Montage des réactions de la mémoire longue | Conception et intégration du montage automatique et du cycle de vie des réactions pour la mémoire longue. | 11 octobre 2026, 4 h — mémoire longue, le montage des réactions : FAIT, sur master |
| Nœuds injoignables à la construction de l'index vectoriel | Suivi et reproduction de l'anomalie d'élagage rendant des lignes injoignables lors de la construction d'un index vectoriel. | L'index vectoriel peut laisser une ligne injoignable dès sa construction |
| Offsets provisoires de l'extension plein texte | Cette fiche documente l'absence de remappage des offsets provisoires par la marche A2 dans l'extension plein texte. | L'extension de plein texte range des offsets provisoires |
| Persistance des index FTS et sparse | Cette fiche traite de la persistance et de la durabilité des index FTS et sparse en l'absence de magasin de blobs. | 1006 |
| Plantage avec un buffer pool réduit | Cette fiche documente l'anomalie de plantage par violation d'accès survenant lorsqu'un petit buffer pool est configuré. | SIGSEGV avec un buffer pool de 1 Gio |
| Plantage de l'élagage des voisins à la suppression | Cette fiche documente l'anomalie et le contournement du plantage dans shrinkForNode lors de la suppression vectorielle. | L'élagage immédiat des voisins plante dans le chemin de suppression |
| Politique envers les projets amonts | Règles d'emprunt, de licence et de reprise des correctifs issus des dépôts amonts. | Les amonts : on les lit, on ne leur transmet rien, on ne copie pas |
| Refus du COPY après ALTER DEFAULT NULL | Cette fiche documente l'anomalie du moteur rejetant l'instruction COPY lorsqu'une colonne ajoutée avec une valeur par défaut nulle est omise. | Une colonne ajoutée par ALTER … DEFAULT NULL fait refuser tout COPY qui l'omet |
| Report de la poussée des blobs d'index | Étude et décisions concernant l'ajournement de la poussée des blobs d'index plein texte lors des ingestions. | Gelé (base en mémoire seulement) : le report de la poussée des blobs d'index |
| Reprise après point de reprise interrompu | Cette fiche documente l'anomalie de réouverture de base liée à l'index de clé primaire après un point de reprise interrompu. | Reprise après un point de reprise interrompu : corrigée |
| Récupération des lignes supprimées dans rag3db | Travaux de récupération et de purge des lignes supprimées et des blobs d'index dans rag3db. | Récupération des lignes supprimées dans rag3db |
| Réouverture de base sur journal WAL vide | Traitement et diagnostic de l'anomalie de lecture d'un journal WAL de taille nulle lors de la réouverture d'une base. | e2e_idempotent_registration rougit environ une fois sur dix à la réouverture |
| Statut d'index dans le chat | Affichage de l'état d'avancement de l'indexation directement dans l'interface de discussion sans solliciter l'agent. | Ligne de statut d'index dans l'application chat |
| Sujets en entités dérivées | Conception et évolution de l'entité Subject sous forme d'entité dérivée pour la mémoire longue. | Une correction à porter quand le lot des sujets viendra |
| Transition d'état des modèles d'embarquement | Cette fiche suit la déclaration des transitions d'état lors de la requalification des modèles d'embarquement. | Un modèle d'embarquement requalifié en ancien = une transition d'état déclarée |
| Validation de la passe Rust | Consignes de construction des binaires et pièges à éviter lors des tests de la passe Rust. | Un faux vert à éviter dans la passe Rust |
| Visions du produit code | Orientations et idées retenues pour le produit code et l'exploration des relations dans rag3weaver. | Visions |
| Édition pendant l'indexation | Prise en compte sans ralentissement des modifications de fichiers survenues au cours d'une indexation. | Une édition pendant l'indexation |

## 3. Les sections qui rejoignent une fiche existante

| fiche existante | la section du journal |
|---|---|
| Durabilité sur faute d'entrée-sortie ou coupure | Reprise après arrêt brutal, livrée le 3 octobre 2026 au soir |
| bilan : tâche réelle, bornée, dans ce worktree du dépôt rag3db (extension/rag3weaver). ticket : d… | Le backend de code |

## 4. D'où viennent ces chiffres

- Le découpage : un script de mesure, hors du crate, réécrit le journal en un titre par chantier (une puce « - **…** », un
  paragraphe en gras ou une ligne de tableau), la partie d'origine rappelée dans le texte. Un sous-titre en gras isolé
  (« Non vérifié », « chemin ») devient aussi une section : c'est le bruit du découpage, et l'archiviste le range ou le juge « none ».
- L'archiviste de master (86b7ebf3a) : le nom exact puis la voisine parmi **toutes** les fiches, la phrase d'au moins six mots,
  le refus des noms datés ; `gemini-3.8-flash` par Vertex.
- La copie et ses données sont à luciepc, `~/.cache/rag3weaver-build/chantiers-96/` (`analyse.json`, le document découpé).
