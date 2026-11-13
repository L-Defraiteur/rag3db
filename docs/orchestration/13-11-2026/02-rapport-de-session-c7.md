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
