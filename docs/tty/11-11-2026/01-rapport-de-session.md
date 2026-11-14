# Rapport de la session « tty » : l'agent de code en interface terminal

État au 11 novembre 2026. La session a travaillé du 7 au 8 novembre 2026. Elle s'appelait « tty » ; c'était une session Claude Code ouverte par Lucie. Ce rapport est écrit pour un agent qui n'a aucune mémoire de nos échanges.

## En deux phrases

Rag3weaver a maintenant un **terminal pour son agent de code**. C'est un programme en ligne de commande, `rag3weaver-tty`, qui suit au fil de l'eau ce que fait l'agent, même à travers ssh, avec une animation « démoniaque » quand l'agent réfléchit et le code coloré. Tout ce qui suit est sur la branche `master` du dépôt `rag3db` (GitHub `L-Defraiteur/rag3db`) ; rien n'est resté en branche.

Les mots utilisés :
- **L'hôte** : le serveur HTTP local de rag3weaver (`127.0.0.1:7970`), qui tient les bases et fait tourner les agents.
- **L'équipier** : l'agent de code autonome que l'hôte lance pour une tâche.
- **Une tâche** : un travail confié à l'équipier, dans son propre worktree git (une copie de travail sur une branche à elle).
- **Un lot** : un ensemble de commits poussé d'un coup sur `master` après ses tests.

## Ce qui est sur master (avec les hashes)

| Lot | Ce qu'il apporte | Commit |
|---|---|---|
| tty-1 | le client `rag3weaver-tty --plain`. Il lit le flux d'événements de l'hôte (SSE, `GET /ide/flux`), lance une demande à l'équipier et reprend sans perte ni doublon après une coupure (`Last-Event-ID`). Côté hôte, `describe()` dit maintenant pour chaque outil s'il lit, écrit ou exécute (`policy`). | `cfc5cc8e8` |
| tty-2 | la vue « inline » en ratatui et crossterm. Le texte coule dans l'historique normal du terminal, et seules trois lignes se redessinent en bas. Mesure : 28 octets par token, borne à 35. | `1fc1e24bd` |
| tty-3 | le **Daemon Code Summoning**, les trois animations reprises de l'ancienne page HTML de Lucie (Cercle, Transmutation, Glitch). Avec : les couleurs dégradées selon le terminal ; les options `--depot` et `--tache` ; l'en-tête « dépôt · branche · tâche » ; le flux filtré sur le worktree de la tâche. | `f66e72d31` |
| thème commun | le thème « daemon » déplacé dans `extension/rag3weaver/templates/themes/daemon.json`, fichier lu aussi par l'IDE. | `f2e6e2521` |
| coloration | les blocs de code colorés par l'hôte (`POST /highlight`, grammaires tree-sitter de codeparsers) dans la palette `code` du thème. | `3696a71cb` |
| l'IDE (lot d'archi, pas le mien) | l'IDE React lit le même `daemon.json`, servi en `GET /ide/theme/daemon.json`. | `7dac6418b` |

Les docs sont dans `extension/rag3weaver/docs/07-novembre-2026-tty/` :

| Fichier | Contenu |
|---|---|
| `01-les-anciens-tty.md` | l'enquête sur les vieux projets |
| `02-conception.md` | la conception : choix de la bibliothèque, flux de l'hôte, états, mesures, vocabulaire, coloration |
| `03-rapport-de-session.md` | le rapport au fil des lots |
| `04-knowledge-dump.md` | le knowledge dump du 7 novembre |
| `05-daemon-code-summoning.html` | la page HTML d'origine de Lucie |
| `06-theme-commun.md` | le thème commun avec l'IDE |
| `07-le-code-colore-du-hub.md` | le relevé du code coloré de Luciform Hub |

Les lignes publiques sont dans `docs/nouveautes.md` (le 7 et le 8 novembre).

## Ce qui reste en branche

Rien qui ne soit déjà sur master. Les branches distantes `origin/tty-*` sont seize anciennes étapes (`tty-1`, `tty-1-2`, `tty-1-3`, `tty-1-4`, `tty-2`, `tty-2-2` … `tty-code-3`, `tty-docs-code`, `tty-theme`, `tty-theme-2`, `tty-theme-3`). Toutes sont atterries, et une règle de Lucie interdit de les effacer sans son accord.

## Les thèmes suivants

Lucie a nommé : **fées, forêt, bureau, papillons, Mordor, Dracula** (Dracula au sens vampire, pas la palette de couleurs « Dracula » des éditeurs). Aucun n'est fait.

Chacun sera un fichier de même forme que `templates/themes/daemon.json`, posé à côté. Le plus simple pour commencer est d'en écrire un seul (par exemple `foret.json`), puis d'ajouter au terminal une option `--theme <nom>`.

L'IDE n'a aujourd'hui qu'une route fixe pour `daemon.json`. Archi a dit qu'il ajouterait la route `/ide/theme/<nom>.json` et un menu quand le premier thème suivant arriverait : il faut le prévenir à ce moment-là.

## Le pont Codex ↔ Claude

**Ce n'est pas mon périmètre** : c'est le lot 5 de la session « rag3weaver archi ». Ce que j'en sais :

- **Ce que veut Lucie** (le 7 nov.) : que ses sessions Codex et ses sessions Claude Code se parlent, Codex ayant « un modèle compétent là-bas aussi ».
- **Ce qui est sur master** (archi, `1a0c8d64d`, `e764e6b99`, `6c4f4a743`) :
  - `rag3weaver-backend mail send|reply|pending|seen`, un courrier entre agents dans la boîte de l'hôte ;
  - la lecture des fils Codex (`src/codex_threads.rs`) et la livraison vers Codex (`src/daemon_host/livraison_codex.rs`) ;
  - la conception du « canal » vers Claude Code.
- **La page** : `extension/rag3weaver/docs/07-novembre-2026-hooks-claude-code/07-lot-5-le-pont-claude-code-equipier-codex.md`.
- **La décision de Lucie le 8 nov. au matin** : « Codex plus tard », rien à activer pour l'instant.

## Ce qui reste sur disque

- **Ici** (le portable de Lucie) : les journaux `~/.cache/rag3weaver-build/tty-code.log`, `tty-theme.log` et `tty-theme-rouge.log`. Aucun worktree ni target de la session : ils ont été effacés après chaque lot.
- **À luciepc** (l'autre poste de Lucie, joint par `ssh luciepc`) :
  - les dossiers de journaux de passe `~/.cache/rag3weaver-build/tty-lot1`, `tty-lot1-4`, `tty-lot1-4b`, `tty-lot2`, `tty-lot2-4`, `tty-lot3`, `tty-theme`, `tty-theme-2`, `tty-code` et `tty-code-3` ;
  - les scripts `tty-passe.sh`, `tty2-passe.sh` et `tty-reprise.sh`, à côté de leurs sorties `*.out`, ainsi que `flux-tty.log`.
  - Il n'y a ni worktree ni target.
- Rien n'a été supprimé pour ce rapport.

## Reprendre

**Mise à jour du 14 novembre 2026.** Aucun worktree n'a de travail inachevé, ni sur le portable ni à luciepc. Les lots de la semaine sont sur master, et leurs worktrees et targets ont été effacés après chaque push, par `git cherry` puis par `garde_worktree.py` pour le dernier :

| Lot | Commit |
|---|---|
| forêt | `56deb90ea` |
| cyberpunk, cyborg et alien | `87061f03c` |
| tty-4, la boîte de l'équipier | `212893a17` |
| tty-5, la mesure à travers ssh et le repli en texte brut | `ec4f7aa6d` |
| les docs qui les accompagnent | jusqu'à `d7a8245c9` |

Le worktree de la passation Codex, `~/git_workspaces/cx-docs-tty-features`, est retiré le 14 nov., avec l'accord de Lucie. Son administration est archivée sous `~/.cache/rag3weaver-build/archives/tty-cx-docs-tty-features-14nov/`, avec le rapport VERT de la garde. Les branches distantes `cx-tty-*` et `tty-*` restent.

**Ce qui attend :**
- **tmux et mosh** : leur installation demande le mot de Lucie (c7 le lui demande). Le harnais `extension/rag3weaver/tty/mesures/mesure.py` les mesurera tel quel.
- **Windows et macOS** : le ticket `docs/tty/tickets/2026-11-13-le-terminal-sous-windows-et-macos-a-jouer.md` attend une machine. Le plan est dans `docs/tty/13-11-2026/04-la-mesure-a-travers-ssh.md`.

**Pour reprendre un lot**, partir de master avec un worktree neuf préfixé `cx-` :

```bash
cd ~/git_workspaces/rag3db && git fetch -q origin master
git worktree add ../cx-tty-<lot> -b cx-tty-<lot> origin/master
cd ../cx-tty-<lot> && git submodule update --init extension/rag3weaver/codeparsers
cd extension/rag3weaver/tty
CARGO_TARGET_DIR=~/.cache/rag3weaver-build/target-cx-tty-<lot> POSTE_MEM_MAX=32G ~/.cache/rag3weaver-build/poste lourd timeout 1800 \
  bash -c 'cargo test -j 8 && cargo test -j 8 --no-default-features'
```

Ensuite :
- **La passe** : le crate avec et sans la vue, puis, dans `extension/rag3weaver`, `RAG3DB_LIB=<lib du moment> ./run_e2e.sh --features openai-llm,anthropic-llm --test e2e_tty e2e_tty`. Les e2e se jouent sur un target à eux, neuf : `run_e2e.sh` refuse un target non vide sans provenance. On ajoute `e2e_catalogue_gabarits` si `templates/` bouge, et la passe entière si l'hôte ou `src/` bougent. La lib du moment est à demander à c7 (`0c41f9fe6` depuis le 14 nov.).
- **Le dernier pas**, seul sous `poste push` : `~/.cache/rag3weaver-build/push_garde.sh --code-apporte <parent du premier commit du lot> origin/master` décide du rejeu, puis `--derive "$PWD"` après le rebase. On pousse seulement si le verdict est exactement `VERT`, puis on lit les trois contrôles, et le hash part à c7.
- **Le ménage**, après le push : `git cherry` à 0, l'arbre propre, aucun processus, puis le worktree et le target effacés. Les branches distantes restent.

## Ce qui est bloqué, et sur quoi

- **Le ticket `docs/tickets/closed/2026-11-07-la-lib-rag3weaver-ne-se-batit-pas-sans-rag3db-native.md`** (ouvert, famille produit) : la lib `rag3weaver` ne compile pas sans la feature `rag3db-native`. Deux sorties possibles : poser deux `#[cfg(feature = "rag3db-native")]` (`src/backend.rs` vers la ligne 4411, `src/dataflow/record_nodes.rs` vers la ligne 4567), ou déclarer que la lib exige le moteur. Le choix revient à Lucie. Le terminal n'en souffre pas : il vit dans son propre crate.
- **Rien d'autre n'est bloqué.** Lucie n'a pas encore dit si l'ordre des prochains pas lui convient.

## Les tickets ouverts qui sont à moi

| Ticket | En une ligne |
|---|---|
| `docs/tickets/closed/2026-11-07-la-lib-rag3weaver-ne-se-batit-pas-sans-rag3db-native.md` | la lib ne compile pas sans `rag3db-native` (contourné par le crate à part du terminal) |

Il n'a pas été déplacé : c7 dira « déplace » quand le script d'index lira `docs/*/tickets/`.

## Les trois prochains pas

1. **tty-4, la boîte de l'équipier dans le terminal.** Il s'agit de montrer ses questions, d'y répondre (`answer` avec `reply_to`) et de valider ou refuser ses demandes d'exécution de commande (`approval`). On remplace aussi l'état déduit des événements par l'état explicite que l'hôte sert maintenant (`running|waiting|ended|error`). La forme exacte est dans `extension/rag3weaver/docs/07-novembre-2026-everything-declarative/01-l-equipier-agent-de-code.md`, § 2.
2. **tty-5, la mesure à travers les liens.** On mesure le délai entre une frappe et son écho, et les octets par seconde, sous ssh, tmux et mosh, de luciepc vers le portable. Il faut aussi un passage sous Windows et un sous macOS, puisque le terminal doit tourner partout. Et regarder une fois de plus la durée variable d'`e2e_tty` (4 s, une fois 52,9 s).
3. **Le premier thème suivant** (par exemple « forêt »), avec l'option `--theme`, puis prévenir archi pour la route et le menu de l'IDE.


## Actualisation du 13 novembre 2026 — compilation minimale

Le blocage de compilation sans `rag3db-native` décrit dans l'état historique
ci-dessus est levé. Ticket clos :
`docs/tickets/closed/2026-11-07-la-lib-rag3weaver-ne-se-batit-pas-sans-rag3db-native.md`.
Correctifs `9231f07dc`, `c74e3310d`, garde minimale `5a4049b9e` ; F8 canonique
8/8 sur APP `9ec421069`, preuves publiées par `cdcc032df` dans le rapport
Recherche `docs/recherche/13-11-2026/13-preuves-finales-union-garde-target.md`.
Tty a relu les sources, la garde et les journaux minimaux conservés ; aucun
Cargo, test, service ni bâti nouveau. Ce défaut n'attend plus de décision de
Lucie. Le crate tty séparé est conservé ; aucun nouveau lot autorisé par cette
régularisation. Les propositions tty-4, mesures et thèmes restent des pistes
historiques en attente d'objectif établi, pas des actions démarrées.
