# Lot 2 — les blocs de lecture : SessionStart et UserPromptSubmit, et l'essai sur un vrai worktree

7 novembre 2026, rag3weaver archi. Le second lot de la page 01 (§7). Le hook **lit** la mémoire
et la transcription, et rend ses deux blocs ; il n'écrit toujours rien dans la mémoire.

## 1. L'essai de Lucie : une seule session, du vrai travail

Lucie : « une seule session Claude qui a les hooks, sans toucher les autres », et « elle se rend
utile tant qu'à faire, elle choppe un worktree depuis là où elle est ». Trois niveaux de cible
pour `scripts/hooks_claude_code.sh`, du plus étroit au plus large :

| cible | fichier | pour qui |
|---|---|---|
| (défaut) | `.claude/settings.local.json` du dossier courant | la seule session lancée dans ce dossier |
| `--settings <fichier>` | un fichier à soi | `claude --settings <fichier>` |
| `--global` | `~/.claude/settings.json` | toutes les sessions — **jamais sans le mot de Lucie** |

Et l'essai sur du vrai travail :

```
scripts/hooks_claude_code.sh --worktree hooks-essai --poser
#  → worktree ~/git_workspaces/rag3db-hooks-essai (branche hooks-essai, depuis origin/master),
#    sous-module à jour, les cinq hooks dans son .claude/settings.local.json, puis :
cd ~/git_workspaces/rag3db-hooks-essai && claude
#  le bloc [mémoire] apparaît au premier tour ; le journal : ~/.local/state/rag3weaver/hooks.log
scripts/hooks_claude_code.sh --worktree hooks-essai --retirer --poser     # retirer
```

Le réglage (`~/.config/rag3weaver/hooks.json`) est commun à toutes les cibles ; il n'est posé
qu'une fois et jamais réécrit. Le premier lot réel de cette session, donné par c7 : la recherche
sans dossier dans l'IDE (la saisie disponible dès l'ouverture du backend, le dossier en filtre
facultatif, témoin navigateur) ; sa fiche de session et ses apports seront le premier matériau du
juge sur une session Claude Code.

## 2. Ce que le hook rend

**`SessionStart`** (`startup`, `resume`, `clear`, `compact`) — jusqu'à 3 000 caractères :

- qui est la session : la fiche d'identité qui cite `handle:session:<id>` (`cited_by`, genre
  `handle`) — `Tu es **96** — la session de la mémoire longue` ; puis les rubriques `doing`,
  `holding`, `mandates`, `peers` de son dernier apport `identity` ; puis **le plan en cours** : la
  première fiche citée par `doing` (`fiche:<nom>`), son dernier apport `plan`, ou « sans plan
  écrit » ;
- après une compaction (`source: compact`), la consigne d'abord : « écris ton apport `identity`
  avant toute autre chose : c'est à toi de dire qui tu es, pas au harnais » ;
- **sans fiche** : le bloc dit comment s'inscrire (le L1 `handle` par `put_apport` +
  `relier_apport` + `citer_source`, ou la fiche d'identité d'abord) et **rien n'est créé à sa
  place** ; `RAG3WEAVER_AGENT_KEY` est un indice facultatif qui ne vaut que si la fiche de cette
  clé existe — une variable ne crée pas une identité (96).

**`UserPromptSubmit`** — jusqu'à 1 500 caractères, vide quand il n'y a rien :

- **le défi d'identité** au premier tour après une compaction (aucun tour `assistant` depuis la
  frontière, lu dans la transcription) : « écris ton apport `identity` avant toute autre
  chose », avec la fiche et le dernier `identity` connu ; au tour suivant, plus rien ;
- **les fiches en rapport** avec le prompt (`recall_fiches`, 3 au plus, états `current` et
  `to_review`, prompts de 8 caractères et plus) : nom et `about`, « `get_fiche` pour lire » ;
- **les fichiers que la session a touchés** (`Edit`, `Write`, `MultiEdit`, `NotebookEdit` dans la
  transcription, les cinq derniers) et qu'un **autre** agent a changés depuis 48 h : les apports
  `diff` que `cited_by path:<fichier>` rend, deux au plus ;
- **le dernier verdict du juge** sur la tâche courante : la fiche `bilan : <tâche>`, son dernier
  apport `bilan` ;
- **le contexte** : `message.usage` de la dernière ligne `assistant` (exact ; les octets ÷ 4 en
  repli, dit comme tel) contre `context_tokens` × `suggest_ratio` (1 000 000 × 0,7) : « range
  maintenant ce qui doit l'être », **une fois par session** (une marque sous
  `~/.local/state/rag3weaver/hooks-suggestions/<session>`).

Chaque bloc commence par `[mémoire]`, est borné en caractères et le dit (`[… borné]`) ; chaque
lecture qui échoue va au journal, jamais dans le bloc. `PreCompact`, `PostCompact` et `Stop`
journalisent l'hôte joint et ne rendent rien encore.

## 3. L'adaptateur de transcription (`src/claude_code_transcript.rs`)

Les formes relevées le 7 novembre sur les transcriptions de ce poste, et rien d'autre :

| ligne | ce qu'on lit |
|---|---|
| `type: assistant` | `message.usage` (`input_tokens` + `cache_read_input_tokens` + `cache_creation_input_tokens`) = le contexte réel du tour ; les blocs `tool_use` d'édition et leur `input.file_path` |
| `type: system`, `subtype: compact_boundary` | une compaction, son `compactMetadata.trigger` |
| `type: user`, `isCompactSummary: true` | le résumé de compaction (`message.content`) |

Une ligne illisible est comptée et sautée ; « premier tour après compaction » = une frontière vue
et aucun `assistant` depuis. Témoins : l'usage du dernier tour (un tour sans usage ne l'efface
pas), les fichiers sans doublon (`Read` n'édite pas), la compaction et le tour d'après, les
lignes illisibles, le repli par les octets, un fichier absent dit.

## 4. Les témoins du lot

- `claude_code_hook::tests`, sur un hôte canné par outil : `SessionStart` rend qui je suis, ce que
  je tiens, les pairs et le plan en cours, et appelle `cited_by handle:session:<id>` en premier ;
  après compaction, la consigne d'identité ; sans fiche, la demande du handle ou de la fiche et
  **une seule lecture, aucune écriture** ; `RAG3WEAVER_AGENT_KEY` avec et sans fiche ;
  `UserPromptSubmit` rend les fiches en rapport, le diff d'un autre sur un fichier touché (pas le
  sien), le verdict du juge ; un prompt court sans rien à dire rend un bloc vide ; le premier tour
  après compaction lance le défi, le tour suivant non ; le contexte plein suggère de ranger une
  seule fois ; le bloc est borné et le dit ; les rubriques et les `fiche:` se lisent.
- `claude_code_transcript::tests` : §3.
- `scripts/test_hooks_claude_code.sh` : étendu à la cible par défaut (le dossier courant), à
  `--worktree` sur un dépôt d'essai (worktree créé sur sa branche, hooks dans son
  `settings.local.json`, commande imprimée, seconde pose idempotente) et aux exclusions
  (`--global` avec `--settings`).

## 5. Ce qui reste aux lots 3 et 4

Les écritures mécaniques (`via: hook`, `author` = la clé) : le L1 `handle` quand la clé est connue
par la variable et que la fiche existe ; le L1 `compaction` de `PostCompact` (`put_source` +
`put_apport` + `relier_apport` + `citer_source`) ; la fin de run de `Stop` (`POST /journal/run`,
genre `claude_code_turn`, le handle de session — route à ouvrir dans l'hôte, à 73) ; les consignes
du résumé de `PreCompact` ; `ranger` quand M9 arrive. Et, au second tour après compaction si
l'apport `identity` n'est pas là, le redemander **une fois** et le journaliser (c7).
