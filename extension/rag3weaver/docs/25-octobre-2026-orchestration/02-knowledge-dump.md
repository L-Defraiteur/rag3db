# Orchestration — ce que je sais du projet (de mémoire, 25 octobre 2026)

*Écrit de mémoire par la session d'orchestration, pour celle qui reprendra.
Vérifier dans le code et les docs avant d'agir sur un détail.*

## Les objets

- **rag3db** : un fork C++ de Kuzu (jamais rien vers l'amont : Kuzu, Vela,
  Ladybug). Multi-écrivains par défaut avec verrous nommés ; format de
  stockage versionné depuis ce soir (41 : version posée à la main, bâti
  écrivain dans l'en-tête, FormatRecord au journal ; 42 : numéros de colonnes
  stables avec trous). Le point de reprise se refait en « voie 2 » : segment
  neuf sur pages neuves (V2-1b), porte à deux niveaux et bloc échangé sous
  les lecteurs (V2-1c, en passe), puis les écrivains (V2-2). Les requêtes
  préparées portent leur préparation (plus de magasin). Les index HNSW ont
  un rattrapage à la validation ; le CREATE concurrent (A) arrive avec un
  format 43.
- **rag3weaver** (`extension/rag3weaver`) : l'orchestrateur Rust, déclaratif
  : des gabarits (templates/backends/<genre>, outils en .mmd, réactions,
  composants React déclarés, Plans JSON dans plans/ lus par le PlanNode via
  l'IR du Plan, rendus en Cypher et en SQL PostgreSQL). Un hôte systemd par
  socket (7970 ici, à luciepc aussi), jeton stable, repos après 30 min ;
  bus d'événements (agent, dataflow, reactions, catalog, tokens) ; lectures
  sans attente (L4-L6, un instantané par appel) ; un journal des runs
  (ce soir). L'IDE (`/ide/entrer?jeton=…`) : arbre, fiches de fichiers avec
  outline, symboles, recherche (scores par signal, replis, reclassement à la
  demande), graphe, DAG vivant, propositions, tâches en worktree, chat de
  l'atelier (propose des déclarations ; code dans une tâche sur un dépôt
  étranger), vue des réactions, arrêt de tout run. Les pages sont du JS en
  fichiers servis depuis ce soir.
- **Mémoire longue** (gabarit `memory`, MCP memoire-longue chez Lucie) : la
  fiche est le sujet ; un apport (L1) par contribution, L2 recalculé (par
  l'archiviste, à venir), L1 bis ; Source, Ref, genres ; fiches en fichiers
  Markdown avec en-tête YAML, bloc machine, `format: N`, `manifeste.yaml`
  à la racine ; base reconstructible depuis le dossier à parité ;
  cited_by par Plan déclaré, trié par le dernier apport. La base de Lucie
  n'a que deux lignes ; sa migration vers le gabarit câblé attend le dossier.
- **codeparsers** (sous-module) : tree-sitter (0.27 depuis ce soir), douze
  langages ; la résolution « ne devine pas » (plus de fichier/dossier/nom
  seul) ; garde-fou des langages ; INCLUDES (visibilité par les #include,
  puis par la déclaration) en mesure ; un oracle LSP (clangd,
  rust-analyzer, tsserver, pyright) pour juger les arêtes, dont le juge est
  en correction.
- **lucivy** : le plein texte de Lucie (crates.io ; son dépôt
  L-Defraiteur/lucivy, publie par tag v* via CI). rag3db épingle la branche
  4.3.1 (a925d94) par [patch.crates-io] temporaire ; défauts corrigés :
  figement du commit, shard sans écrivain, assertion BM25 sous commits
  concurrents, drop qui perdait des écritures.
- **Magasin de libs** : ~/.cache/rag3db-libs/<build>/ + `courant` ; binaires
  liés par RUNPATH ; planchers (4f9274a38, a45a15adf, dcbaef7b1) vérifiés
  par rag3db_libs.py ; courant = 04063ee06 aux deux postes.
- **Embarquement** : services sur luciepc 7878 (granite-278m, 768), 7879
  (bge-m3 1024 + reranker + OCR), 7880 (granite-107m, 384) ; tunnels d'ici
  7979-7981 par l'unité rag3weaver-tunnels.service ; unités systemd pour
  les services à poser dans la fenêtre de 21 h 30 ; la carte est pleine
  avec les trois.
- **Gemini** 3.8 Flash par Vertex (.vault, projet lr-hub-472010) : le modèle
  du produit ; les locaux pour déboguer. Budget de raisonnement déclarable
  par modèle (models.llm.reasoning), jetons de raisonnement et cache comptés.
- **alpha.3** (paquet npm) : archives sur les trois plateformes, rien publié
  sans Lucie (OTP npm pour rag3db ; lucivy par tag).

## Les sessions (noms du 25 octobre)

orchestration (c7, moi) ; arbre principal (l'IR, code.rs, la lib du magasin,
les instantanés) ; optimiseur = G (hôtes, lucivy, alpha.3, CI) ; mémoire ;
codeparsers ; everything declarative (IDE, chat, juge) ; coeur c++ (format,
point de reprise) ; coeur c++/hotfixs/tickets (HNSW, requêtes préparées,
CREATE concurrent) ; recherche ; banc ; embarquement ; lucivy (la session de
Lucie pour son dépôt). Les docs de chaque session sont dans
`extension/rag3weaver/docs/<jour>-<session>/` ; les tickets dans
`docs/tickets/` (ouverts) et `closed/`.

## Les postes

- Ici (lucie-tablette, ROG Flow) : l'arbre de Lucie
  (~/git_workspaces/rag3db, master en avance rapide, lecture seule pour
  les sessions de code, docs d'orchestration directement dedans) ; un
  lourd à la fois ; la mesure 7e/9e y attend minuit.
- luciepc (ssh, fish, dédié à nos projets, gh L-Defraiteur, remote ssh) :
  tout le lourd, plusieurs compilations à la fois, pousser depuis là-bas ;
  les worktrees frères ; PostgreSQL de test sur 5433 (celui d'ici est
  arrêté) ; zram ; `poste lourd|mesure|push`.

## Les méthodes qui tiennent

Témoin rouge d'abord, chiffres des deux passes ; vert = aucun rouge ; un
rouge imprévu se reproduit sans harnais avant d'accuser le moteur ; une
règle de résolution se mesure sur code-lourd avant master ; un seuil ne se
relâche pas sans preuve ; parité au bit près ; les docs disent ce qui est
tranché et pourquoi ; un défaut non corrigé = un ticket, un filet cite son
ticket ; la forme la plus générale avec sa preuve, la complexité n'est
jamais un argument ; s'inspirer de PostgreSQL et Neo4j ; identifiants en
anglais, le reste en français ; jamais l'adresse sairen dans les dépôts ;
aucune attribution IA dans les commits ; jamais de push en force ; un refus
de permission ne se contourne pas.

## Ce qui attend Lucie

Le dossier de ses fiches ; le tag lucivy 4.3.1 ; le défaut du reranker ; la
liste blanche des réactions pour les agents ; le chat qui ouvre lui-même une
tâche ; le grain du point de reprise (segment) après V2-1c ; la décision
sur la ligne de nouveautés du jour qui manque (V2-1c, le juge).
