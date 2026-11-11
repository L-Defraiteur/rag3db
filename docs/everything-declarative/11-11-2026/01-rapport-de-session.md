# Rapport de session — everything declarative, au 11 novembre 2026

## État courant après la passe du relais — priorité de reprise

Le lot des réclamations a atterri en `91e5f760b` et ses artefacts propres ont été
nettoyés ; les sections anciennes ci-dessous restent historiques. Le relais
contextuel attend uniquement son intégration autorisée et son nettoyage : source
ED finale `09e86e689`, branche `cx-ed-relais-commun-4`, worktree luciepc
`~/git_workspaces/cx-ed-relais`. Ne pas relancer une passe ni toucher le lot remaining.

Ticket du parcours commun :
`docs/everything-declarative/tickets/2026-11-11-le-chat-de-l-atelier-n-expose-pas-les-outils-de-recherche-ni-wait-output.md`
(ED runtime/intégration, archi UI). Ticket de la version contrôlée/exécutée :
`docs/everything-declarative/tickets/2026-11-11-une-invocation-peut-executer-une-autre-version-que-celle-verifiee.md`
(ED). Tous deux restent ouverts avant atterrissage.

Verdict agrégé relu : features 5/5, gardes 10/10, lib entière **2114/0/64 ignorés**
en 42,84 s, lectures **24/0/0** en 10,24 s, PostgreSQL privé **34/0/0** en 7,67 s,
2/2 suites sans SAUTÉ sur `09e86e689`, sous poste lourd. Les ignorés de la lib
restent ses marques existantes, le navigateur étant joué séparément.
Archi : **3/0/0 en 132,16 s** sur `36865616d`, ELF
`6782d8a52d41986af857b99ba85d051dd79557dc97f67419b782caba9673a607`,
220 chemins/contenus actuels présents. La réutilisation de cette preuve UI est
acceptée par orchestration après lecture du seul ajout `#[cfg(test)]` au wrapper
privé `call_tool` entre 368 et 09. Les 39 navigateurs sur a92 et les preuves 458
restent distincts ; ni Gemini ni les 39 n'ont été rejoués pour ce diff.

Preuves copiées aux deux postes : `~/.cache/rag3weaver-build/I-luciepc/relais-20261111/`
(`verdict-final`, `verdict-final.json`, logs `*-commun-5.log`, rouge déterministe
`version-rouge-949dcb556.log`, vert `version-vert.log`) et
`cx-ui-contextuel-36865616d-cibles-final-20261111/` (navigateur et attestation ELF).
Le rouge de compilation 368 (raccourci privé devenu mort hors tests) et le début
de passe arrêté car lancé sans poste lourd sont conservés séparément ; aucun vert
n'est revendiqué sur ces tentatives. Source finale et preuves restent immuables.

Les suites séparées restent ouvertes : remaining (`cx-ed-reste-explicite-3`,
`1acc67338`, juge rouge 18 bilans/26 après 600 s), erreurs SQL masquées de dette,
et accord de commande hors domaine (ticket désormais dans le dossier ED).
Ces problèmes ne sont pas corrigés par le relais. L'archiviste automatique reste
une conception mémoire distincte, sans nouvelle activation ni orchestrateur livré.


## Reprise du 11 novembre après 17 h — état en cours

- Lot repris sur `d1345a115` : branche `cx-ed-reclamations-rendues-3`,
  worktree luciepc `/home/lucied/git_workspaces/cx-ed-reclamations`.
  Correctif `1d5eefde2`, témoin de la dette annoncée `51bf676e4`, branche poussée.
- Lib épinglée : `rag3db-build-29a7bc52aaf11e6204f9beafaffb154105417e13-Release-p12-v11-g17-s18-rcOFF-stOFF-asanOFF-tsanOFF-ubsanOFF`.
- Rouge observé sans correctif : `refused_chunks_are_retaken_by_the_next_index_after_a_reopen`,
  dette 6 au lieu de 0 après refus, fermeture et réouverture avec service sain (2,17 s).
  Avec correctif : vert (3,95 s). Rattrapage : trois verts sans correctif, donc non-reproduction
  de l'intermittence ; un vert avec correctif (27,11 s).
- Première passe : `check_features` VERT 5/5 ; lib 2094 verts, un rouge, 58 ignorés
  (étape navigateur séparée non jouée). Le rouge attendait la ligne de fin ancienne sans dette ;
  le témoin vérifie maintenant exactement les 12 morceaux dus sans embarqueur.
- Deuxième passe complète en cours sous `poste lourd`, unité `cx-ed-reclamations-passe-2` :
  check_features, lib entière, navigateur, e2e_lectures et quatre suites e2e_tx.
  Aucun verdict vert global, aucun tour ni push master.
- Preuves et scripts sur luciepc : `~/.cache/rag3weaver-build/I-luciepc/reclamations-20261111/`
  (`refus-rouge-1.log`, `rouge-{1,2,3}.log`, `vert-*.log`, `lib.log`,
  `check-2.log`, `lib-2.log`, `passe-2.log`, `passe.sh`).
  Target unique inchangé : `~/.cache/rag3weaver-build/target-I-apres`.
- Ancien worktree distant `rag3db-I-apres` conservé détaché sur `1d5eefde2` ;
  `extension/rag3weaver/ir/Cargo.lock` non suivi préexistant conservé.
- Relecture vues marche 1 transmise à arbre principal/recherche : empreinte du contenu réellement
  chunké, vecteurs par modèle, couverture des occurrences après déduplication, parent/génération
  épinglés, filtre par occurrence avant top-k, source et cible des arêtes visibles, lecture d'une
  branche sans worktree. Aucun changement de `code.rs`.
- Inventaire chat transmis à archi : atelier ne propose pas search_code/grep_files/list_files/
  outline/usages/impact/callees, présents dans le gabarit code ; accords et reprise doivent garder
  le dépôt et la vue. ED tient les contrats/gabarits/runtime, archi le parcours et l'UI.

Le rapport historique ci-dessous décrit l'état avant cette reprise.

> **Le premier lot à jouer** : le correctif des réclamations du rattrapage, branche
> `reclamations-rendues`, commit **9da8e4433**, **jamais compilé ni joué**. Il rend la CI de master
> verte (le témoin `owed_vectors_are_caught_up_on_open_when_the_manifest_declares_it` y rougit par
> intermittence). Repartir de cette branche, pas de master : la marche est au § 8.1.

*Écrit le 11 novembre 2026 vers 16 h 30 par la session « everything declarative » (abrégée **ED**)
de Claude Code, pour un agent qui reprend sans aucune mémoire de nos échanges. Le savoir de fond
(comment marchent les pièces) est dans `02-knowledge-dump.md`, à côté de ce fichier.*

## 0. Les noms à connaître pour lire ce rapport

- **ED** : cette session, « everything declarative ». Son domaine : le backend déclaratif de
  rag3weaver (manifestes `backend.json`, gabarits livrés, outils en graphes), l'hôte des backends,
  l'atelier, l'équipier par pas et son juge.
- **c7** : la session d'orchestration (elle fixe la file des pushs, dit « à toi » quand c'est notre
  tour). Les autres sessions citées : **archi** (l'IDE React et les crochets), **96** (la mémoire
  longue de Lucie), **recherche**, **G** (l'optimiseur, qui pose les libs et l'hôte du portable),
  **hooks-essai**, **codeparsers**.
- **Le portable** : le poste de Lucie (`/home/lucied/git_workspaces/rag3db`, son arbre, où **personne
  ne commite**). **luciepc** : l'autre poste, joignable par `ssh lucied@luciepc`, où tournent les
  passes et les pushs. Son shell interactif est fish : une commande distante complexe passe par
  `ssh lucied@luciepc bash -s <<'EOF' … EOF`.
- **Une passe** : le jeu de tests qu'on joue avant de pousser (la lib entière par
  `cargo test --lib`, `scripts/check_features.sh`, et selon le lot des e2e). **Vert** = aucun rouge
  et aucun test sauté. **Le verdict** : le fichier `~/.cache/rag3weaver-build/I-luciepc/l2/verdict`
  de luciepc, qui porte le commit qu'une passe a jugé vert ; sans lui, le script de push refuse.
- **Un témoin** : un test écrit pour prouver un défaut. **Rouge d'abord** : on le joue sur le code
  d'avant pour le voir rougir, puis sur le correctif pour le voir vert.
- **Les trois contrôles** après un push : `git status` propre, aucun rebase en cours, et
  `origin/master` égal à `HEAD`. Un push n'est « fait » qu'après les trois.
- **La lib** : le moteur C++ rag3db, bâti à part et rangé dans un « magasin »
  (`~/.cache/rag3db-libs/<id>`). Les passes l'épinglent par `RAG3DB_LIB=<id>`. Depuis le
  9 novembre : `rag3db-build-9bae60786b05ec1463c60d2630a4de4444bc2cfd-Release-p12-v11-g17-s18-rcOFF-stOFF-asanOFF-tsanOFF-ubsanOFF`,
  **sans** la variable `RAG3WEAVER_MOTEUR_ANCIEN` (vérifier auprès de c7 qu'elle n'a pas changé).

## 1. Ce qui est sur master de cette session (8-9 novembre)

| Commit | Lot |
|---|---|
| 4f54e473a | Le juge par pas : un pas de l'équipier se juge sur son but, fiche « bilan : <tâche> » |
| 7276a02fa | Le témoin des symboles cités relit quand le catalogue est tenu ou l'index partiel |
| f16628f8b | La pile de l'atelier (17 commits) : commandes hors tâche, gabarits vivants, reprise du premier index, `catch_up_on_open`, phrase de l'index |
| cad8944f5, d6c4ca144 | Deux tickets (e2e_lectures fermé en rouge d'environnement ; les morceaux refusés) |
| 0da9fdda4 | Le témoin du rattrapage, vert sous les deux régimes de service |
| a81f947d4 | Le champ `ui` : la carte d'avancement sous le tour, `show_progress` |
| 8be79cbbf | Un index qu'un outil attend tient la porte (`lancer_l_index` réclame l'index) |

Depuis, archi a bâti sur ces lots (la carte de page 623e97249 ; l'atelier qui lit et écrit,
d23af9cd2 et les commits du 9 novembre de 11 h à 14 h). 96 a corrigé le recul du modèle
(a6637eeee).

## 2. Le correctif des réclamations du rattrapage — pas sur master

**Le défaut** (lu le 9 novembre à 02 h 45). Une passe d'embarquement de vecteurs réclame ses
morceaux en posant `_embed_claim = "<horodatage>|<écrivain>"`. Une réclamation ne périme qu'après
`MARQUE_PERIMEE_MS` = 60 s (`extension/rag3weaver/src/catalog.rs`, ~3395), et une passe ne
reprend d'office que les siennes (`extension/rag3weaver/src/dialect.rs`, ~2293 : `ENDS WITH $mien`).
Quand une passe est coupée (fermeture du backend, arrêt de l'hôte), elle garde ses réclamations
(jusqu'à 512 morceaux par table). Le catalogue rouvert a un autre identifiant d'écrivain : il saute
ces morceaux pendant 60 s, et un rattrapage qui finit dans ce délai se dit « indexation terminée »
en laissant une dette.

**Pourquoi la CI rougit par intermittence** sur
`daemon_host::worktrees::tests::owed_vectors_are_caught_up_on_open_when_the_manifest_declares_it`
(`extension/rag3weaver/src/daemon_host/worktrees.rs`) : ce témoin coupe un rattrapage par une
fermeture, puis rouvre. Si la fermeture tombe **pendant** une passe en vol, les réclamations
restent, et l'étape suivante voit « la dette n'est pas soldée » (1 024 morceaux = 2 tables × 512).
Si elle tombe entre deux passes, il est vert. Le témoin a raison contre le produit ; c7 l'a nommé
« rouge connu » pour les passes des autres. Deux formes du témoin existent :
- sur master (0da9fdda4) : il mesure la dette **en rouvrant** le dépôt, ce qui relance l'index et
  fausse la mesure sur un runner lent (deuxième cause de rouge) ;
- sur la branche `temoin-rattrapage-sans-reouverture` (**219053a4b**) : la coupure se lit à la
  dernière ligne du journal d'index, sans rouvrir ; 1 500 fichiers pour garder plusieurs passes.
  Cette forme est le **rouge d'abord** du correctif : rejouée trois fois, verte deux fois, rouge à
  la troisième sur le vrai défaut.

**Le correctif** (branche `reclamations-rendues`, **9da8e4433**, au-dessus de 219053a4b
cherry-pické en 1fdc09752, sur 48c93d7ab) :
- `SchemaDialect::rendre_reclamations(table)` dans les deux dialectes (rag3db, Postgres) ;
- `Catalog::rendre_mes_reclamations()` : vide `_embed_claim` là où il finit par l'écrivain de ce
  catalogue, sur toutes les tables de morceaux ;
- dans le fil d'index (`extension/rag3weaver/src/dataflow/index_nodes.rs`, `spawn_index_with_stop`) :
  les réclamations sont rendues **à toute sortie** (finie, en échec, arrêtée) ;
- la ligne de fin dit « reste dû : N morceau(x) sans vecteur — sans service d'embarquement, ou
  réclamés par un autre écrivain » quand une dette reste sans muet ni échec ;
- un témoin de plus, `refused_chunks_are_retaken_by_the_next_index_after_a_reopen`, pour le ticket
  des morceaux refusés (même cause présumée : refus, réclamations gardées, réouverture).

**Où il en est** : écrit et poussé en branche le 9 novembre à 09 h 27, **jamais joué**. La session
attendait depuis une approbation de Lucie sur une commande, qui n'est pas venue.

## 3. Les défauts de l'installeur de l'atelier (non faits)

`rag3weaver-backend atelier` (`extension/rag3weaver/src/bin/rag3weaver-backend.rs`, `atelier_cli`) :
1. sur un atelier **déjà posé**, il répond « déjà posé » et ne note pas la pose (`livre.json`
   absent) : le gabarit vivant ne le remettra jamais au livré ;
2. sur une **pose neuve** du 8 novembre au soir, aucun `livre.json` n'est apparu sous
   `~/.local/state/rag3weaver/manifestes/`. Première chose à vérifier : que le binaire lancé
   contenait `record` (bâti après f16628f8b), puis `XDG_STATE_HOME`, puis l'empreinte du chemin
   canonique (`builtin_templates::state_dir_of`).
Témoin rouge d'abord pour chacun. M8 (ci-dessous) propose « l'adoption d'une copie d'avant
livre.json », qui couvre le premier cas.

## 4. Les tickets de l'équipier (essai du 9 novembre par 96)

Contexte : `extension/rag3weaver/docs/09-novembre-2026-memoire-longue/01-l-equipier-sur-les-readme.md`.
Une tâche réelle sur Gemini : douze appels au modèle, l'un d'eux 1 004 s en reprises muettes, puis
un seul `step_done` et l'hôte a clos la tâche « fait sans plan » sans rien livrer.
- **Le recul invisible** (`docs/memoire/tickets/2026-11-09-le-recul-du-client-du-modele-ne-se-voit-pas-et-n-a-pas-de-borne-de-pas.md`) :
  **fait par 96** (a6637eeee : chaque reprise après un 429 ou un 503 se dit et se borne). Le ticket
  est encore ouvert : à fermer avec ce hash après relecture.
- **La fin sans livrable** (`docs/memoire/tickets/2026-11-09-une-tache-sans-plan-se-dit-faite-sur-un-step-done-qui-dit-ce-qui-reste.md`) :
  **à ED**. Une tâche ne doit pas se dire faite sans le livrable demandé ni l'accord de la boîte
  (voir la règle de fin dans le knowledge dump, § 5).
- **La cause des reculs** (`docs/memoire/tickets/2026-11-09-le-code-d-un-refus-du-modele-n-est-garde-nulle-part.md`) :
  pour ED et Lucie (son quota Vertex) ; le run compte les reprises mais ne garde ni le code rendu
  (429, 503) ni l'heure.

## 5. Les relectures demandées (non faites)

- **Les vues par worktree, marche 1** :
  `extension/rag3weaver/docs/08-novembre-2026-arbre/01-conception-des-vues-marche-1.md` (10b8affce).
  Priorité fixée par Lucie le 8 au soir. La part d'ED : le gabarit de code et `code.rs` (la version
  de scope comme entité, la vue déclarée, l'index qui n'embarque que le neuf). Rien à coder avant
  la page relue.
- **L'atelier lit et écrit** :
  `extension/rag3weaver/docs/09-novembre-2026-atelier-lire-ecrire/01-conception.md`, codé et poussé
  par archi le 9 novembre (`extension/rag3weaver/src/daemon_host/atelier_fichiers.rs`, `read_file`,
  `replace_text`, `replace_lines`, `insert_lines`, `read_scope`, `ask_permission`, la carte
  d'accord, `writes_outside_tasks`, la racine `~/rw-code`). Poussé sans la relecture d'ED.
- **M8, la montée des gabarits** :
  `extension/rag3weaver/docs/09-novembre-2026-memoire-longue/02-m8-la-montee-des-gabarits.md`
  (96, 0fff7aa00). Elle étend le gabarit vivant d'ED : adopter une copie posée avant `livre.json`,
  étendre la base aux fichiers copiés (graphes, scripts), fusion à trois au-delà, refus nommé.
  « Rien ne se code avant la relecture d'ED. »

## 6. Le lot « commit des gabarits livrés » (non commencé)

Tranché par c7 le 8 novembre : un manifeste suivi par un dépôt git de déclarations (un arbre
inscrit par `code-tree add` ou `attach_worktree`) n'est aujourd'hui pas remis au gabarit livré
(`Refresh::TrackedByGit`). La forme voulue : la remise au livré se fait par **un commit** dans ce
dépôt, à l'identité neutre (`crate::backend_code::git_identity_or_default`, posée par hooks-essai :
`rag3weaver <rag3weaver@localhost>`), message « gabarit livré <hash du binaire> : <fichiers> », sur
la branche courante, seulement si les fichiers sont propres ; sinon refus dit sur stderr et sur le
flux ; `TemplateRefreshed` porte le hash du commit. Aucune ligne de code, aucune branche.
Dépend aussi de M8 (à relire d'abord).

## 7. Mes tickets ouverts

Huit sont dans `docs/everything-declarative/tickets/` (déplacés le 11 novembre). Les quatre du
9 novembre, ouverts par la session mémoire (96) mais portés par ED, sont dans
`docs/memoire/tickets/` (96 les y a déplacés, 14645b23c) : on les lit et on les ferme là. Une ligne
chacun ; nom de fichier seul.
- `2026-11-08-des-morceaux-refuses-par-le-service-ne-sont-jamais-repris.md` — perte ; correctif
  présumé dans `reclamations-rendues` (même cause que le rattrapage), à prouver par son témoin.
- `2026-11-09-une-tache-sans-plan-se-dit-faite-sur-un-step-done-qui-dit-ce-qui-reste.md` — la fin
  sans livrable de l'équipier.
- `2026-11-09-le-code-d-un-refus-du-modele-n-est-garde-nulle-part.md` — garder le code et l'heure
  de chaque refus du modèle dans le run.
- `2026-11-09-le-recul-du-client-du-modele-ne-se-voit-pas-et-n-a-pas-de-borne-de-pas.md` — fait par
  96 (a6637eeee), à fermer.
- `2026-11-08-l-equipier-sur-gemini-ce-que-les-essais-reels-ont-montre.md` — porteur ED ; une tâche
  réelle s'arrête ou tourne à vide (attente du superviseur, ligne de boîte perdue).
- `2026-11-09-les-temoins-ecrivent-leur-etat-de-manifeste-dans-le-dossier-d-etat-reel.md` — les
  tests écrivent des `livre.json` dans le vrai `~/.local/state` (400 relevés) : faire poser
  `RAG3WEAVER_STATE_DIR` par les témoins (dossier temporaire).
- `2026-11-07-le-temoin-du-chat-dans-la-page-rougit-a-la-premiere-execution-apres-un-bati.md` —
  faux rouge du témoin navigateur du chat.
- `2026-11-07-des-tests-de-la-lib-rougissent-sous-charge.md` — témoins de durée sous charge.
- `2026-11-08-le-temoin-de-la-porte-de-l-equipier-lit-le-second-retour-avant-qu-il-arrive.md` —
  course dans un témoin de l'équipier.
- `2026-10-25-le-temoin-des-vues-de-l-ide-rougit-sous-le-lot.md` — faux rouge du témoin des vues
  de l'IDE.
- `2026-10-12-une-lecture-refusee-pendant-une-reaction-une-fois-sur-neuf.md` — « Catalogue occupé »
  une fois sur neuf pendant une réaction en chaîne.
- `2026-10-18-trois-temoins-rougissent-sous-charge.md` — ouvert par ED ; trois témoins instables
  sous charge.
Cité sans être à ED : `2026-11-08-un-backend-ecrit-encore-dans-son-dossier-apres-son-drop.md`
(recherche ; il parle de `stop_indexings_of`, que la session recherche étend).

## 8. Reprendre — les travaux pas finis, worktree par worktree

### 8.1 Le correctif des réclamations (prêt à jouer)

- **Où** : sur le portable, worktree `/home/lucied/.cache/rag3weaver-build/wt-juge`, branche
  `reclamations-rendues`, dernier commit **9da8e4433** (poussé sur `origin/reclamations-rendues`).
  Base : 48c93d7ab, plus 1fdc09752 (le témoin 219053a4b cherry-pické). Master a beaucoup avancé
  depuis : il faudra rejouer sur le master du moment.
- **Fait** : le code (dialectes, catalogue, fil d'index, ligne de fin), deux témoins.
- **Reste** : compiler (rien n'a été compilé), jouer rouge d'abord puis vert, la passe, le push,
  puis fermer le ticket des morceaux refusés si son témoin le prouve.
- **Target** : le target de luciepc `~/.cache/rag3weaver-build/target-I-apres` (20 Go), lié au
  worktree de luciepc `~/git_workspaces/rag3db-I-apres` (aujourd'hui sur 9939a6688, une ancienne
  tête de passe, propre). Rien de compilé pour cette branche.
- **Commandes** (sur luciepc, scripts déjà en place dans `~/.cache/rag3weaver-build/`) :
  1. Rouge d'abord, sur master sans le correctif : dans `~/git_workspaces/rag3db-I-apres`,
     `git fetch origin && git checkout --detach origin/master && git cherry-pick 219053a4b`, puis
     `source <(grep -E '^(LIB=|export RAG3DB|export CARGO|FEATURES=)' ~/.cache/rag3weaver-build/I-court-9bae.sh)`
     et `cd extension/rag3weaver && bash scripts/cargo_magasin.sh test -j 8 --lib --features $FEATURES -- owed_vectors_are_caught_up_on_open`,
     trois fois (il doit rougir au moins une fois sur « la dette n'est pas soldée »).
  2. La passe et le push : copier `~/.cache/rag3weaver-build/I-pousse-reclame.sh` en changeant la
     branche (`index-reclame-par-l-outil` → `reclamations-rendues`) et la ligne
     `git cherry-pick f9f6445d8` en `git cherry-pick 1fdc09752 9da8e4433`, et le dossier de sortie.
     Ce script prend le master du moment, cherry-pick, joue `I-court-9bae.sh` (lib entière,
     check_features, natif seul), les témoins de reprise et du flux, écrit le verdict et pousse par
     `~/.cache/rag3weaver-build/poste push everything-declarative -- bash ~/.cache/rag3weaver-build/I-pousse-l2.sh`,
     qui fait les trois contrôles. Le lancer par
     `XDG_RUNTIME_DIR=/run/user/$(id -u) systemd-run --user --unit=<nom> --collect bash <script>`.
  3. Attendre le « à toi » de c7 avant le push si c7 tient la file.

### 8.2 Le commit des gabarits livrés

Pas de worktree, pas de branche, pas de code : à partir du master du moment, après la relecture de
M8 (§ 5) et en s'appuyant sur `extension/rag3weaver/src/builtin_templates.rs` (`refresh`,
`Refresh::TrackedByGit`).

### 8.3 Ce qui reste sur disque, sans travail en cours

- Portable : `/home/lucied/.cache/rag3weaver-build/wt-gabarits` (worktree des docs d'ED, où ces
  fichiers sont écrits). Branches poussées et atterries, à ne pas supprimer sans c7 :
  `index-reclame-par-l-outil`, `temoin-rattrapage-service`, `temoin-rattrapage-sans-reouverture`,
  `atelier-ui-3`, `gabarits-vivants-3` et leurs aînées.
- luciepc : `~/git_workspaces/rag3db-I-apres` (worktree des passes d'ED) et son target
  `~/.cache/rag3weaver-build/target-I-apres` (20 Go) ; 264 scripts `I-*.sh` et les journaux
  `~/.cache/rag3weaver-build/I-luciepc/` (779 Mo) ; `~/.cache/rag3weaver-build/I-rouge-atelier/`
  (variantes rouges des témoins de l'atelier).

## 9. Ce qui est bloqué, et sur quoi

- Le correctif des réclamations : sur une approbation de Lucie depuis le 9 novembre au matin ;
  aujourd'hui, sur personne, il suffit de le jouer (§ 8.1).
- La CI de master : rouge par intermittence sur le témoin du rattrapage tant que ce correctif
  n'est pas sur master.
- Le lot du commit des gabarits livrés et les défauts de l'installeur : sur la relecture de M8.

## 10. Les trois prochains pas

1. Jouer et pousser le correctif des réclamations (§ 8.1), puis fermer le ticket des morceaux
   refusés si son témoin le prouve : c'est ce qui rend la CI verte.
2. Relire les trois pages (§ 5) : vues par worktree d'abord (priorité de Lucie), puis l'atelier lit
   et écrit, puis M8.
3. Les défauts de l'installeur de l'atelier (§ 3), puis le commit des gabarits livrés (§ 6), puis la
   fin sans livrable de l'équipier (§ 4).

## Validation complémentaire avant intégration

Passe 5 verte sur `bbba12fb2` : features 5/5, lib 2096 verts / zéro échec / 58 ignorés
(marques existantes, dont navigateur joué séparément), navigateur 33/33 sans ignoré,
lectures 24/24 et transactions vecteurs 3/3, arrêt 18/18, merge 1/1, ligne 3/3, sans ignoré
ni SAUTÉ. Le rouge sparse de la passe 4 est soldé en déclarant le service creux pour les e2e.

Rebase sur master `bd9fd9852677a0a942ac775215be4dafc57eace6`, docs seules depuis `41526b29a` :
branche `cx-ed-reclamations-rendues-6`, candidat source identique au commit jugé
(diff src/templates/tests vide), précédent HEAD `9f7e13298`. Aucun push master.
Une suite e2e_postgres complémentaire est lancée sous jeu large sur la base privée
`cx_ed_reclamations_20261111` du conteneur de test, jamais le schéma partagé.
Journal : `I-luciepc/reclamations-20261111/postgres-6.log` ; verdict à lire.

Les erreurs SQL masquées de libération et de lecture de dette sont un défaut connu distinct,
consigné dans le ticket ED du 11 novembre correspondant. Aucun comportement d'erreur n'est
présenté comme validé par les témoins nominaux ; le correctif de propagation reste à faire.

Complément PostgreSQL terminé : `e2e_postgres` 34/34, zéro échec, zéro ignoré,
1/1 suite demandée, aucun SAUTÉ (7,87 s de tests), sous jeu large et base privée.
Les deux gardes préalables passent aussi. Preuve conservée dans `postgres-6.log`.
Ce résultat valide la suite existante, pas les injections d'erreurs SQL du ticket ouvert.
