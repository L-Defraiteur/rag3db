# Survol des fiches proposées depuis le journal des chantiers

*14 novembre 2026, session mémoire (96), pour Lucie. La passe a été faite **dans ta mémoire** (18 h 19 – 18 h 43). L'archiviste
a rangé `docs/journal-des-chantiers.md`, découpé en un chantier par section, sans son en-tête (son mode d'emploi). Chaque fiche
créée est `proposed`, avec son uuid : son fichier est `~/memoire-longue/fiches/<uuid>.md`. Tant que tu ne l'as pas acceptée, rien n'est `current`.*

**Ce qu'on te demande.** Pour chaque fiche thématique, « garde » (une fiche pour toutes ces sections) ou « scinde » (une fiche
par chantier). Pour les autres, un coup d'œil suffit : « retire » si une fiche n'a pas lieu d'être (transition `reject`).

**Les nombres.** 152 sections décidées, 0 erreur, 18,5 min. 143 sont rangées et 9 jugées « none ». **57 fiches créées** : 20 thématiques, 37 à un chantier. 8 sections rejoignent 5 fiches existantes. Aucun nom daté refusé, aucune description de moins de six mots. À la fermeture, la base et les fichiers comptent tous deux 173 fiches.

**Pendant la fenêtre** (18 h 35), le juge d'un run d'agent (l'essai tty `agent-1a1db20d720-9091`) a écrit par l'hôte, qui a
rouvert la base. Ces écritures ont été gardées (`~/.local/share/rag3weaver/instantanes/ecrit-pendant-la-fenetre-14-nov/`),
puis **rejouées** après le retour à l'identique, sauf `via`, que l'hôte pose :

- la Source `sources/2026-11-14-agent-1a1db20d720-9091.md` ;
- la fiche « bilan : essai-tty » et ses trois apports ;
- les deux fiches outil, « outil : report » et « outil : step_done ».

Ta mémoire compte maintenant 174 fiches. Le ticket
`docs/memoire/tickets/2026-11-14-un-backend-ferme-par-l-hote-se-rouvre-au-premier-appel.md` demande que la fermeture refuse ces
réouvertures.

**Pour revenir en arrière** : faire fermer le backend `memoire-longue` par G, et remettre
`~/.local/share/rag3weaver/instantanes/avant-chantiers-14-nov-2/` à sa place.

## 1. Les 20 fiches thématiques — garde ou scinde

### Résolveur de relations inter-fichiers — 16 sections

*Ce chantier assure l'indépendance du graphe de relations par rapport à la taille des paquets lors de la résolution entre fichiers.* — `b9f1bd0a-7b32-27bd-8738-a87ba1d17c80`

- Prochain lot côté justesse : les paires .h / .cpp
- Mis de côté : la résolution parallèle par fichier
- Le fait
- La décision
- Dégelé le 4 octobre à midi
- Fausses
- Pour la session cœur C++
- Ce que ça ouvre
- Livré le 4 octobre vers 1 h
- À faire à la mise à jour
- L'analyse était quadratique, et sa sortie pas déterministe
- La justesse
- Ce que la règle retire de l'index d'aujourd'hui
- Livré côté codeparsers (4 octobre, nuit)
- Justes
- sur disque

### Chantier multi-écrivains du cœur C++ — 13 sections

*Ce chantier couvre les développements et correctifs du mode multi-écrivains pour le cœur C++.* — `778e9088-62b6-b5ef-5e9f-369c46ace0d9`

- Écritures parallèles
- Suite de la session cœur C++
- Marche A5 ter, ouverte, non commencée : une garde qui ne coûte rien au lecteur
- A5 bis
- Un faux vert à éviter dans la liste de livraison
- Livraison du cœur C++, 10 octobre 2026 — la condition 2 de la stèle est fermée
- Les défauts du mode multi-écrivains et des transactions, prouvés par le banc
- La course de A5 fait planter le processus
- Marche A5 bis, livrée le 3 octobre 2026
- Le mode multi-écrivains corrompt en silence — prouvé le 2 octobre 2026
- Les verrous
- Marche A5, livrée le 3 octobre 2026
- 10 octobre 2026, 20 h 30 — cœur C++, marche A3′ sur master

### Audit des erreurs tues du cœur C++ — 11 sections

*Ce chantier recense et évalue les sites du cœur C++ où une opération échoue sans propager l'erreur.* — `2c43c6b6-47bf-a319-8823-cde4ea85eac5`

- 4642
- 2713, :6488
- 2663
- 1347
- chemin
- 3142, :6723
- 1679, :1703
- 3189, :3197, :3321, :3329, :5466
- 6762
- 5621, :5680
- 4495, :4547, :4576

### Validation et construction des binaires de rag3weaver — 9 sections

*Cette fiche consigne les règles de construction et pièges d'exécution des tests pour rag3weaver.* — `c7c9aad4-08b6-2152-f20d-0f3291887955`

- La règle
- bibliothèque
- 5. Le moteur remplacé pendant la passe
- e2e_mesure_sync_source
- Le second défaut, indépendant
- Sur cette machine (ROG Flow Z13)
- Un faux vert à éviter dans la passe Rust
- 7. Le binaire de test plus vieux que le moteur qu'il charge
- Et la conséquence sur ce qu'on rend

### Expérience MTGA — 7 sections

*Ce chantier regroupe les développements, composants et données liés à l'expérience Magic: The Gathering Arena.* — `4e582d70-abb1-4d95-0bbd-5ea05d1463ba`

- Les docs des expériences, noms de cartes compris
- Deck builder MTG (produit)
- Le moteur, le backend déclaratif, le harnais, le chat, les scripts de préparation et d'ingestion
- Persistance des abilities imbriquées
- Base MTG
- Le manifeste backend.json, les schémas, les graphes, les règles Rhai, le gabarit de rendu
- Les branchements vers les sources locales : mtga-reader, Player.log, les SQLite du client Arena

### Reprise après arrêt brutal — 6 sections

*Ce chantier assure la reprise et la cohérence du cœur C++ après un arrêt brutal.* — `c2b263c2-d033-58d2-aaa8-ab29ba1265d5`

- e2e_arret_brutal (le mien)
- Reprise après arrêt brutal, livrée le 3 octobre 2026 au soir
- Le rejeu d'une mise à jour de nœud n'informait aucun index
- WAL illisible : la vraie cause est un bug d'écriture
- Reprise après un point de reprise interrompu : corrigée
- Deux faux verts à connaître pour tout test de reprise

### Chantier de l'index vectoriel — 5 sections

*Ce chantier couvre la robustesse, la connectivité du graphe et le comportement sous transaction de l'index vectoriel.* — `1fb72299-2e57-0783-1282-77e12996fc76`

- L'index vectoriel peut laisser une ligne injoignable dès sa construction
- L'élagage immédiat des voisins plante dans le chemin de suppression
- Les points d'entrée de l'index vectoriel vivent hors transaction
- Index vectoriel : un DELETE annulé par ROLLBACK ne le laisse plus muet ; un survivant vers lequel ne menaient que des nœuds supprimés reste
- Non vérifié

### Gestion des transactions manuelles — 3 sections

*Ce chantier traite du comportement et de la conformité du moteur lors des transactions manuelles BEGIN, COMMIT et ROLLBACK.* — `0b6cbc7b-081f-4fe5-1bdd-7ad0da00541a`

- Marche A2 : les offsets provisoires des nœuds d'une transaction sont remappés au commit ; les insertions sont journalisées au commit ; au co
- Ce que T0 change pour un client
- Marche T0 : après une erreur dans BEGIN … COMMIT, tout est refusé jusqu'au ROLLBACK (TransactionContext::MANUAL_TRANSACTION_FAILED), erreur

### Orchestration des sessions rag3weaver — 3 sections

*Cette fiche couvre le suivi, les bilans d'arrêt et la coordination des sessions de travail sur rag3weaver.* — `90e15c16-72f5-8fff-a1ec-4b10e20e41f7`

- Règles de travail
- Confirmation produit (arbre principal, 10 octobre au soir)
- Arrêt du 2 octobre 2026, 01 h 25

### Rangement des textes en mémoire longue — 3 sections

*Cette fiche traite de l'algorithme de classement des textes et de l'évaluation de la similarité cosinus pour le rangement en mémoire longue.* — `ccec2c6c-9726-a648-081d-6f87f9280027`

- ranger
- Et une forme à retenir pour le nœud
- décider qu'il faut créer

### Synchronisation par périmètre — 3 sections

*Ce chantier couvre le mécanisme de synchronisation par instantanés de périmètre et la gestion des sessions associées.* — `75b25386-f886-d805-538f-c5749af828b5`

- Synchronisation par périmètre
- transition
- 2 et 3 octobre 2026, synchronisation par périmètre

### Validation des déclarations de configuration sans effet — 3 sections

*Ce chantier traite du rejet par validation des options de configuration acceptées mais jamais appliquées.* — `548ecf35-6cfb-a894-385f-d80535357940`

- 710
- 1995
- 1985

### Chantier codeparsers-genre-d-usage — 2 sections

*Ce chantier suit les développements du genre d'usage, de la pile de pointeurs et des outils d'analyse associés dans codeparsers.* — `a4086a64-38a8-368f-dea6-59a034bf8fb7`

- Codeparsers
- codeparsers-genre-d-usage (sous-module codeparsers, dépôt L-Defraiteur/codeparsers)

### Chemin fixe de l'extension vectorielle au bâti — 2 sections

*Cette règle signale le risque de collision lors de bâtis concurrents dû au chemin fixe de l'extension vectorielle.* — `697fb1af-a30e-ea9d-f972-6b441f007aef`

- 9. Le « bâti à part » qui n'est pas à part
- Précision du même soir, et elle corrige la première version de cette note

### Déclaration unifiée des modèles dans rag3weaver — 2 sections

*Ce chantier unifie la déclaration et la résolution des modèles locaux ou distants pour chaque capacité de rag3weaver.* — `2512f993-f12c-a5d3-eef2-b7aad30db1b0`

- Seul l'embarquement sait dire « je calcule ailleurs »
- modeles-declares (fusionnée)

### Forks burn, cubecl et cubek — 2 sections

*Suivi et état des forks externes de burn, cubecl et cubek utilisés par le projet.* — `c7e87664-05c0-90c5-a948-2bda4754e826`

- Forks burn, cubecl, cubek
- Adresse professionnelle sur les forks

### Publication des modèles sur Hugging Face — 2 sections

*Cette fiche couvre la publication et l'hébergement public des poids de modèles sur Hugging Face.* — `67bd63cc-96be-8453-682c-ef180d0f9c54`

- Les poids granite ne sont ni sur ce poste ni publiés
- Les poids ne dépendent plus d'aucun disque

### Repli des KB en entités dérivées — 2 sections

*Ce chantier couvre le repli des bases de connaissances en entités dérivées et la mesure des poids de fusion.* — `4e52fc0e-eaf8-a1ea-640f-3a2cf929e5d3`

- 3 octobre 2026
- Repli des KB en entités dérivées

### Services d'embarquement et carte partagée — 2 sections

*Ce chantier traite de la régulation de charge sur carte graphique partagée et du raccordement aux services d'embarquement.* — `9136bc33-b539-8a74-ae0d-ddad7bc514fb`

- regime-carte-partagee-2
- (services sur luciepc)

### Tests de l'API C — 2 sections

*Cette fiche couvre le suivi et la correction de la suite de tests de l'API C.* — `81660708-2b67-8e3e-ead2-19777b476449`

- c_api_test : deux tests faux depuis le renommage du fork (le moteur était juste)
- c_api_test entre dans la liste de livraison de toute marche du moteur

## 2. Les 37 fiches à un chantier

| fiche | ce qu'elle couvre | la section du journal | uuid |
|---|---|---|---|
| Bac à sable des commandes de l'agent de code | Ce chantier traite du confinement et du bac à sable des commandes exécutées par l'agent de code. | Bac à sable des commandes de l'agent de code | `7ee6174e-0705-a17e-3090-5f2578b9d688` |
| Balayage de table sur UNWIND MATCH par clé | Ce chantier traite de l'anomalie du planificateur qui effectue un balayage complet de table lors d'un UNWIND suivi d'un MATCH sur clé primaire. | UNWIND $items AS item MATCH (n {_uuid: item.champ}) balaie la table entière | `aa9a0335-5bb8-a3c6-2b89-71aaaa65766d` |
| Champ folds des scopes | Ce chantier concerne l'intégration et le remplissage du champ folds dans les structures de scopes. | Champ folds des scopes | `5edb5708-e438-4f66-0076-caee9b81ee4a` |
| Chantier amont Vela | Ce chantier assure le suivi des contributions et correctifs portés sur le dépôt distant Vela. | Amont Vela | `2c70dc2e-a643-7563-f1f5-72d2b61ec2ed` |
| Chantier des PR amont burn et cubek | Ce chantier suit la préparation et la soumission des correctifs aux dépôts amont de burn et cubek. | PR amont burn / cubek | `552190dd-3392-6cc6-fe23-d3e7efc6041d` |
| Chantier everything declarative | Prototype tout déclaratif structuré en six lots pour le moteur de script, le nœud scripté et le rechargement à chaud. | 10 octobre 2026, 23 h — chantier I « everything declarative », branche proto-declaratif | `1df90f1a-ec42-619c-3d90-591eb9ed26e5` |
| Chantier lucivy | Suivi des versions et des usages de la bibliothèque lucivy publiée sur crates.io. | lucivy | `b798cedd-3a0b-a740-f384-fcd26e7ac3ec` |
| Chemin de masse des lots de naissances | Ce chantier traite de l'optimisation et du débogage de l'insertion en masse des lots de naissances. | Chemin de masse des lots de naissances | `ce585edd-1816-8fee-ec2e-c8641558f624` |
| Collision d'identité des lignes entre cellules | Cette fiche traite du calcul d'identifiant de ligne indépendant de la cellule et des risques d'écrasement inter-cellules. | L'identité d'une ligne ne dépend pas de la cellule | `63cb22e9-9967-c79e-31cf-09afb9c6de2a` |
| Comportement de SET et CREATE face aux champs nuls | Ce chantier traite de la divergence de comportement entre les instructions SET et CREATE face aux champs nuls dans le cœur C++. | Tranché le 3 octobre par l'orchestration, sans le faire trancher à Lucie | `f932b965-d89c-acc3-0bb3-2f325e3ab221` |
| Comptage count(*) sur QUERY_VECTOR_INDEX | Ce chantier traite de l'anomalie où la fonction count(*) sur QUERY_VECTOR_INDEX retourne toujours k. | QUERY_VECTOR_INDEX … RETURN count() rend toujours k | `d6d117f4-6c95-5727-e47b-7eb9410b5e95` |
| Condition de validité associée au verdict | Cette règle impose qu'un banc d'essai, un test ou un rapport affiche ses conditions de validité aux côtés de son résultat. | Et la forme commune aux neuf | `e541963b-704f-f65f-0569-e29ebd3f2275` |
| Continuations de ligne dans les scripts de génération de code | Cette règle traite des pièges d'espacement et de coupure de ligne lors de l'écriture de code source par un script. | 10. Le script qui écrit le code avale la continuation de ligne | `cc4613cf-a60b-d27f-9517-9783f1d8197b` |
| Contrôle de cohérence entre extension et moteur | Cette règle traite de la détection et du signalement des écarts d'âge ou de version entre les extensions et la bibliothèque du moteur. | 7 bis. L'extension et la bibliothèque d'âges différents, et rien ne le voit | `952c6535-9cce-65d6-5535-4e108ab8515a` |
| Dérivation de l'entité Subject | Cette fiche couvre la définition de l'entité Subject comme entité dérivée afin d'agréger sa description et son contenu. | Une correction à porter quand le lot des sujets viendra | `584fe7ab-70ea-9857-ee97-c5298c964e83` |
| Exclusion des fichiers générés volumineux | Politique visant à identifier et écarter de l'apprentissage des agents les fichiers de code générés de grande taille. | Les géants générés | `d15b2424-0528-b52a-5442-06f364153f6f` |
| Exploration des relations dans le produit code | Cette fiche rassemble les orientations de conception et les visions retenues pour l'exploration des relations dans le produit code. | Visions | `3d73d759-7e26-efc5-23b0-40a9f3396bd8` |
| Indexation de nos propres dépôts par codeparsers | Ce chantier vise à maximiser la couverture d'analyse et d'indexation du code C++ et Rust sur nos propres dépôts. | 10 octobre 2026, soir — codeparsers, chantier « indexer nos propres dépôts » (B4, B5, B2/B1, B3), master de codeparsers en avance rapide | `6c7dcf5a-a99f-3d87-995e-d6d556680d35` |
| Indexation en tâche de fond | Ce chantier couvre l'exécution de l'indexation en arrière-plan sans bloquer la recherche plein texte. | index-en-fond (fusionnée) | `44dbb088-48a1-b522-30e2-39f2a88e2b5f` |
| Localisation des binaires du moteur en worktree | Cette règle traite de la résolution du chemin des bibliothèques et extensions du moteur lors de l'exécution de tests depuis un worktree. | 6. Le chemin du moteur dérivé du chemin du code | `26756be1-857c-e382-0325-d9169e0ddf25` |
| Mise à jour des sous-modules Git en worktree | Règles et commandes pour synchroniser les sous-modules Git après un rebase dans un worktree. | 1. Le sous-module reste en arrière après un rebase | `c71e2438-60e7-9979-58ff-9270e6272098` |
| Montage des réactions de la mémoire longue | Chantier d'implémentation et de raccordement automatique des réactions du moteur réactif pour la mémoire longue. | 11 octobre 2026, 4 h — mémoire longue, le montage des réactions : FAIT, sur master | `2802212b-c622-deee-ba0f-3d62bc7c1d01` |
| Nœud de modèle de décision | Ce chantier couvre la conception et l'évaluation d'un nœud de modèle de décision générique et branchable sans entraînement interne. | Modèle de décision | `47b28556-90ef-4ec7-9883-ad4856d2f009` |
| Offsets provisoires dans l'extension de plein texte | Ce chantier traite de l'anomalie où l'extension de plein texte conserve des offsets provisoires non remappés par la marche A2. | L'extension de plein texte range des offsets provisoires | `6ab722bb-3e39-3390-571e-f7a22540ac56` |
| Orientation des comparaisons dans git diff | Cette règle rappelle le sens des bornes dans git diff pour éviter de prendre ses propres ajouts pour des suppressions. | 8. Le diff qui montre vos propres fichiers comme supprimés | `f9903f8f-66b0-b3a9-42b0-7f26dbde8ba7` |
| Partage de git stash entre worktrees | Cette règle proscrit l'usage de git stash en environnement multi-worktrees au profit de fichiers patchs. | 2. git stash est partagé entre les worktrees | `7e31f788-511a-7dbd-ba03-1b4734a6eda7` |
| Politique vis-à-vis des dépôts amonts | Cette règle fixe la politique de consultation sans transmission ni copie directe vis-à-vis des projets amonts. | Les amonts : on les lit, on ne leur transmet rien, on ne copie pas | `9a372d67-17f5-80c4-4837-38b8c52ed316` |
| Produit mémoire longue | Cette fiche rassemble les principes fondamentaux, choix d'architecture et visions du produit de mémoire longue. | Mémoire longue | `a977b49c-0906-5378-6e34-08d83c9935c9` |
| Refus de COPY après ALTER TABLE DEFAULT NULL | Ce chantier traite de l'anomalie du moteur refusant toute commande COPY qui omet une colonne préalablement ajoutée avec DEFAULT NULL. | Une colonne ajoutée par ALTER … DEFAULT NULL fait refuser tout COPY qui l'omet | `ac70b216-30e2-0f22-ff90-7c11d44c14a0` |
| Relecture du fichier généré plutôt que du script | Cette règle préconise d'inspecter directement le fichier produit ou le diff plutôt que de relire le script de génération. | relire le fichier, pas le script | `5470d7bb-a433-34e3-fb93-1847486159e1` |
| Récupération des lignes supprimées dans rag3db | Ce chantier concerne la récupération d'espace et la purge des lignes supprimées dans rag3db. | Récupération des lignes supprimées dans rag3db | `d4e9ffa1-c127-89ac-ac72-259b3bcd87a2` |
| Réouverture de base sur journal WAL vide | Ce chantier traite de l'anomalie provoquant une erreur lors de la réouverture d'une base dont le journal WAL est vide. | e2e_idempotent_registration rougit environ une fois sur dix à la réouverture | `d8eb4f94-5e5f-3dcb-c220-f3c7779f0dfa` |
| SIGSEGV avec buffer pool de 1 Gio | Ce chantier traite de l'anomalie provoquant un plantage SIGSEGV lors de l'utilisation d'un buffer pool réduit à un gigaoctet. | SIGSEGV avec un buffer pool de 1 Gio | `1f545947-0b0c-023d-8513-f1ea542249ea` |
| Statut d'index dans le chat | Ce chantier concerne l'affichage direct par l'application de chat de l'état d'avancement de l'indexation. | Ligne de statut d'index dans l'application chat | `faa0ab36-4e27-4b12-841e-a5245541d758` |
| Sélection automatique du mode de recherche | Ce chantier traite de la sélection automatique du mode de recherche en fonction de l'état d'avancement de l'indexation. | La recherche choisit seule son mode | `523ffd59-a14a-1a4e-f315-4e0b57312573` |
| Transition d'état des modèles d'embarquement | Ce chantier formalise la requalification d'un modèle d'embarquement en ancien modèle comme une transition d'état déclarée dans le cycle de vie. | Un modèle d'embarquement requalifié en ancien = une transition d'état déclarée | `2ec6ef68-aebe-e432-2c39-cfb8f09c3524` |
| Typage des champs NULL dans les structures littérales | Ce chantier traite de l'inférence de type pour un champ NULL présent dans une liste de structures littérales. | Un champ NULL d'une liste de structures littérales prend le type des autres éléments | `c95ef350-beff-9c3a-80f5-7aa4d8a02a99` |

## 3. Les sections qui rejoignent une fiche existante

| fiche existante | la section du journal |
|---|---|
| régime GPU | Pourquoi aucun régime doux ne sauve ces suites-là |
| régime GPU | « complète hors carte locale » |
| bilan : tâche réelle, bornée, dans ce worktree du dépôt rag3db (extension/rag3weaver). ticket : d… | Indexer un dépôt |
| bilan : tâche réelle, bornée, dans ce worktree du dépôt rag3db (extension/rag3weaver). ticket : d… | Une édition pendant l'indexation |
| bilan : tâche réelle, bornée, dans ce worktree du dépôt rag3db (extension/rag3weaver). ticket : d… | Gelé (base en mémoire seulement) : le report de la poussée des blobs d'index |
| bilan : tâche réelle, bornée, dans ce worktree du dépôt rag3db (extension/rag3weaver). ticket : d… | Et le remède échoue |
| Mémoire de code pour agents | Le backend de code |
| Recherche | pas-c-ponderations (130984f61) |

## 4. D'où viennent ces chiffres

- **Le découpage.** Un script de mesure, hors du crate, réécrit le journal en un titre par chantier : une puce « - **…** », un paragraphe
  en gras ou une ligne de tableau, avec la partie d'origine rappelée dans le texte. L'en-tête du journal est écarté. Un sous-titre
  en gras isolé devient aussi une section : c'est le bruit du découpage, que l'archiviste range ou juge « none ».
- **L'archiviste de master** (86b7ebf3a), avec `gemini-3.8-flash` par Vertex :
  - le nom exact, puis la voisine, parmi **toutes** les fiches ;
  - une phrase d'au moins six mots ;
  - le refus des noms datés.
- **Le protocole.** La passe a joué sur une copie de l'instantané à luciepc, et la base rendue a été posée backend fermé. L'essai sur
  copie qui l'a précédée comptait 56 fiches créées, en-tête compris.
- **Les données** sont à luciepc : `~/.cache/rag3weaver-build/chantiers-96-reelle/` (`analyse.json`, le document découpé, la base rendue).
