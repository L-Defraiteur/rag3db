# Les hooks de Claude Code — la mémoire qui parle à la session

7 novembre 2026, rag3weaver archi (ex-orchestration, chantier E), pour c7 et 96. Le §1 du
plan du week-end (`../06-novembre-2026-orchestration/01-plan-du-week-end.md`). Page de
conception **avant le code** ; rien n'est codé. Elle s'appuie sur la vision du 4 octobre
(`visions/2026-10-04-00h09-l-archiviste-branche-sur-claude-code.md`), la page M9 du 18
octobre (`../18-octobre-2026-memoire-longue/04`, §3 : la porte `ranger`) et la page de
l'identité du 25 (`../25-octobre-2026-memoire-longue/04`, §6 : l'apport `identity`).

## 1. Ce que Claude Code donne, relevé dans sa documentation le 7 novembre

Relevé par un agent de lecture de la documentation officielle (`code.claude.com/docs/en/hooks`) ;
les lignes marquées ◇ ne sont lues que dans des résumés de la page, à relire avant de figer.

| événement | reçoit sur stdin (JSON) | peut rendre | délai par défaut |
|---|---|---|---|
| tous | `session_id`, `transcript_path` (le `.jsonl`), `cwd`, `hook_event_name`, `prompt_id` | — | 600 s |
| `SessionStart` | `source` : `startup`, `resume`, `clear`, `compact`, `fork` ; au `resume`/`fork` : `context_tokens` | du contexte pour Claude : stdout brut sur code 0, ou `hookSpecificOutput.additionalContext` ◇ | 600 s ◇ |
| `UserPromptSubmit` | `prompt` | du contexte ajouté au prompt (stdout brut ou `additionalContext`) ◇ ; code 2 **bloque** le prompt | **30 s** |
| `PreCompact` | `trigger` : `manual`/`auto`, `custom_instructions` | **pas de contexte** : le stdout devient les **consignes du résumé** de compaction (10 000 caractères au plus) ; peut bloquer | 600 s |
| `PostCompact` | `trigger`, **`compact_summary`** | rien : effets de bord seulement | 600 s |
| `Stop` | `stop_hook_active`, `last_assistant_message` | `additionalContext` (continue la conversation) ; code 2 empêche l'arrêt | 600 s |

Trois conséquences qui changent le brief :

- **`PreCompact` ne peut pas injecter de contexte.** « Recevoir en retour ce qu'il faut
  garder en tête » se fait à `SessionStart` avec `source: compact` — c'est le premier tour
  après la compaction, et c'est là que la mémoire remplace le contexte perdu.
- **Le résumé de compaction nous est donné** (`PostCompact.compact_summary`) : c'est la
  parole de l'agent sur sa propre session, prête à ranger. Et `PreCompact` peut dire au
  résumé ce qu'il doit garder (les six rubriques de l'identité, le plan, ce qui est tenu).
- **Aucun événement ne dit le remplissage du contexte** hors `context_tokens` au `resume`.
  L'estimation se fait sur la taille de la transcription, et c'est une estimation.

## 2. Le dessin : une commande mince, tout le travail dans l'hôte

Un seul exécutable pour les cinq événements : **`rag3weaver-backend hook`**, qui lit le JSON
du hook sur son entrée, parle à l'hôte 7970 (`POST /session` avec le jeton de
`~/.local/share/rag3weaver/daemon.token`, puis `POST /call` sur le backend mémoire déclaré),
rend son bloc sur stdout, et **sort toujours en 0**. C'est la « commande mince du crochet »
de M9, étendue aux cinq événements.

Règles, toutes dans la commande, aucune dans l'hôte :

- **Jamais bloquant** : jamais de code 2, jamais `decision: block`, jamais `continue: false`.
  Un délai propre, plus court que celui de Claude Code (`UserPromptSubmit` 5 s, les autres
  20 s) : au-delà, le bloc est vide et la raison va au journal.
- **Sans hôte, elle se tait** : l'hôte injoignable, le jeton absent, le backend non ouvert,
  un outil qui refuse — rien sur stdout, une ligne datée dans
  `~/.local/share/rag3weaver/hooks.log` (`<événement> <session> : <raison>`), code 0. Elle ne
  démarre **jamais** d'hôte (l'unité systemd le fait au premier appel ; une session Claude
  Code qui ouvre un hôte avec un vieux binaire est exactement ce que Lucie ne veut plus).
- **Vide quand il n'y a rien** : un bloc toujours présent apprend au modèle à ne plus le
  lire (la règle du bloc d'attente de `agent.rs`). Le bloc est borné (1 500 caractères pour
  `UserPromptSubmit`, 3 000 pour `SessionStart`), en Markdown, et commence par une ligne qui
  dit d'où il vient (`[mémoire]`).
- **Elle ne devine pas l'identité.** La clé d'une session se lit par `cited_by
  handle:session:<session_id>` (page 04 §4). Sans fiche : le bloc de `SessionStart` le dit
  une fois (« aucune fiche d'identité pour cette session : choisis ta clé, `put_fiche`,
  puis un L1 `handle` ») et rien n'est créé à sa place. Rien d'automatique sur l'identité,
  qui est la parole de l'agent.
- **Le réglage** : `~/.config/rag3weaver/hooks.json` — le manifeste du backend mémoire
  (celui de `~/.claude.json`, `memoire-longue`), le seuil d'estimation, les bornes, le
  journal. Posé par le script d'installation, lisible et modifiable à la main.

## 3. Événement par événement

| événement | lit | écrit | rend |
|---|---|---|---|
| `SessionStart` (`startup`, `resume`, `clear`) | la fiche d'identité par `handle:session:<id>` ; son dernier L1 `identity` ; la dernière fiche de tâche qu'elle cite (`doing`) et ses apports `plan` ouverts | un L1 `handle` si l'adresse a changé (`uds:` n'est pas donnée au hook : seul `session:` l'est) | le whois (clé, alias, pairs), le `doing` et le `holding` du dernier `identity`, le plan en cours, en trois paragraphes |
| `SessionStart` (`compact`) | le dernier L1 `identity` et le L1 `compaction` que `PostCompact` vient d'écrire | — | le même bloc, précédé de : « ton contexte a été compacté ; écris maintenant ton apport `identity` (six rubriques) sur ta fiche, puis reprends ». La consigne, pas l'écriture |
| `UserPromptSubmit` | `recall_fiches(prompt)` (3 fiches au plus, nom + `about`) ; les apports `diff` des dernières 48 h sur les fichiers que la transcription montre touchés par cette session (`cited_by path:<fichier>`, section `diff`, autre auteur) ; la fiche `bilan : <tâche>` du dernier verdict du juge pour la tâche citée dans `doing` ; la taille de la transcription | rien | un bloc en trois lignes au plus : « tu tiens un plan sur… », « `src/x.rs` a changé depuis par <agent> : … », « le juge a noté que … » ; et, au-delà du seuil, une quatrième : « ta transcription fait ~N k jetons : range maintenant (un apport, un plan clos) plutôt que d'attendre la compaction » |
| `PreCompact` | le dernier L1 `identity` | `ranger` (M9) : la tranche de transcription depuis le dernier `Rangement` de cette session, motif `pre_compact:<trigger>` — en arrière-plan dans l'hôte | sur stdout, **les consignes du résumé** : garder les six rubriques de l'identité telles qu'elles, le plan courant et ses pas faits/restants, les worktrees et branches tenus, les mandats reçus, les pairs par leur clé |
| `PostCompact` | `compact_summary` | `put_source` (`sources/<jour>-compaction-<session>.md`, `via: hook`) et un L1 de section `compaction` sur la fiche de session, `author` = la clé, cité `doc:` et `handle:session:` | rien |
| `Stop` | `last_assistant_message`, et dans la transcription les appels d'outils du tour (depuis le `prompt_id`) | la fin de run : `POST /journal/run` (nouvelle route, petite) qui écrit `runs_root/claude-code/<session>-<prompt_id>.json.zst` dans la forme du journal des runs (les tours tels que la transcription les donne, les appels d'outils, le dernier message), pour que le juge le juge comme un tour du chat | rien — jamais d'`additionalContext` au `Stop` : c'est la porte des boucles (`stop_hook_active`) |

Ce qui est nouveau dans l'hôte, et seulement cela : la route `POST /journal/run` (un run
venu de dehors, dans le journal existant) ; `ranger` est M9 et reste un lot à part — tant
qu'elle n'existe pas, `PreCompact` ne fait que rendre ses consignes.

L'estimation du contexte : octets de la transcription ÷ 4, moins ce qui précède le dernier
`compact_summary` lu dans le fichier ; seuil par défaut 60 % de 200 000 jetons, réglable.
C'est grossier et dit comme tel ; `SessionStart` en `resume` corrige par `context_tokens`.

## 4. L'adaptateur de transcription

Le `.jsonl` est interne à Claude Code et non documenté : l'adaptateur (`src/claude_code_transcript.rs`)
est petit, tolérant, et ne lit que ce dont les hooks ont besoin — les lignes de rôle
`assistant` et `user` depuis une position, les blocs `tool_use` (nom, `file_path`), les
marques de compaction. Une ligne qu'il ne comprend pas est sautée et comptée, jamais une
raison de se taire. Deux transcriptions réelles enregistrées (une courte, une après
compaction) servent de témoins ; un format qui bouge casse un test, pas une session.

## 5. L'installation

`scripts/hooks_claude_code.sh` : idempotent, dit ce qu'il pose. Il écrit `hooks.json`, puis
fusionne dans `~/.claude/settings.json` les cinq entrées, marquées (`"command"` qui commence
par le chemin du binaire installé, `~/.local/bin/rag3weaver-backend hook`), sans toucher aux
autres hooks ; relancé, il ne change rien et le dit ; `--retirer` enlève les siennes.
`SessionStart` et `PreCompact` portent un `matcher` (`startup|resume|clear|compact`,
`manual|auto`) ; `UserPromptSubmit` et `Stop` n'en ont pas. Le `timeout` de chaque entrée est
celui du §2 plus cinq secondes.

## 6. Les témoins

- **Un faux hôte** (le `FakeServer` des tests, sur HTTP) : les cinq événements rejoués depuis
  des JSON enregistrés, et pour chacun le bloc rendu comparé mot à mot ; l'ordre des `/call`.
- **Sans hôte** : port fermé → stdout vide, code 0, la ligne au journal ; jeton absent, idem.
- **Le délai** : un faux hôte qui ne répond pas → bloc vide en 5 s, pas 30.
- **L'adaptateur** : les deux transcriptions réelles ; une ligne corrompue au milieu.
- **L'installation** : un `settings.json` avec d'autres hooks avant, le même après plus les
  cinq ; relancé deux fois, identique ; `--retirer` rend l'original.
- **De bout en bout**, à la main et dit au rapport : une vraie session Claude Code, sa fiche,
  une compaction forcée (`/compact`), le L1 `compaction` relu dans la mémoire de Lucie.

## 7. Les lots, dans l'ordre

1. La commande `hook`, le réglage, le journal, le silence sans hôte, le faux hôte, le script
   d'installation — rien d'injecté encore.
2. `SessionStart` et `UserPromptSubmit` : lecture seule, les blocs rendus. C'est le lot qui
   se juge à l'usage : une session de c7 ou de 96 avec les hooks, une journée.
3. `PostCompact` et `Stop` : les écritures (le L1 `compaction`, la route `/journal/run`).
4. `PreCompact` : les consignes ; `ranger` quand M9 arrive.

## 8. Ce qui attend c7 et 96

- **Où vit le plan d'une session Claude Code** : un L1 de section `plan` sur sa fiche de
  tâche (comme l'équipier), ou un apport sur la fiche d'identité ? Je propose la fiche de
  tâche, pour que `find_tasks` (§3 du plan) le trouve par le sujet.
- **Le `/journal/run` depuis dehors** : le journal des runs accepte-t-il un run qu'aucun bus
  n'a vu, ou faut-il un sujet `hook` ? Question à 73 (arbre principal), qui le tient.
- **Le seuil** (60 %) et les bornes des blocs : des réglages déclarés, les chiffres sont un
  point de départ.
- **L'`identity` après compaction est une consigne, pas une écriture** (page 04 §6 : « c'est
  la parole de l'agent »). Si Lucie préfère que le hook l'écrive à partir du
  `compact_summary`, c'est une ligne de plus dans `PostCompact` — à dire.
