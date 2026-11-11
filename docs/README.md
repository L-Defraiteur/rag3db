# docs/ — comment s'y retrouver, et comment reprendre

Ce dossier est fait pour qu'un agent de code (Claude, Codex, autre) reprenne le travail sans avoir
vécu nos échanges. Depuis le 11 novembre 2026, il est organisé **par session de travail** : chaque
session (un périmètre du projet, tenu par un agent) a son dossier, avec ses rapports datés, son
knowledge dump et ses tickets ouverts.

## Par où commencer

1. `docs/orchestration/11-11-2026/02-knowledge-dump.md` : les règles du jeu (dépôts, postes, lib
   épinglée, passes, push, verrous, identité git, ce qu'on ne fait jamais).
2. `docs/orchestration/11-11-2026/01-rapport-de-session.md` : l'état du projet ce jour-là, session par
   session, et la file de ce qui reste.
3. Le dossier de la session dont on reprend le périmètre : `docs/<session>/11-11-2026/01-rapport-de-session.md`
   (état, **section « Reprendre »** : chaque worktree inachevé avec son chemin, sa branche, son dernier
   commit et les commandes pour continuer), puis `02-knowledge-dump.md` (tout ce qu'elle sait).
4. `docs/nouveautes.md` : ce qui est atteint, une ligne par étape, du plus récent au plus ancien.
5. `docs/procedures/push-d-un-lot.md` : la procédure de passe et de push, ligne par ligne.

## Les sessions et leurs dossiers

| dossier | périmètre |
|---|---|
| `docs/orchestration/` | l'orchestration des sessions : la file des pushes, les libs posées, les visions (`visions/`), les nouveautés (`news/`) |
| `docs/coeur-cpp/` | le moteur C++ rag3db (fork de Kuzu) : stockage, point de reprise, transactions, suites C++ |
| `docs/coeur-cpp-hotfixs-tickets/` | les hotfixs du moteur, le rejeu de traces exactes, l'outillage des tickets |
| `docs/everything-declarative/` | rag3weaver déclaratif : manifestes, gabarits, hôte et backends, atelier, équipier par pas, juge |
| `docs/rag3weaver-archi/` | l'IDE React, l'atelier (outils, cartes d'accord), les hooks Claude Code |
| `docs/recherche/` | la recherche (dense, creuse, plein texte, lucivy), le bus, l'arrêt des backends, run_e2e |
| `docs/memoire/` | la mémoire longue : fiches, sujets, archiviste, banc des fiches, classeur local, M8 |
| `docs/embarquement/` | les embedders et le service d'embarquement, le régime GPU, le rattrapage |
| `docs/codeparsers/` | le sous-module codeparsers : grammaires, IR, résolution des arêtes, bancs A/B |
| `docs/optimiseur/` | les deux postes : libs au magasin, binaires posés, services, tunnels, disque de luciepc |
| `docs/banc/` | les bancs de modèles locaux (juge local, décideur d'actions) |
| `docs/arbre-principal/` | l'arbre de travail de Lucie et ses worktrees |
| `docs/lucivy/` | lucivy, le moteur de plein texte publié (crates.io, npm, PyPI) |
| `docs/tty/` | l'agent de code en interface terminal, le pont entre agents de code |

Les dossiers datés (`docs/<date>-…/` et `extension/rag3weaver/docs/<date>-…/`) sont les archives des
sessions d'avant le 11 novembre : on y lit l'histoire d'un chantier, pas son état.

## Les tickets

- Un ticket ouvert vit dans `docs/<session>/tickets/` (la session qui le tient) ou, s'il n'a pas de
  session, dans `docs/tickets/`. Un ticket fermé va dans `docs/tickets/closed/` (`git mv`).
- **Fermer un ticket de session** : `git mv docs/<session>/tickets/<nom>.md docs/tickets/closed/<nom>.md`,
  et dans le même commit son **État** réécrit avec un premier mot qui dit qu'il est clos (corrigé,
  fermé, fait, résolu, abandonné, non reproduit, accepté, dissous, remplacé), suivi du hash.
  **Déplacer** un ticket ouvert d'une session à une autre, c'est un `git mv` entre leurs dossiers
  `tickets/`. Un même nom ne vit qu'à un endroit : le script refuse un ticket présent à deux places,
  un ticket clos hors de `closed/`, et un ticket de `closed/` dont l'État dit « ouvert ».
- `python3 extension/rag3weaver/scripts/check_tickets.py` contrôle les en-têtes (VERT ou ROUGE avec
  la liste des refus) ; `--index` écrit `docs/tickets/INDEX.md` (non suivi par git), avec le dossier de
  chaque ticket.
- `--lot origin/master` liste, avant un push, ce que le lot fait aux tickets depuis sa fourche avec
  origin/master (`git merge-base HEAD origin/master`, pas la pointe : ce que d'autres lots ont poussé
  entre-temps n'y paraît pas). Une ligne par ticket :
  - `ouvert   <session>/tickets/<nom>.md` ;
  - `fermé    closed/<nom>.md` ;
  - `rouvert  <nom>.md` ;
  - `déplacé  <session>/tickets/<nom>.md (depuis <nom>.md)` ;
  - `état     <nom>.md : « ancien » → « nouveau »` ;
  - `retiré   <nom>.md`.

  Un ticket de `docs/tickets/` s'écrit `<nom>.md` ou `closed/<nom>.md`, celui d'une session
  `<session>/tickets/<nom>.md`. Un déplacement, même réécrit au point que git ne voit plus de
  renommage, se lit « déplacé » ou « fermé », jamais comme un ouvert et un retiré.
- Un défaut connu non corrigé est un ticket, jamais une ligne de message.

## Les règles qui ne se discutent pas

- Identité git : `Lucie Defraiteur <luciedefraiteur@gmail.com>` (`git -c user.email=… -c user.name=…`),
  jamais l'adresse professionnelle ; aucune ligne d'attribution ni mention d'un modèle dans les
  commits, les PR et les docs.
- Master avance seulement en avance rapide, sur un verdict VERT lu ; jamais de push en force, jamais
  d'amend d'un commit poussé ; après un rebase, un nom de branche neuf ; chaque commit poussé sur sa
  branche dans la minute. Un push de code passe par la passe et `poste push` ; les docs seules se
  poussent sans verrou ni passe.
- Rien n'est proposé aux amonts (Kuzu, Vela, Ladybug). Un tag `v*` de lucivy publie partout : jamais
  sans le mot de Lucie.
- `.vault/` ne s'affiche ni ne se copie ; le `.env` ne se lit pas.
- On ne tue que ses propres processus ; `/tmp` est de la mémoire vive ; un poste prend au plus
  `-j8` ; les verrous `poste lourd`, `poste mesure`, `poste push` (`extension/rag3weaver/scripts/poste`).
