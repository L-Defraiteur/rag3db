# rag3weaver archi — rapport de session

Mis à jour le 7 novembre 2026 à 15 h 35. Ce fichier se met à jour sur place. La session est
l'ancienne orchestration des 3-4 octobre (rag3db-9f), puis le chantier E du 8 octobre (Claude
en agent) ; depuis le 7 novembre, le §1 du plan du week-end : les hooks de Claude Code.

## 1. Fait aujourd'hui

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

- Lot 3 bis : la table `serveurs` dans le run du Stop (depuis `~/.claude.json`), pour que le juge
  de 73 rejoue nos outils ; un témoin sur un vrai gabarit `memory` dans l'hôte de test (mon faux
  hôte répond à tout `cited_by` : c'est ce qui a caché le défaut de la valeur du handle) ; `whois`
  à la place de `cited_by` au SessionStart quand la branche de 73 sera sur master.
- Lot 4 : `ranger` / `archive` (pas 3 de 96) pour PostCompact ; la seconde moitié de l'exigence
  de Lucie (redéclaration des gabarits par propositions, vue de l'IDE) est en suite dans la page 04.
- Premier usage réel : la session `hooks-essai` de Lucie (bloc rendu, fiche écrite, 14 h 55) ; les
  trois défauts qu'elle a montrés et les deux de 73 sont dans le lot 3. Elle doit relancer une
  fois pour prendre la commande stable ; après, plus jamais.
- Jamais joué : la passe d'agent Gemini / Claude (page 03 du 10 octobre) — attend le mot de Lucie.

## 4. Ce qui attend Lucie

- La CI de master est rouge depuis 13 h 58 UTC (lue par le push) : à regarder, ce n'est pas un rouge local.
- Le repli serveur d'Anthropic en produit (hors comparaison) ; le modèle de la seconde passe.

## 5. Comment reprendre

Mémoire `hooks-claude-code-chantier` ; pages 01-03 du dossier ; worktree `wt-hooks2`, target
`hooks2`, docs par `wt-docs-archi` (détaché de master, push `HEAD:master` sans passe) ; scripts
sous `~/.cache/rag3weaver-build/wt-hooks2-{passe,push}.sh`, `controle-du-worktree.sh`.
