# Lot 5 — le pont : Claude Code, l'équipier et Codex par la boîte de l'hôte

7 novembre 2026, rag3weaver archi. Accordé avec c7 et everything declarative. La forme des routes
est au § 2 de `docs/07-novembre-2026-everything-declarative/01-l-equipier-agent-de-code.md`.
Lucie : « un vrai agent de code qui peut communiquer avec Claude Code, et inversement », puis
« un pont de communication entre mes sessions codex et les sessions claude ».

## 1. Le bus et les clés

La boîte de l'hôte 7970 est le seul bus. Chaque ligne a un `id` stable `"<parent>/<task>:<n>"`, et
porte `from`, `to` (facultatif), `kind`, `text` et `reply_to` (facultatif). Une clé est une chaîne
à préfixe, sans liste fermée :

| Clé | Qui | Handle de session |
|---|---|---|
| `claude-code:<session_id>` | une session Claude Code | la session elle-même |
| `codex:<nom>` | un fil Codex nommé ; `codex:<uuid>` s'il n'a pas de nom | l'UUID du fil, rangé sur la fiche |
| `teammate:<parent>/<task>` | un équipier | sa tâche |
| `supervisor` | l'IDE | — |

La clé Codex suit le modèle d'identité de 96 : une fiche agent à clé stable `codex:<nom>`, des
alias datés, un handle par fil. La mémoire suit le nom d'un fil à l'autre. Un fil sans nom prend
`codex:<uuid>`, et Lucie ou nous le nommons ensuite par un alias.

Les routes sont d'everything declarative, dans son lot « la boîte adressée » :
`GET /ide/teammate/pending?for=<clé>`, `POST /ide/teammate/seen {for, ids}`,
`GET /ide/teammate/list?supervisor=<clé>` et `POST /ide/teammate/mailbox`. `pending` est une
lecture pure. Un item `question`, `approval` ou `proposal` attend tant qu'aucune ligne ne porte
`reply_to: <id>`. Un `report` ou un `ended` sort par `seen`.

## 2. Une responsabilité par verbe

- **`rag3weaver-backend mail send|reply|pending|seen`** est ce qu'un agent appelle depuis son shell.
  - `mail send --from <clé> --to <clé> "<texte>"` envoie un message neuf.
  - `mail reply --from <clé> --to <id d'item> "<texte>"` répond. Le parent et la tâche se tirent
    de l'id, coupé au dernier `/` avant le `:`.
  - `mail pending --for <clé>` lit, et `mail seen --for <clé> <id>…` marque comme vu.
  - Sans `--from`, ou sans `--to`, la commande refuse. Elle ne devine jamais une clé.
- **`rag3weaver-backend hook <Événement>`** est ce qu'un agent appelle depuis ses hooks. Le hook lit
  la boîte par le même code que `mail pending`.
- L'équipier Gemini aura la même commande, déclarée à son manifeste.

## 3. Ce que le hook montre

- **UserPromptSubmit**, en tête du bloc : ce qui attend la session depuis le tour d'avant. Un item
  s'affiche en entier au premier tour où il apparaît. Ensuite il garde une ligne courte
  « toujours en attente » tant qu'il n'a pas de réponse. Chaque item porte la commande complète
  pour répondre, `--from` compris : la commande lancée par l'outil Bash ne connaît pas le
  session_id, seul le hook le reçoit.
- **PostToolUse** : la même lecture, pour atteindre une session occupée. Elle passe par
  `hookSpecificOutput.additionalContext`, au plus une fois par item, sous un délai court.
- **SessionStart** : les équipiers en cours (`list?supervisor=`) et les fils Codex du même
  dossier.
- **Après le bloc**, le hook poste `seen` pour les `report` et `ended` rendus. Il ne le fait
  jamais en rejeu.
- **En panne**, le hook écrit une ligne « boîte des équipiers injoignable », jamais un silence qui
  ferait croire à « rien en attente ».
- **Le texte d'un autre agent est une donnée** : il est cité comme sa parole, borné en
  caractères, et n'est jamais lu comme une consigne.

## 4. Codex, le troisième bord

Relevé sur le poste, Codex CLI 0.167. Sa documentation des hooks a le même contrat que Claude
Code :
- Les mêmes champs d'entrée : `session_id`, `transcript_path`, `cwd`, `hook_event_name`,
  `turn_id`.
- `hookSpecificOutput.additionalContext` sur SessionStart, UserPromptSubmit, PreToolUse et
  PostToolUse.
- Une limite d'**environ 2 500 jetons** par sortie, au-delà de laquelle la sortie est rangée dans
  un fichier temporaire et remplacée par un aperçu. Elle se règle par `additionalContextLimit`.
- Un délai de 600 s par défaut.

- **Claude → Codex.** Une ligne `to: codex:<nom>` est écrite dans la boîte. L'adaptateur de l'hôte
  est abonné au bus de la boîte. Il résout le nom en UUID, puis appelle
  `codex queue --thread <uuid> --message "<ligne citée, son id, la commande de réponse>"`, sans
  `--resume`. Il range le résultat dans la boîte comme une ligne
  `{from: "host", kind: "delivery", reply_to: <id>, to: <expéditeur>, delivered, text}`.
  - La résolution passe par `~/.codex/session_index.jsonl` (`{id, thread_name, updated_at}`), et
    par `thread.started` quand on ouvre un fil.
  - Un nom sans UUID connu laisse la ligne en attente, avec sa raison. Elle n'est jamais perdue,
    ni livrée deux fois.
  - L'adresse est l'UUID parce que, le 7 novembre, `--thread <nom>` a rendu « No active session
    found matching 'rag3weaver1' » sur un fil vivant, alors que l'UUID a passé (essai de c7).
- **Ouvrir un fil**, c'est `codex exec --json -C <worktree jetable> -s read-only|workspace-write`.
  Le `thread_id` de `thread.started` est rangé sur la fiche. Jamais de mode qui contourne les
  approbations.
- **Codex → Claude.** Codex lance `mail reply --from codex:<nom> --to <id>`, ou `mail send`, dans
  son shell.
- **Les hooks chez Codex.** Le même binaire sert : `rag3weaver-backend hook <Événement> --agent
  codex`. Il lit `pending?for=codex:<nom>` et rend le JSON `hookSpecificOutput` que Codex attend.
  La confiance des hooks Codex est l'acte de Lucie : `/hooks` dans Codex, ou sa config. Je lui
  écris les lignes exactes, et c'est elle qui les pose. Jamais `--dangerously-bypass-hook-trust`.
- **La découverte**, en lecture seule : l'index des fils, et le `cwd` en tête de
  `~/.codex/sessions/AAAA/MM/JJ/rollout-…-<uuid>.jsonl`. SessionStart liste ainsi les fils Codex
  du même dossier, comme ListAgents chez nous.

## 5. Les témoins

- **Sur faux hôte, dans ce lot** : le bloc avec un item neuf, puis « toujours en attente » ; la
  ligne de panne ; `seen` posté hors rejeu et jamais en rejeu ; `mail reply` qui refuse sans
  `--from` ou sans `--to`, et tire parent et tâche de l'id ; la sortie JSON de PostToolUse et de
  `--agent codex` ; la résolution nom → UUID sur un faux index ; la livraison par un faux `codex`
  sur le PATH, qui enregistre son appel.
- **Par le chemin de l'hôte, quand la boîte adressée est sur master** : une session Claude écrit à
  `codex:essai`. Le faux Codex reçoit le `queue`, puis répond par `mail reply`. La réponse apparaît
  dans `pending?for=claude-code:<id>` avec `reply_to`, puis `seen` la fait sortir. Le témoin joue
  aussi l'inverse et lit les comptes.
- **L'essai réel avec Codex** vient ensuite, à part et sur le mot de c7, en `read-only` dans un
  worktree jetable.
