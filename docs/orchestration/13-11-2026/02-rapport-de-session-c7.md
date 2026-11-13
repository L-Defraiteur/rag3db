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
