# Parcours d'essai, pour Lucie, le samedi 21 novembre 2026

Ce qui se lance, ce qu'on essaie au terminal et dans l'IDE, ce qu'on doit voir à chaque pas, et ce qui manque encore, avec le contournement. Écrit par tty, avec la part d'archi sur l'équipier et les relevés de recherche dans l'IDE, d'après les essais de la nuit, joués par le chemin de l'hôte. Les relevés détaillés :
- tty : `docs/tty/21-11-2026/01-releve-scenario-2-un-projet-de-zero.md` et `02-releve-scenario-1-un-dev-qui-connait-les-lacunes.md` ;
- archi : `docs/rag3weaver-archi/20-11-2026/05-parcours-d-essai-l-equipier.md` et `docs/rag3weaver-archi/21-11-2026/01-les-rejeux-de-la-nuit.md` ;
- recherche : `docs/recherche/21-11-2026/01-scenario-2-un-projet-de-zero.md` et `02-scenario-1-travailler-dans-l-ide.md`.

## Avant de commencer

- **L'hôte et le terminal du matin** sont dans `~/.local/bin` : `rag3weaver-backend` et `rag3weaver-tty`, bâtis de la même tête par G.
- **Après une pose**, un gros dépôt peut refuser le chat et les outils pendant de longues minutes, le temps de se rouvrir (`rag3db` : dix-huit minutes cette nuit). Relire `daemon_status`, ou attendre « flux : ouvert » et le dépôt « chargé » dans l'IDE, avant de conclure à une panne.

## 1. Au terminal

1. **Un dépôt** : `rag3weaver-backend code-tree add <dossier du projet> --from templates/backends/code/backend.json`, depuis la racine du crate.
   - Attendu : « code-<nom> : manifeste … », puis « … enregistré à l'hôte ».
   - Un projet neuf : `git init`, une identité git dans le dépôt si la configuration globale n'en a pas (le message le dit), et **un premier commit** (`git commit --allow-empty -m début`), sans quoi aucune tâche ne s'ouvre.
2. **Une tâche** : `rag3weaver-tty --repo <nom> --task <tâche> --request "<ta demande>"`.
   - Attendu : ta demande dans l'historique, le travail qui défile en bas (l'animation du thème, ce qu'il attend nommé à côté), puis la réponse.
   - Essayer : une question en lecture seule (« où est définie X, que fait-elle ? ») ; puis une consigne d'édition (elle s'écrit sur la branche `agent/<tâche>`, jamais dans ton arbre) ; puis une commande hors de la liste sûre, pour voir un accord.
3. **Les accords** : la ligne d'état dit « attend ton accord : <commande> », avec `[ Oui ]   Non`. `o` ou `n` répondent, ← → et Entrée aussi. Ce qui attend en plus est compté (« (+2) »).
   - Attendu : `› accordé : <commande>`, puis la commande part.
4. **La mémoire** : `/memory` liste les L1 du run de la tâche ; `/turns <n>` ouvre leurs tours dans la visionneuse (Ctrl-O).

## 2. Dans l'IDE

1. **La page d'une tâche**, `/ide/code/<id du dépôt>/taches/<tâche>`, même lancée au terminal : la boîte bouge au flux, sans recharger.
   - « Écrire à l'équipier » envoie un tour : il paraît au terminal sous `[ide · demande]`, et l'équipier y répond.
   - Une commande qui attend ton accord paraît en carte, « Oui » ou « Non » ; la réponse paraît au terminal (`› accordé`).
2. **Le chat d'un dépôt**, et ses fenêtres détachées :
   - « dépôt à côté » : un fichier cité dans le chat (`src/main.rs:12`) s'ouvre dans la fenêtre du dépôt, à sa ligne ;
   - « graphe à côté » : la fenêtre suit les outils du tour ;
   - l'en-tête d'une fenêtre détachée compte les accords qui attendent dans le chat, et dit « l'index … est prêt » quand l'index a rattrapé.
3. **L'index pas prêt** : sous un outil qui a répondu avant que l'index ne rattrape, une ligne dit combien de temps il faut encore, pour le texte et pour les vecteurs ; une fois l'index à jour, une seule ligne le dit.
4. **Un projet de zéro depuis l'IDE** : le champ « Ouvrir un dossier » de l'accueil (recherche, `ide-ouvrir`, dans le second hôte du matin) ; sinon, au terminal, `rag3weaver-backend atelier` depuis le dossier du projet.

## 3. L'équipier, de bout en bout (la part d'archi)

- Une petite demande se fait en un pas, sans plan ; une demande en trois éléments ouvre un plan et le suit pas à pas ; une précision envoyée en route s'ajoute à la demande ; un « arrêt » coupe le pas.
- Chaque pas voit ce que le pas d'avant a lu, écrit et lancé. Au troisième pas de suite sans élément réussi, l'équipier s'arrête et le dit.
- **En file, pas encore sur l'hôte** (lot `raisonnement-borne-3`) : une réponse du modèle coupée à son plafond relancée, au plus deux fois par pas ; le modèle par défaut à `reasoning: low`, réglable par `"models": {"llm": {"reasoning": "medium"}}` dans un manifeste. **Sans ce lot**, un Gemini 3.x par défaut peut remplir sa sortie de raisonnement et couper sa réponse avant tout outil : le pas finit alors sans rien faire. `/model` liste déjà les modèles d'une tâche.

## 4. Ce qui manque encore, et comment contourner

| Ce qui manque | Contourner |
|---|---|
| `use_view` refuse le nom que rend `open_view` | Rien à faire : `open_view` active la vue ; sinon le nom `worktree:/…` que liste `views` |
| L'équipier peut dire « fini » sur un arbre qui ne compile pas (archi, rejeu de la nuit ; correctif en cours, `teammate.verify`) | Lui demander de jouer le test nommé et de coller sa dernière ligne |
| Une tâche sur un dépôt sans commit ne s'ouvre pas | Un premier commit : le message le dit, avec la commande |
| Une carte d'accord par commande, parfois en double | Répondre à la plus récente d'abord, `o` ou `n` |
| `cargo check` peut rester long ou bloqué dans une tâche sous charge | Lui laisser le temps ; la boîte le dit (« the command is still running ») |
| `search_code` sans `options` refuse | Passer `options: {}` |
| Pas de `fetch` ni de recherche web, pas de cron, pas de navigateur pour l'équipier, pas encore de palette par familles, pas de `run` en fond ; le sous-agent à vues est en file (`sous-agent-vues-3`) | — (le navigateur est le prochain chantier de tty) |
| Le mode pair existe, mais seulement avec `teammate.pair: true` | À n'allumer qu'en le voulant |

## Corrigé cette nuit (atterri ou en file)

- **`close_view` dit ce qu'il laisse** (arbre).

- **On écrit dans une vue ouverte par `open_view`** (arbre).

- **Le terminal reconnaît un dépôt sous tous ses noms** (`--repo` résolu par l'hôte, atterri) : le nom court, `code-<nom>`, le chemin du projet ou du manifeste.

- **Au terminal, un message tapé pendant qu'un accord attend reste un message** (atterri, PR 301) : seuls `o` et `n` minuscules, seuls, répondent tout de suite.
- **Une tâche sur un dépôt sans commit le dit**, avec la commande (atterri).
- **Une commande longue est dite « still running »**, et plus « en attente d'un index » (atterri).
- **Le terminal est posé avec l'hôte**, dans `~/.local/bin`, aux deux postes (G).
- **L'équipier peut commiter depuis sa tâche** (archi, `bac-commit-worktree`, dans l'hôte de 6 h 10) : le bac à sable écrit le gitdir du worktree de la tâche, ses objets et la ref de sa branche, jamais `config`, `hooks/` ni `info/`.
- **L'agent n'invente plus d'identité git** (archi, atterri, `identite-git`) : le bac à sable reçoit `user.name` et `user.email` de la config git globale de l'hôte, et refuse `git config user.*`, accord compris. Il faut donc une identité dans sa config git globale.
