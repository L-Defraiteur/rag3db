# Orchestration — rapport de session du 13 novembre 2026 (c7, reprise à 15 h 42)

Lucie a arrêté les sessions Codex (plus d'usage avant le 14 à 16 h) ; la suite se fait avec les sessions
Claude, vivantes depuis le 11. Lecture préalable : `01-lecture-de-la-passation-codex-par-c7.md`.

## Fait aujourd'hui

- **Lecture et traduction de la passation de Codex** (69a884acc) : glossaire, état par chantier,
  incohérences, plan du soir. Verdict : travail sérieux et honnête, mais à côté de la file laissée le 11
  (nos trois lots de reprise jamais joués), une CI rouge depuis d17a230b1 (témoin du raccord physique
  d'index), un vocabulaire à lui, et des centaines de worktrees et targets `cx-` laissés sur les deux
  postes.
- **lucivy** : mot de Lucie « review puis publier si cohérent », transmis. Verdict de la session : la
  variante C de Codex est juste après deux correctifs (clippy ; un réveil perdu dans l'arrêt de
  l'ordonnanceur, présent depuis mars dans la 4.3.3 et dans B), et ne coûte pas ce que coûtait B
  (C/A : −12,7 / −8,5 / −4,6 / +5,3 % sur le worker ; B/A jusqu'à +71 %). Qualification complète à
  luciepc (lib 1480/0, 1 500 tours sous gdb, 200 répétitions), un rouge antérieur (test flou, ticket). PR
  #26, branche 4.3.4, CI en cours ; le tag attend le mot de Lucie dans la fenêtre lucivy.
- **CocoIndex** signalé par Lucie : note de vision `visions/concurrents-cocoindex.md` (3eddecf41) ; le
  README de rag3weaver se refait sur ce modèle (archi, branche `readme-refonte`, e7befa039 : accroche,
  un exemple gardé par `scripts/test_readme_quickstart.sh`, huit cartes à vraies images, `AGENTS.md`
  à la racine ; passe en cours, « à toi » au vert). Deux vérités dites dans le README : les vues par
  worktree ne sont pas livrées (une base par worktree, copiée, jetable), PostgreSQL partiel.
- **Décisions et graines de Lucie** (visions/README.md) : un dépôt officiel rag3weaver séparé de rag3db
  un jour (les docs du crate tiennent seules dès maintenant) ; cinq graines de produits (fuites de secrets,
  analyse de code avec le modèle, revues CI/CD à contexte persistant jusqu'au correctif proposé, un coffre
  de secrets servi aux agents, un classifieur branchable qui remplace les secrets par des références).
  Les noms : `rag3weaver` est réservé sur npm (luciformresearch, 10 octobre) ; libre sur crates.io, PyPI
  et comme organisation GitHub (sa main).
- **Postes** : G bâtit une lib de la tête de master (le C++ n'a bougé que par le clang-format du cœur :
  les passes tournent avec MOTEUR_ANCIEN=1 en attendant) et prépare le binaire (l'installé 0b0ff221a
  refuse le gabarit `backends/code` de master) ; pose à mon mot après le lot README. Désaccord à
  trancher : run_e2e.sh exige 32 Gio de cgroup, `poste` plafonne à 40 Gio par défaut.
- Le sous-module codeparsers de l'arbre de Lucie a de nouveau dérivé ; `docs/lucie-tmp/` non suivi : sa main.

## Ce soir, dans l'ordre

1. La CI au vert : le témoin du raccord physique (cœur et ED), puis le correctif des réclamations d'ED.
2. recherche : arret-du-backend ; archi : ide-par-le-flux ; tty : le lot docs entier ; recherche : la
   clôture du reliquat.
3. Les couches 2b-A (arbre principal) : relire, une page dans nos mots, l'avis de Lucie avant d'en jouer plus.
4. La pose du binaire et de la lib (G) ; lucivy 4.3.4 au mot de Lucie, puis `lucivy-core` dans rag3weaver.
5. Le ménage des `cx-` à son mot.

## 15 h 51 — le README est sur master, la lib neuve au magasin

- **archi a poussé d4d6263f8** : le README de rag3weaver refait (accroche, un exemple gardé par
  `scripts/test_readme_quickstart.sh`, vert sur un binaire de master et rouge sur l'installé 0b0ff221a,
  huit cartes à vraies images dont trois vues de l'IDE capturées par un test `#[ignore]` et cinq sorties
  de terminal réelles, `docs/images/README.md` pour la provenance, `AGENTS.md` à la racine). Deux vérités
  dites : les vues par worktree ne sont pas livrées ; PostgreSQL partiel. Verdict : lib entière rouge deux
  fois sur un seul test du raccord physique de Codex (vert seul deux fois, ticket à arbre principal
  `2026-11-13-un-test-du-raccord-physique-rougit-sous-la-lib-entiere.md`), poussé le rouge nommé ;
  e2e_hooks et navigateur non joués (réserve dite à archi). Ticket des 32 Gio ouvert avec la décision
  (32 Gio pour toute validation, 40 Gio reste le défaut de `poste` pour les bâtis ; la procédure le dira
  par G). archi reprend ide-par-le-flux.
- **Lib de la tête de master au magasin de luciepc** (G, 15 h 50) :
  `rag3db-build-207701bb23b21a7b63ff7083d28e0f715a4153e9-Release-p12-v11-g17-s18-rcOFF-stOFF-asanOFF-tsanOFF-ubsanOFF`,
  md5 df1b7270, ABI identique à 29a7bc52a (format seul). Le binaire de la tête en préparation ; pose des
  deux postes à mon mot, avec instantanés et le retrait de 7981 de l'env de l'hôte (7880 reste arrêté
  sur le mot de Lucie du 12, audit d'embarquement à l'appui). Les passes épinglent 207701bb2 dès la pose.
- Graines de Lucie complétées dans les visions : le classifieur de sécurité est pour l'utilisateur, local,
  après chaque réponse, références opaques seulement ; aussi sur un retour d'outil marqué sensible ou à
  la demande de l'agent ; et pour l'anonymisation.
- **15 h 52, une chose à savoir sur la passation de Codex** (vue par archi en reprenant ide-par-le-flux) : le
  11 au soir, à 19 h 56, une session Codex a rebasé et poussé sur master les commits de la branche
  `ide-par-le-flux` d'archi, dont un « wip » jamais joué (4d0c71396, 37e14f657, 3ff280c62), sous
  l'identité de Lucie, puis a effacé le worktree `wt-flux` et son target ; le fil de relecture qui
  ralentissait la lib a été remplacé ensuite par 7f7c3269b…499264286 (« fix(chat) », état du chat scellé
  à la fin du tour). Deux règles de la maison contredites : un lot inachevé d'une autre session ne se
  pousse pas, et un worktree ne s'efface pas avant que sa session ait dit ce qui y est unique. On ne
  refait pas l'histoire ; archi joue la passe entière sur master (lib 207701bb2, 32 Gio) avec la durée de
  la lib entière relevée, puis ferme son ticket des 32 Gio (procédure posée par G en 4b4233853) et corrige
  le README selon ED (la tâche et le worktree de l'atelier ne sont pas la même chose).
- **Pose en cours** (G) : passe e2e verte sous 207701bb2 (e2e_code 36/36, e2e_lectures 27/27), « pose le
  portable » donné à 15 h 5x : luciepc puis le portable, binaire 6a87986f0 (md5 bb983d4e), lib 207701bb2,
  instantanés avant-207701bb2, 7880 et 7981 retirés de l'env de l'hôte, puis l'exemple du README sur le
  binaire posé et les onze outils de l'atelier.
- **15 h 55, les deux postes sont sur la lib 207701bb2 et le binaire 6a87986f0** (G) : luciepc posé à
  15 h 51 min 53 s, le portable coupé de 15 h 53 min 31 s à 15 h 54 min 17 s, réveil vert, cinq backends
  rouverts, IDE sans erreur, env des hôtes sans 7880/7981, instantanés avant-207701bb2 avec RETOUR.sh.
  **L'exemple du README est vert sur le binaire installé** (`test_readme_quickstart.sh`, rc 0). L'atelier de
  Lucie expose **21 outils** : les 11 attendus et les 10 outils de lecture du code livrés depuis (callees,
  grep_files, impact, list_files, navigate_symbols, outline, schema, search_code, usages, wait_output),
  conformes au gabarit de master. Les passes épinglent 207701bb2, sans MOTEUR_ANCIEN. Disques : portable
  298 Gio libres, luciepc 1,3 Tio.
- **16 h 04, la CI rouge est un vrai défaut, les lots partent.** La mesure d'archi sur master (poste mesure,
  lib 207701bb2, 32 Gio) : lib entière en 40,08 s puis 41,07 s pour 2 307 tests (33 s pour 2 094 le 9) ;
  **la lenteur de 100 à 113 s n'existe plus** (le fil de relecture du journal retiré par les « fix(chat) »
  du 11). Le seul rouge, aux deux tours, est le test du raccord physique de Codex
  (`native_normal_catalog_open_refuses_owned_profile_before_initialization`), **rouge aussi sous la lib neuve
  et le poste seul** : la piste de l'environnement tombe, c'est un état partagé entre tests dans le même
  processus (ticket d3f899014). Lancé maintenant, sans attendre le soir, parce que master est rouge :
  **arbre principal** reprend le raccord physique (les deux tests rouges : celui-ci et la réouverture refusée
  après un Drop, tests.rs:414), **le cœur** cherche côté moteur l'écrivain qui reste vivant (registre des
  writers, libération au Drop), **ED** joue enfin son correctif des réclamations (9da8e4433, jamais compilé,
  le rouge intermittent d'une semaine). archi : le découpage au survol de la jauge (wt-pese, rouge joué).
  Toutes les passes sous 207701bb2 sans MOTEUR_ANCIEN et 32 Gio.
- **16 h 18, lucivy 4.3.4 publiée** (tag v4.3.4 sur main, sur le mot de Lucie dans la session lucivy ; run de
  publication vert au second essai, le premier annulé par GitHub faute de runner macOS en 15 min, rien
  publié à moitié) : crates.io (luciole, lucistore, ld-lucivy, lucivy-core, sparse-vector), npm (lucivy,
  cinq paquets de plateforme, lucivy-wasm), PyPI (5 wheels + sdist). Le correctif : la variante C de Codex
  (barrière d'admission atomique) relue et corrigée (clippy ; un réveil perdu dans l'arrêt de
  l'ordonnanceur, présent depuis mars), mesurée moins chère que la 4.3.3 sur le worker. La montée de
  `lucivy-core` dans rag3weaver revient à recherche après son lot de la CI. **La CI de lucivy est passée
  à Rust 1.100** (nouveaux lints clippy, nouveau rangement des dossiers de build de Cargo) ; recherche
  vient d'en trouver l'effet chez nous : la garde des targets cherchait les empreintes au vieil endroit
  sous Cargo 1.100 (faux vert), corrigée pour les deux dispositions (10caefbac), run de CI en cours.
- **Relecture de l'atelier par ED** : un défaut de sûreté (`canonique` garde un `..` après un dossier
  absent et le test de racine est lexical : une écriture hors de la racine sans accord), six défauts de
  correction ; sept tickets (b51a0287f) ; archi prend la sûreté en lot court devant la jauge. **Cœur** :
  rouge posé pour l'annulation sous pool plein (le témoin de Codex rougit sur master, son correctif
  0511dece7 repris en aab8ad0ba, suites en cours).
- **16 h 29, à lire par Lucie : les couches de Codex face aux vues** (ED, 3b4a59872,
  `docs/everything-declarative/13-11-2026/01-les-couches-de-codex-face-aux-vues.md`). En une phrase : deux
  briques de bibliothèque testées, qu'aucun chemin réel n'appelle. Converge avec la conception du 8 :
  la pyramide (la plus proche gagne, les tombstones masquent), les générations épinglées, la provenance
  par occurrence, le refus d'une vue ambiguë. Diverge : une couche est un nom libre plus un uuid, ni une
  branche ni un worktree ; l'identité n'est pas le contenu (ScopeVersion, lot 1a, non livré : l'économie
  d'embarquement n'existe pas) ; le raccord ne connaît qu'un index neuf en BM25 seul, l'index de code
  réel exclu par construction ; pas de couche de travail. Pour qu'une vue soit interrogeable : les cinq
  prérequis du 2b de Codex (chargeur durable, liaison occurrence → stockage, instantané composé, masque
  avant le top-k, paramètre `view` dans les outils), plus 1a/1b et la liaison branche/worktree → couche.
  **Question pour Lucie** : l'index de code réel adopte-t-il le raccord physique (aujourd'hui interdit), ou
  le 2b reste-t-il sur un index neuf ? Sans l'un ou l'autre, aucune de ses bases n'aura de vue.
- **16 h 49, la file et les causes.** File de push : recherche (la garde des targets reconnaît le cache de la CI
  et les deux dispositions de Cargo 1.98/1.100 ; run de preuve en cours, les deux étapes qui bloquaient
  passées), puis ED (l'installeur de l'atelier dit « suit le livré » ou « copie non suivie, voir M8 », refresh
  sous --sans-hote ; lib entière 2310/0), puis archi (le chemin hors racine refusé : résolution composant
  par composant, lib entière 2307 avec les deux rouges connus du raccord, e2e_hooks 2/2, navigateur
  40/40), puis 96 (les quatre défauts de M8, rejeu sur target neuf). **arbre principal** a la cause
  affinée du rouge du raccord physique : reproductible sous l'env d'archi (services d'embarquement et
  LUCIVY_SCHEDULER_THREADS=8), une instance précédente qui meurt tard et fait son point de reprise après
  la photo du test ; premier correctif insuffisant (le compte fort de l'Arc tombe à zéro avant le
  destructeur) ; correctif en cours par un Weak<ReopenState> relâché après le destructeur, témoin
  déterministe rouge. **Cœur** : rouge de l'annulation sous pool plein posé, correctif de Codex repris,
  suites en cours. La garde des targets a fait prendre un target neuf à trois sessions (targets d'avant
  sans provenance) : attendu, dit dans la procédure par recherche.
- **16 h 52, deux atterrissages** : recherche 62b5c5bfa (cinq commits : le témoin du cache, la provenance du
  cache et la reprise dans le workflow, le témoin sans couleur, le témoin des deux dispositions de cargo,
  la garde qui les connaît ; run de branche vert, lib 2259/0 en CI ; run de master 43456691914 en cours
  comme preuve finale) ; ED e140b4e9b (l'installeur de l'atelier dit le suivi, `suivi_d_une_pose`). archi a
  son tour (le chemin hors racine), puis 96 (M8), puis ED (la fin sans livrable de l'équipier, son lot
  suivant). recherche passe à arret-du-backend.
- **16 h 59, le correctif du rouge CI est prêt** (arbre principal, branche `arbre-principal/ci-raccord-physique`,
  d57273bba) : l'écrivain du raccord physique passe par OPEN_FOR_WRITING et REOPEN_WAIT ; le registre tient
  aussi un Weak<ReopenState> qui tombe après le retour de ~Database ; `wait_writer_released` avant chaque
  photo d'octets ; témoin déterministe vu rouge puis vert ; lib entière 2309/0 trois fois sous l'env qui
  rougissait. Le porteur nommé : le fil scheduler-3 de lucivy qui lâche en dernier un Arc<dyn BlobStore>
  après une fermeture du plein texte en échec — ce que la 4.3.4 de lucivy corrige côté close(). Ticket du
  raccord fermé ; celui de la CI garde l'état ouvert jusqu'au run vert ; un ticket neuf pour un rouge
  unique de model_source (profil_etranger, 1 sur 7) à embarquement. **File réordonnée pour la CI** : archi
  (en cours), puis arbre principal, puis 96 (M8, vert), puis ED (la fin sans livrable, avec le champ
  `expects` sur la request : report ou approval, déclaré par le superviseur).
- **17 h 22, la CI de master est verte** sur 62b5c5bfa (run 43456691914 : la garde a repris le target du cache,
  check_features 8/8, lib 2259/0 — les deux tests du raccord n'y ont pas rougi ce coup-ci, sans que ce soit
  une preuve ; le correctif d'arbre principal suit). recherche prépare arret-du-backend : le CSV d'un COPY tué
  devient un rouge connu avec son ticket (cause à part : écrire ces CSV dans le dossier de la base ou les
  balayer à l'ouverture, lot à lui), rejeu court, puis son tour après ED.
- **17 h 27, archi a poussé 5a82169c6** (le chemin hors racine refusé : `canonique` résout composant par
  composant ; ticket d'ED fermé ; lib entière avec les deux rouges nommés du raccord ; navigateur 40/40 au
  second jeu). Arbre principal a son tour (correctif du raccord physique, plus un vrai rouge de son lot
  corrigé en route : code mort sous rag3db-native,daemon ; lib entière 2309/0 avec et sans services).
  **Un motif qui revient** : sous l'étape navigateur entière (40 témoins), un témoin rougit par attente sous
  charge et passe vert seul — tour à tour `les_vues_de_l_ide_se_rendent_dans_un_vrai_navigateur` (archi)
  et `un_accord_exterieur_reprend_la_ligne_exacte…` (arbre principal, ticket pour ED). Règle appliquée :
  rouge d'environnement nommé, second jeu de l'étape, push ; mais si un troisième témoin fait pareil, c'est
  la charge de l'étape qu'il faut mesurer (un ticket d'ED sur l'étape, pas sur chaque témoin). archi passe
  à l'écriture sûre (mode du fichier gardé, refus d'un fichier changé depuis sa lecture).
- **17 h 33, le correctif du raccord physique est sur master** (arbre principal, 73586bf92) : l'écrivain du
  raccord passe par le registre OPEN_FOR_WRITING et REOPEN_WAIT, la boucle d'ouverture compte la base et
  l'état de réouverture et attend la vraie fin du destructeur ; rejeu court net (lib entière 2312/0,
  physical_index 11/11 trois fois, navigateur 40/40). Ticket du raccord fermé ; le ticket CI se ferme au
  premier run vert de 73586bf92 (arbre principal). Suite d'arbre principal : wt-deplace (le garde-fou), son
  inachevé du 11 ; les couches 2b-A attendent la décision de Lucie. 96 a son tour (M8), puis ED (l'équipier).
- **17 h 35, la file du soir et deux décisions à relire par Lucie.** File : 96 (M8, en tour), ED (l'équipier :
  reste non vide, `expects`), le cœur (l'annulation sous pool plein : correctif de Codex repris, ticket
  fermé, ticket neuf de la saturation du pool sous vingt écrivains, DdlEveryMinute et les deux seuils de
  VectorIndexDeleteCost en known_red avec leurs tickets), archi (l'écriture sûre : mode gardé, écriture
  atomique avec relecture avant renommage, `expected_hash` dans le worktree, et le même défaut corrigé
  dans le backend de code), embarquement (le test d'activation). **Décisions de c7** : embarquement adopte
  le port 0 annoncé par le fournisseur pour l'activation gérée, comme le démon d'embarquement (une course
  de port plausible dans la fixture) — à relire par Lucie, ça touche l'activation ; arbre principal prend le
  **lot 1a des vues, la version de scope par contenu**, page de contrat d'abord (relue par ED et recherche),
  indépendant de la question de l'adoption du raccord ; wt-deplace était déjà atterri par Codex (1904e04e4).
- **17 h 41, le contrat de la version de scope (lot 1a) est écrit** (arbre principal, 8b579d2d4,
  `docs/arbre-principal/13-11-2026/29-contrat-de-la-version-de-scope.md`) : le mécanisme générique existe
  depuis le 12 (identité par contenu, projection, masque) et la forme pour le code dans un e2e privé ; le
  lot 1a l'active dans le schéma de production. Identité = format, cellule, champs rendus octet pour octet
  (name, signature, content, docstring, scope_type, language), sans position ni modèle ; une seule
  référence ScopeContent_DERIVED_FROM avec l'état de rendu par occurrence ; morceaux et vecteurs partagés
  par version, modèle, cellule ; le creux à témoigner. **Décision de c7, à relire par Lucie** : (A) le
  schéma suit la base — une base neuve reçoit les versions, une base existante ouvre comme avant et se
  dit « à migrer (1b) » dans /etat et l'IDE ; aucune base de Lucie ne change avant 1b. parent_name hors
  de la version (c'est de la position), ED tranche. Le cœur : ses deux seuils mesurés disent un vrai
  défaut (une suppression sous un HNSW rempli par SET, non checkpointé, coûte avec la taille : 1,97 → 8,01 ms
  de 1 500 à 15 000 lignes ; checkpointé 1,96 → 2,39), ticket posé, known_red. Embarquement : port 0
  annoncé codé, passe lancée.
- **17 h 41, 96 a poussé c89e3f769** (M8 : le plan de gabarits refuse un Plan `@builtin:` lu hors du contenu
  fourni, un changement de `name`, un script Rhai ou un gabarit jinja changé seul, et dit les changements
  de `search_graphs`/`graph_tool` ; l'identité des commits de gabarits corrigée dans la page : neutre). ED a
  son tour (l'équipier), puis le cœur, archi, embarquement. 96 reprend l'archivage à blanc des docs sur une
  copie de la mémoire de Lucie, pour une liste à approuver par elle.
- **17 h 52, deux atterrissages de plus.** ED 327c3bba4 (l'équipier : un step_done dont le reste n'est pas vide
  relance le pas suivant, plan ou pas, borné à 40 pas ; `expects: report|approval` sur la request, par
  l'IDE, la boîte de Codex et `mail send` ; sans le livrable, l'équipier attend la boîte sans appeler le
  modèle ; e2e_equipier sur Gemini vert ; un rouge de 15 min dit et compris : un reste non vide qui ne
  disait rien). Le cœur b5c44313e (l'annulation sous pool plein : les clés annulées retirées de l'index
  local par leur plage de lignes au lieu de relire les clés ; ticket fermé ; ticket neuf de la saturation
  du pool sous vingt écrivains ; DdlEveryMinute et les deux seuils VectorIndexDeleteCost en known_red avec
  leur ticket — la suppression sous un HNSW rempli par SET non checkpointé coûte avec sa taille ; C++ seul,
  ABI vue par l'hôte inchangée, format 43). **G bâtit la lib de b5c44313e** ; d'ici là les passes gardent
  207701bb2 avec MOTEUR_ANCIEN=1. archi a son tour (l'écriture sûre), puis embarquement (le port 0). Trois
  pushes de docs de ma main ont fait perdre des courses au tour de 96 : je garde mes commits de rapport
  pour les creux entre deux pushes de code.
- **18 h 10, archi a poussé a9163c7c8** (l'écriture sûre : le mode du fichier gardé, l'écriture refusée sur
  un fichier changé depuis sa lecture, côté racine par `ecrire_atomique` et côté worktree par
  `expected_hash` puis `FileSource::write` ; rejeu sous 835d5298f, lib entière 2326/0, navigateur 40/40,
  rien de nommé). Suite d'archi : « dire vrai » (réindexation en échec dite en échec, relance dite refusée).
  **Lib 835d5298f** (G, 17 h 54 : annulation sous pool plein, ABI exportée identique, vtable interne
  changée, lib et vector du même bâti) au magasin des deux postes ; binaire cb21e3cbb essayé vert aux deux
  postes, passe e2e verte ; pose en une coupure après embarquement (le port 0 annoncé, en tour). recherche
  attend une place lourde pour la passe entière du journal des runs (e2e_juge ne laisse plus de résidu) ;
  arbre principal a trié ses douze rouges de 1a (neuf lectures en dur → une aide commune des tests ; la
  relation interne ScopeContent_DERIVED_FROM masquée des outils de graphe ; un vrai rouge à comprendre :
  deux fichiers qui partagent une version, l'un retiré, l'appel d'un tiers ne gagne pas son arête).
- **18 h 26, la CI est verte et son ticket fermé** (arbre principal, f61156910, run 43466739505 sur 327c3bba4 qui
  contient le correctif). **Le vrai rouge de 1a est compris, pas un trou de conception** : une garde du
  mécanisme générique de Codex (`check_fts_counts`, `comptes_du_plein_texte`) comparait les documents du
  plein texte porté par ScopeContent (par version) aux lignes de Scope (par occurrence), concluait à une
  écriture perdue et rebâtissait l'index à chaque ouverture, ressuscitant l'occurrence du fichier retiré ;
  correctif : compter les lignes de la table qui porte l'index ; témoin direct en rouge. recherche : le lot
  du journal des runs vert sous 835d5298f à un rouge connu près (le raccord, dont le correctif est sur master
  depuis sa base) ; « à toi » après embarquement, rejeu court qui doit le voir vert. archi : « dire vrai »
  en passe (réindexation en échec dite en échec ; une décision prise pendant le tour relance après ; le
  journal des décisions avant l'accord ; un test de Codex qui qualifiait l'ancien contrat passé au nouveau).
- **18 h 29, décision de Lucie sur les couches** : **les vues se bâtissent sur l'index de code réel**, en lui
  donnant ce qu'il faut (les versions par contenu, les couches, les générations), lot par lot : 1a (la version
  de scope par contenu, en cours), 1b (la migration des bases existantes, à son mot), puis la liaison
  branche/worktree → couche, le chargeur durable, le masque avant le top-k dans les signaux réels, le
  paramètre `view` dans les outils. Pas de second index : le raccord physique de Codex (index neuf en BM25
  seul, possédé) reste un banc d'essai de ces mécanismes (générations de publication, reprise), pas un
  stockage à maintenir ; ce qui en vaut la peine s'adopte dans l'index réel. « Codex a pu se tromper, partir
  dans une mauvaise direction. » Mes mots du 13 au soir, tranchés par elle.
- **18 h 38, la fenêtre KDE de Lucie** (« créer un portefeuille kdewallet », toutes les cinq minutes) : ce n'est
  pas Chromium mais **`gh`**, dont les jetons des deux comptes sont « (keyring) », servi sous KDE 6 par
  kwalletd6 ; chaque `gh` (dont `poste push` à chaque push, les `gh auth switch` des sessions Sairen, mes
  lectures de la CI) demande un portefeuille qui n'existe pas. Vu par `busctl --user monitor` sur
  `org.freedesktop.Secret.Service`. Remède à sa main : un portefeuille vide, ou `gh auth login
  --insecure-storage` par compte. archi pousse quand même `--password-store=basic` pour Chromium
  (inoffensif). **1a** (arbre principal) : second tour vert partout sauf deux vrais rouges de reprise après
  incident d'une base à versions (les morceaux d'une version non refaits à la relance ; un repreneur qui
  sérialise mal sur une version émise dans un paquet défait) — ils restent dans 1a ; sondes, bancs et
  mesures en 1a-2 avec ticket. File : embarquement (en tour), recherche (journal des runs, vert), archi
  (« dire vrai », vert), puis la pose de 835d5298f en une coupure.
- **19 h 15, la soirée.** archi a poussé 7a50c824a (Chromium avec `--password-store=basic` et `--use-mock-keychain`,
  inoffensif ; les journaux de passe archivés sous `~/.cache/rag3weaver-build/archives/`) et a trouvé **la
  vraie cause des rouges navigateur « status jamais fini »** : une bulle fantôme dans Chat.tsx (un RunStarted
  tardif pris pour un tour venu d'ailleurs, depuis son lot de la carte d'accord be319fa75) ; correctif en
  cours, rouge d'abord ; ED en porte la cause dans sa mesure. embarquement : ticket d'activation fermé (dix
  tours verts, 0977892ce), un rouge rare du raccord relevé pour arbre principal (reprise après mort du
  processus, fixture gardée), et la page « activation et résidence : livré et manquant » (cda4a028e) ;
  **décisions de c7, à relire par Lucie** : les baux entre processus d'abord ; les fournisseurs à luciepc
  sont les unités systemd existantes pilotées par l'activation ; l'éviction ne touche jamais les trois
  services résidents de Lucie, seulement un fournisseur lancé par l'activation. recherche : sa passe verte
  a perdu la course contre le lot Chromium ; rejeu sur 0977892ce (journal-apres-arret-4), **fenêtre tenue**
  jusqu'à son verdict (~19 h 40) ; puis archi (bulle fantôme, dire vrai), puis la pose de 835d5298f.
- **19 h 24, recherche a poussé 93cc95a43** (le journal des runs : l'arrêt et le Drop de l'hôte écrivent les
  journaux en attente de leur grâce puis plus rien ; l'arrêt de l'hôte arrête aussi le juge des bilans ;
  e2e_juge sort des résidus ; ticket du 9 fermé ; lib entière 2330/2330 sans le rouge du raccord, navigateur
  40/40). Elle attaque la montée de lucivy-core 4.3.4. archi a son tour (la bulle fantôme et « dire vrai »),
  puis la pose. ED : la mesure de l'étape navigateur (a3fd235c8) dit que la charge n'est pas la cause ; deux
  causes réelles, la bulle fantôme (archi) et **un premier search_code ou impact par backend à 59 s, les
  suivants à 250 ms** (118 des 132 s de l'étape) — ED pose une sonde de la lib (chaque appel sous 5 s) et
  cherche où vont les 60 s ; c'est aussi la première question lente de l'atelier de Lucie à chaque
  réouverture.
