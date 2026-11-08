# rag3weaver archi — rapport de session

Mis à jour le 8 novembre 2026 à 18 h 45. Ce fichier se met à jour sur place. La session est
l'ancienne orchestration des 3-4 octobre (rag3db-9f), puis le chantier E du 8 octobre (Claude
en agent) ; depuis le 7 novembre, les hooks de Claude Code, puis, le 8, l'IDE (la liste de Lucie
du 7 au soir).

## 0. Le 8 novembre

| lot | master | passe |
|---|---|---|
| hooks 3 ter (d) à (g) : l'échéance du client (`deadline_ms`), le vocabulaire et les fiches du juge, `recall_min_score` retiré, le plafond et la clé du juge du rappel (page 08) | 624aaadc9, dc0d24f25, 6fa692df5, d4c1d1b12, 52021b977, 9daa2a450 | lib entière, e2e_hooks, navigateur |
| la sonde de débit à horloge injectée | 96ad1e764 | lib |
| la barre du chat : l'usage du tour et des tours récents, le récapitulatif qui déplie les fiches, l'archiviste après un rechargement (page 08 de l'IDE) | b129247a9, 2daf2e332 | 5/5, lib 2 050, navigateur 22 |
| « Dépôts » se rafraîchit à `RepoAttached` | 1005c5544 | 5/5, lib 2 051, navigateur 23 |
| le thème daemon dans l'IDE (le fichier commun du terminal, l'invocation « réfléchit ») | 7dac6418b, 44442ea75 | 5/5, lib 2 051, navigateur 24 |
| le symbole cité en lien (`POST /ide/symboles`, `Backend::call_discret`) | 6c17c5766, 36aced578 | 5/5, lib 2 058, navigateur 25, e2e_lectures 24 |
| un graphe de clic vide fait attendre l'aperçu au lieu de répondre 422 (le rouge de codeparsers sous charge 32) | 3d968fb98 | pile : 5/5, lib 2 063, navigateur 29 |
| l'accueil parle en dépôts (« Dépôts », « dépôt · branche », le manifeste en infobulle) | e16cd5c56, 4fd994dd8 | (pile) |
| la fin du premier index d'un worktree attaché : RepoIndexed, une ligne dans le chat de l'atelier, le rag indexé dans la barre (Lucie, 16 h 08) | 521ade9d8, 8a81ed38a | (pile) |
| l'avancement de l'index par le flux : IndexProgress, la barre dit le pourcentage, le débit, « figé depuis », « flux coupé » (Lucie, 16 h 30-31) | 6044f5ae2, 9c20d6773 | (pile) |

Docs : page 03 du 8 novembre (`08-novembre-2026-orchestration/03-ide-ce-qui-reste.md`) — la
liste de Lucie relue demande par demande ; elle est faite. Tickets : le code dans le chat fermé ;
le graphe de clic vide fermé avec son lot ; une ligne au ticket des tests sous charge (le creux
d'e2e_lectures à 14 h 37, le tunnel 7979 de luciepc à 2,1 s).

Ce que la journée a appris (aussi en mémoire) : un `pgrep -f` avec le motif dans sa propre ligne
tue son shell ; arrêter une passe, c'est le script et ses descendants, par leurs pids ; un
`indexings_running` compte les vecteurs en fond, ce n'est pas « l'index n'est pas prêt » ; un
`status` d'`usages` peut être la fraîcheur, seul « résultat partiel » dit un index partiel ;
`Backend::call` publie sur le bus du dépôt, que le journal suit.

## 1. Le 7 novembre

| lot | branche → master | passe |
|---|---|---|
| anthropic-llm-2 : le client de l'API Messages d'Anthropic derrière `Llm`, `connect_llm` rend `Arc<dyn Llm>` pour tous les protocoles, feature `anthropic-llm` au jeu large | 90df16ace | check_features 5/5, lib 1 904, e2e_lectures 20, navigateur 12 |
| hooks lot 1 : la commande mince `rag3weaver-backend hook`, réglage, journal, silence sans hôte, script d'installation sur demande (page 02) | 2d3ecad1e | 5/5, lib 1 912, script, navigateur 12 |
| hooks lot 2 : SessionStart et UserPromptSubmit en lecture, adaptateur de transcription, cibles du script (dossier courant, --settings, --global, --worktree) (page 03) | d42787227 | 5/5, lib 1 928, script, navigateur 12 |
| hooks lot 3 : PostCompact (L0 + L1 compaction), Stop (run claude_code_turn, route /journal/run, file des bilans), PreCompact (consignes), handle sous RAG3WEAVER_AGENT_KEY, rappel de l'identity ; gabarits déclarés sous templates/hooks relus à chaque appel, commande stable, --rejouer, --help, versions au journal ; les trois défauts du premier usage et les deux de 73 (page 04) | 8f60895b8 (rejoué sur le master d'INCLUDES) | 5/5, lib 1 946, script, navigateur 12 |

Docs seules sur master : la page 01 de conception des hooks (81b9b9176) ; la règle « le worktree
est jetable » dans `docs/procedures/push-d-un-lot.md` (7f8a3fb75).

## 2. Décisions prises ou reçues

- PreCompact n'injecte pas : SessionStart `compact` et PostCompact (`compact_summary`) font le
  travail (relevé de la doc Claude Code, page 01 §1).
- Le hook écrit le mécanique, l'agent écrit son `identity` (c7 pour 96, mot de Lucie du 25).
- Le contexte se lit sur `message.usage`, pas sur les octets (mesuré : page 02).
- `fallbacks` serveur d'Anthropic éteint pour la comparaison ; `temperature`/`top_p` jamais envoyés.
- `RAG3WEAVER_AGENT_KEY` : un indice, jamais une identité (96).

## 3. En cours et suite

- La carte de l'avancement sous l'appel d'un outil (`ide-carte-ui`, worktree `wt-carte`, commit
  local 165c3d382) : un registre `kind → composant` pour les `ui` d'un résultat d'outil ; attend
  `Turn.ui` / `ToolCallFinished.ui` d'everything declarative (branche atelier-ui) sur master,
  puis son témoin navigateur rouge sur master et vert (attach, `show_progress`, rechargement).
- La reprise de l'index au redémarrage (everything declarative) appellera `suivre_le_premier_index`
  par `lancer_sur` avec `BORNES_DU_SUIVI`.
- Les thèmes suivants (chez tty) : une route `/ide/theme/<nom>.json` et un menu, au premier fichier.

## 4. Ce qui attend Lucie

- La CI de master est rouge depuis 13 h 58 UTC (lue par le push) : à regarder, ce n'est pas un rouge local.
- Le repli serveur d'Anthropic en produit (hors comparaison) ; le modèle de la seconde passe.

## 5. Comment reprendre

Mémoires `hooks-claude-code-chantier`, `orchestrer-les-sessions-rag3db` ; page 03 du 8 novembre ;
un worktree par lot sous `~/.cache/rag3weaver-build/wt-<lot>`, sa passe `wt-<lot>-passe.sh`
(ETAPE=module | navigateur | complete) et son push `wt-<lot>-push.sh` (sort à 3 si du code est
arrivé) ; docs par `wt-docs-archi` (détaché de master, push `HEAD:master` sans passe).
