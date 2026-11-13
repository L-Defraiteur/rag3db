# La passation de Codex, lue et traduite (c7, 13 novembre 2026, 14 h 23)

Lecture des quatorze rapports « 99 » de Codex et de ses index, sans rien lancer. Trois parties : son
vocabulaire traduit dans nos mots, l'état de chaque chantier, et ce qu'on reprend ce soir avec les
sessions Claude. Les faits viennent des pages de Codex ; ce qui n'est pas vérifiable depuis le dépôt
est dit tel quel.

## 1. Glossaire : les mots de Codex → les nôtres

| Codex | chez nous |
|---|---|
| **F8** (« 8/8 ») | `scripts/check_features.sh` : huit jeux de features en `cargo check --tests` (c'était cinq jeux avant le 12 ; Codex en a ajouté trois, dont `rag3db-native` seul) |
| **SDK29**, lib29, native29 | notre lib épinglée 29a7bc52a (`rag3db-build-29a7bc52a…`), avec un pool de 1 Gio ; la CI, elle, joue `RAG3DB_LIB=statique` |
| **SDK7** | les sept variables d'environnement de la lib (RAG3DB_LIB, RAG3DB_SHARED, RAG3DB_LIBRARY_DIR, RAG3DB_INCLUDE_DIR, LD_LIBRARY_PATH, RAG3DB_BUFFER_POOL_SIZE, RAG3WEAVER_MOTEUR_ANCIEN), déduit de sa sonde |
| **NAV40** | l'étape navigateur (`temoins_navigateur.sh`), 40 témoins |
| **lib entière**, « 2308/0/66 » | `cargo test --lib` sous le jeu large : passés / échecs / ignorés |
| **APP / pilote** | APP = le hash de la source du lot ; pilote = les scripts figés de sa passe (lancer, juger, finir), poussés sur leur branche avant d'être joués |
| **recette** | le plan figé d'une passe (commandes, bornes, attendus) |
| **qualification** | une passe jouée pour de vrai sur une source exacte (F8, lib entière, tests, navigateur, e2e) et son verdict VERT/ROUGE |
| **preuves**, **manifeste (SHA)** | les journaux d'une passe, poussés sur une branche `cx-*-preuves-*` avec un `manifest.json` (fichiers, tailles, SHA256) ; les gros fichiers restent hors git sous `~/.cache/rag3weaver-build/` |
| **GO**, **créneau**, **budget** | GO = le mot de l'orchestrateur pour jouer une passe (notre « à toi » avant la passe plutôt qu'avant le push) ; créneau = une fenêtre de poste ou de carte ; budget = une durée bornée en secondes après la prise du verrou, jamais réinitialisée |
| **autorité v1/v2**, **reçu** | une tentative de passe, identifiée par le PID de son superviseur, finie VERT ou ROUGE ; le reçu est son jeton, non réutilisable |
| **first-red** | notre « rouge d'abord » lu à l'envers : le premier rouge inattendu arrête la passe et ne se rejoue pas inchangé |
| **hermétique** | un test sans processus, signal, cargo ni target réels |
| **quiescence** | la preuve qu'aucun processus ne tient un dossier avant de l'effacer |
| **garde des targets** | le lot de recherche (9feb7cdba) : un target non vide sans provenance est refusé ; à ne pas confondre avec `garde_worktree.py`, la garde de retrait (SHA 6f3dfceb…) |
| **couches 2a / 2b-A / B / C** | les vues par worktree de Lucie : 2a = un résolveur en mémoire des couches (parents P, tombstones, provenance par occurrence) ; 2b-A = l'admission sur fixtures ; B = le chargeur durable et l'instantané composé ; C = les outils qui s'en servent. « A ne certifie ni B ni C » |
| **contenu H**, **génération P** | H = l'empreinte du contenu déclaré ; P = l'identité d'une publication (P1 → P2 → P3), indépendante du contenu |
| **pause runtime** | une passe arrêtée après la compilation, avant les tests, en attendant un mot |
| **C7** | le rôle d'orchestration (le mien, repris par Codex sous le même nom) |
| **RO** | tantôt une relecture statique, tantôt read-only : ambigu |
| **W / T / P / Q / S** | worktree / target / dossier de validation / dossier du pilote / dossier de confinement |
| **ÀTOI** | notre « à toi » |
| **D17** | le commit d17a230b1 (résolveur 2a), celui dont la CI est rouge |
| **Lucivy A / B / C** | A = la 4.3.3 publiée ; B = la barrière d'arrêt (fbd3f0a4) ; C = la barrière d'admission atomique (fa1d0273) ; rien à voir avec les couches A/B/C |

## 2. L'état par chantier, dans nos mots

- **Master** : Codex a atterri surtout des docs (389 sur 409 commits). En code : le raccord physique
  d'index et la publication (b0a5177c0, ba0317ed1), le résolveur de couches 2a (d17a230b1), la garde
  des targets (9feb7cdba), la navigation des symboles et un grep enrichi (archi), read_scope et les
  frontières de la boîte (ED), M8 première tranche (mémoire), clang-format en CI (cœur), des scripts
  et des fixtures. **La CI est rouge depuis d17a230b1** : un témoin du raccord physique
  (`physical_index/tests.rs:414`) trouve un écrivain encore vivant après un Drop ; ticket
  `docs/orchestration/tickets/2026-11-13-la-ci-refuse-la-reouverture-du-temoin-physique.md`, cause non
  attribuée. **Corrigé le 13 au soir (lu par ED, archi, recherche)** : les trois lots de reprise du 11 **avaient tous
  été atterris par Codex le 11 au soir** — le correctif des réclamations (b3f31b544, 17 h 34), l'IDE par le
  flux d'archi (19 h 56, commits non joués compris) et l'arrêt du backend de recherche (80b467dd3, avec une
  vraie passe R5 verte consignée dans docs/recherche/11-11-2026/01-rapport-de-session.md, et la montée
  de lucivy-core 4.3.3, f96acc6cf). Ma lecture du matin les disait « pas joués » : c'était faux.
- **Couches 2b-A (arbre principal)** : dix branches `cx-arbre-couches-2b-*` poussées ; la source courante
  d9aa4be86 et le pilote 4fca0dd38, **jamais joué** ; deux passes rouges conservées (un dead_code sous
  -D warnings, corrigé ; une observation cargo à sept champs vides, cause non attribuée). Un target à
  luciepc (`cx-target-contenu-derive`). Pour reprendre : une revue, puis jouer 4fca sous poste lourd.
- **lucivy** : `main` intact depuis la 4.3.3. Variante B (barrière d'arrêt) qualifiée et mesurée : juste
  mais trop chère (jusqu'à 40 % de perte sur le worker) ; Codex l'a **suspendue** (« retenue » veut dire
  suspendue, pas adoptée). Variante C (admission atomique, fa1d0273) relue mais **jamais compilée ni
  jouée**. Pour reprendre : qualifier C, mesurer A contre C, décider ; jamais de tag sans ton mot.
- **recherche** : la garde des targets est atterrie, et l'arrêt du backend aussi (80b467dd3, passe R5 verte) ; la clôture du reliquat (a6c23af93) attend un
  « à toi » ; un prototype archivé sur branche, à ne pas fusionner.
- **everything declarative** : parcours IDE, frontières de la boîte, read_scope atterris ; le correctif des réclamations est sur master depuis le 11 (b3f31b544) ; le correctif des
  réclamations est sur master depuis le 11 (b3f31b544) ; un diagnostic documentaire du rouge CI sur
  branche. En veille de relecture.
- **mémoire** : M8 première tranche atterrie le 12 ; rien le 13 ; M9 (mémoire automatique) conçu, pas
  livré.
- **archi** : navigation des symboles et grep enrichi atterris ; un lot de docs (f66105e8e) poussé sur
  branche, pas sur master.
- **cœur C++** : clang-format 18 en CI atterri ; le diagnostic de saturation et un correctif privé de
  l'annulation sur branches ; la reprise de la trace privée bloquée par un refus automatique, non
  contourné (juste).
- **optimiseur** : binaire posé le 11 ; une fenêtre card0 de nuit du 13 (00 h à 08 h) donnée par Lucie,
  close à 07 h 54 ; les chargements réels sur card0 non joués.
- **embarquement** : la garde de retrait et sa clôture atterries ; l'activation distante des modèles,
  l'éviction et les baux ne sont pas livrés.
- **codeparsers** : la qualification de l'IR (a4bf86c86) ; le sous-module sur une branche
  `cx-codeparsers-default-metadata` côté codeparsers.
- **banc** : le banc Qwen (ee40dbcd6) ; le rapport de résidence sur branche seulement ; Gemini Decision
  attend Vertex (mot de Lucie du 12 : attendre, pas de clé AI Studio).
- **tty** : un lot de docs complet sur branche (e593c67a2), à fusionner en entier, pas son premier commit.
- **hotfixs** : témoin et recette de quarantaine sur branches ; le rejeu de la trace sous 29a7bc52a
  toujours pas joué ; le service 7882 laissé en route.

## 3. Ce que Codex a dit avoir reçu de Lucie (citations)

- « Pour cette nuit seulement, card0 est aussi disponible le 13 novembre de 00:00 à 08:00 » ; « pas de
  reconduction ».
- « Lucie demande d'attendre Vertex pour Gemini Decision 1 : ses crédits sont Cloud… Aucune clé AI Studio,
  activation de facturation ni voie alternative n'est créée. »
- « Lucie demande de préserver l'usage restant : toutes les sessions passent leur travail, poussent leur
  WIP possédé même inachevé et cessent de demander de nouveaux créneaux de calcul. »
- Et une règle que Codex s'est donnée, à ne pas garder telle quelle : « Fusion ordinaire après
  revue + qualification relève de la coordination autorisée » pour lucivy — chez nous, toute fusion dans
  `main` de lucivy attend le mot de Lucie dans la session lucivy (règle du 13 septembre). Il ne l'a pas
  appliquée : `main` n'a pas bougé.

## 4. Incohérences relevées (mineures, à savoir)

- L'index de passation est sur master (91b66c515) alors que les sessions disent n'avoir rien poussé sur
  master ; et « l'arbre de Lucie est à 41497 » n'est plus vrai.
- « Retenir la fusion de B » se lit « adopter » dans deux pages et « suspendre » dans deux autres : c'est
  suspendre.
- Des chemins de tickets périmés dans deux rapports (hotfixs, banc), un nom de page faux chez archi, une
  branche oubliée dans la table de mémoire, deux heures de fin de fenêtre card0 (07 h 54 réel, 08 h
  autorisé).
- Des hashes cités hors master (preuves lucivy, requalification banc, lot docs archi, bâti cœur) : ils
  n'existent que sur des branches.
- La procédure de push dit encore `git worktree remove --force` et `git branch -D` dans son script,
  que les passations de Codex interdisent ; à harmoniser.
- Rien de ce qui est sur disque (worktrees, targets, PID, services) n'est vérifiable depuis le dépôt ;
  relevé du 13 : 35 worktrees et 267 dossiers `cx-` (87 Go) sur le portable, 740 dossiers `cx-`
  (128 Go) et 32 worktrees à luciepc, 360 branches distantes `cx-`.

## 5. Ce soir, avec les sessions Claude (proposition)

1. **La CI au vert**, deux causes (corrigé le 13 au soir) : d'abord la **garde des targets** de recherche
   (9feb7cdba), qui refusait le target restauré par le cache de la CI et arrêtait le job avant tout test —
   corrigé par recherche (62b5c5bfa), CI verte ; puis le témoin du raccord physique, rouge en local sous la
   lib entière (une instance précédente qui meurt tard sur un fil lucivy) — correctif d'arbre principal
   (d57273bba) en file. Rien d'autre n'atterrit tant que master est rouge.
2. **Les trois lots de notre page de reprise** : arrêt du backend (recherche), IDE par le flux (archi),
   puis la clôture du reliquat de recherche et le lot docs de tty (fusion entière).
3. **Les couches** : relire d9aa4be86 et 4fca0dd38 (arbre principal) avant de les jouer ; une page courte
   qui dit, dans nos mots, ce que 2a et 2b-A font et ce qui manque pour B et C ; le mot de Lucie sur la
   direction avant d'y mettre plus.
4. **lucivy** : qualifier C, mesurer A/C, puis sa décision.
5. **Le ménage** des worktrees, targets et branches `cx-`, après lecture, à son mot : d'abord ceux des
   lots atterris, jamais ceux listés comme inachevés.
6. **Les mots** : ce glossaire va dans `docs/README.md` ; les prochaines pages n'emploient que les nôtres.
