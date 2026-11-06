# Plan du week-end des 8 et 9 novembre 2026 : la mémoire de code pour agents

Lucie, 6 novembre : « ce week-end on se focalise principalement là-dessus : le
projet de mémoire de code pour agents, les hooks à Claude d'abord aussi si
possible, sur compaction ou pourcentage de contexte atteint, sur nouveau tour
pour suggérer des choses ; implémentons l'écosystème agentique de code git de
demain ; on laisse de côté pour l'instant les autres sujets. »

Les visions du 6 novembre disent le sens : `visions/2026-11-06-15h46-le-depot-
qui-sait-pourquoi.md` et `visions/2026-11-06-15h46-un-index-par-depot-des-vues-
par-worktree.md`. Cette page dit l'ordre.

## 0. Atterrir ce que la nuit du 26 octobre a laissé vert (samedi matin)

Dix branches poussées, vertes, jamais fusionnées (tout s'est arrêté à 0 h 45) :

| branche | tête | session | contenu |
|---|---|---|---|
| lot-96-m4-4 | e2ae6a9c8 | mémoire | M4 pas 2, SplitSections |
| surlignage-creux-2 | 21d65ef5a | recherche | surlignage des morceaux du dense seul par le creux de bge-m3 |
| codeparsers/includes-2 | beb3a392c | codeparsers | INCLUDES en forme exacte, page 11, borne des déclarants |
| ide-react-vue5-2 | e8a0cf957 | everything declarative | tâches et relecture, l'équipier |
| ide-react-vue6 | 1ecf6ef17 | everything declarative | bilans, fiche par outil |
| ide-react-vue7 | be988da8c | everything declarative | arbre, symbole par sections, graphe, DAG vivant, réactions, propositions |
| ide-react-vue3b | e0678a797 | everything declarative | l'édition (CodeMirror) ; l'ancienne page n'a plus de vue |
| projection-lecture-seule-2 | 875f149b5 | cœur | PROJECT_GRAPH_* passent sous READ ONLY (le dense filtré revient sous l'instantané) |
| v22-retrait-2 | da8104fd5 | cœur | le retrait des générations à chaque point de reprise |
| csr-listes-temoin | b44ebd0bb | hotfixs | témoin (B), vert sur master ; à jouer sur le moteur du 10-12 oct. |

Ordre : chaque session rouvre son worktree, rebase sur master, rejoue sa passe
(docs/procedures/push-d-un-lot.md ; RAG3DB_LIB épinglée 04063ee06 et
MOTEUR_ANCIEN=1 tant que la lib unique n'est pas là), pousse sous if VERT, un
à la fois dans cet ordre : cœur (projection, retrait) → mémoire → recherche →
codeparsers → everything declarative (5, 6, 7, 3b). Puis la **bascule 43** par
docs/procedures/bascule-de-lib.md sur la lib unique bâtie depuis master
(V2-1c + V2-2a+2b + (A) format 43 + projection), instantanés, courant, hôtes
de Lucie (G). Puis le relevé des 18 tables de la base de code (hotfixs) après
ses jours de montre. Rien de nouveau n'entre avant que ce soit posé.

## 1. Les hooks de Claude Code (samedi) : la mémoire qui parle à la session

Le pont (vision du 11 octobre) par les hooks de Claude Code, qui appellent
l'hôte par le MCP ou par un appel direct. Page de conception courte d'abord
(mémoire + everything declarative), puis le code ; identifiants en anglais.

- **PreCompact** : le défi d'identité (page 04 du 25 octobre) : la session
  écrit son apport `identity` (who, runtime, can, doing, holding, mandates,
  peers) et son plan courant sur sa fiche de session, range sa transcription
  (M9), et reçoit en retour ce qu'elle doit garder en tête. C'est le moment où
  la mémoire remplace le contexte perdu.
- **UserPromptSubmit** (nouveau tour) : le hook rappelle la mémoire sur le
  prompt (recall_fiches), lit les plans actifs et en pause de la session, les
  apports `diff` récents sur les fichiers que la session a touchés, le dernier
  verdict du juge sur cette tâche ; il rend un court bloc de contexte (« tu
  avais un plan en pause sur… ; ce fichier a changé depuis par… ; le juge a
  noté que l'outil X t'a trompé ») que Claude Code injecte au tour. Le
  pourcentage de contexte n'est pas un événement : le hook l'estime par la
  taille de la transcription (transcript_path) et, au-delà d'un seuil, suggère
  de ranger maintenant (un apport, un plan clos) plutôt que d'attendre la
  compaction.
- **SessionStart** : whois (la clé stable, les alias) et la reprise : la fiche
  de session de la dernière tâche de cette session, ses plans en pause.
- **Stop** (fin de tour) : la fin de run journalisée, pour que le juge puisse
  juger un tour de Claude Code comme un tour du chat (le journal des runs est
  générique : une réaction y écrit).
- Les hooks sont déclarés dans `~/.claude/settings.json` par un script du
  dépôt (`scripts/hooks_claude_code.sh`, idempotent, qui dit ce qu'il pose),
  et chaque hook est un petit binaire ou script qui parle à l'hôte 7970 ;
  témoins : un faux hôte, les quatre événements, le bloc rendu. Sans hôte, le
  hook se tait et le dit dans un journal ; il ne bloque jamais la session.

## 2. Un index par dépôt, des vues par worktree (dimanche, marches 1 et 2)

- Marche 1 : la version comme entité, au grain du scope (empreinte de contenu,
  liste ordonnée par fichier), morceaux et vecteurs rattachés ; migration du
  format du gabarit de code ; témoin : deux fichiers identiques partagent
  leurs vecteurs. Sessions : recherche (index) + everything declarative
  (gabarit, tâches), l'arbre principal pour L6.
- Marche 2 : la vue (chemin → version), le paramètre de vue sur les outils de
  lecture, la vue par défaut d'un backend ; témoin : la recherche filtrée par
  vue rend la version de la vue.
- Les marches 3 à 5 (la tâche sans copie de base, la fermeture, le relevé
  ancré) suivent la semaine d'après.

## 3. Les changements en fiches et `find_tasks` (dimanche, si la marche 1 tient)

- Un apport `diff` par version de scope qui diffère dans une vue, sur les
  fiches du fichier, du symbole et de la session ; « ce qui change dans cette
  PR » se lit dans la mémoire.
- `find_tasks { author, about, touches }` en Plan déclaré ; la même question
  depuis la fiche d'un symbole dans l'IDE.

## Ce qu'on laisse de côté

V2-2c (format 44) et le plafond L6, le reste des mesures du banc, la pente,
lucivy (tickets apostrophe et clés ignorées à poser par Lucie), les rejugements
des serveurs de codeparsers, l'embarquement, la voix, les roadmaps en fiches
(la page de 96 attend ses trois réponses), la réconciliation des sujets
partagés. Rien n'est perdu : tout est sur master ou sur une branche poussée.

## Les questions qui attendent Lucie

Le dossier `~/memoire-longue/` pris par défaut ; les trois questions de la page
roadmaps et les trois de la page identité (96) ; les tickets lucivy ;
rag3db-service de luciepc à effacer ; le llama-server du portable quand on a
besoin des passes ici.
