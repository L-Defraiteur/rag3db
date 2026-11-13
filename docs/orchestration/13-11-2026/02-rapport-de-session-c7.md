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
