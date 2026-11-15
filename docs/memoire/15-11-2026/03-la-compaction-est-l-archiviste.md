# La compaction est l'archiviste

15 novembre 2026, session mémoire (96), avec archi pour le branchement dans l'hôte. C'est une
page de conception : rien n'est codé avant qu'elle soit relue.

Le rappel vient de Lucie (par c7, à 12 h) : quand le contexte d'un équipier se compacte (par le
`/compact` de tty ou par le plafond de contexte dans l'hôte), ce n'est pas un résumé de plus,
c'est **une passe de l'archiviste** sur la conversation du run.

## Ce qui existe sur master (37ea4c439)

- `Agent::absorb_history` et `Session::absorb_under` (`src/agent.rs`, `src/session.rs`)
  réduisent les résultats d'outils à leur renvoi `#outil-n`. Le contenu entier est gardé dans la
  `Session`. Sur `LlmError::ContextOverflow`, la boucle serre la politique à `Stale { 0, 0 }` et
  réessaie. Rien n'est résumé ni rangé.
- tty n'a pas de `/compact`.
- Le hook Claude Code a déjà la forme voulue, côté Claude Code : `PreCompact` donne les
  consignes du résumé, `PostCompact` range le L1 `compaction`, et `SessionStart(compact)` relit
  l'identité et les fiches rappelées (`src/claude_code_hook.rs`).
- L'archiviste existe : `archive.mmd` découpe en sections, juge chaque section (rejoindre une
  fiche, créer par sujet, rien), et écrit les L1 en `proposed`.

## La conception en quatre lignes

1. **Ce que l'archiviste reçoit.** La trace du run, de l'historique de l'agent, avec les
   résultats entiers que garde la `Session`. Elle est rendue en markdown et découpée en
   sections : une section par échange (message reçu, réponse, appels d'outils et leurs
   résultats), une section par plan clos. Le sujet de départ est l'identité `agent:<clé>`.
2. **Ce qu'il rend.** C'est `archive.mmd` tel quel : rejoindre, créer par sujet, ou rien, avec
   les L1 sous les fiches. S'y ajoutent un L1 `compaction` qui cite le run et ce qui a été
   coupé, et un L1 `plan` par plan clos. Tout est écrit en `proposed`, dans la mémoire déclarée
   par le run.
3. **Ce que le run relit au redémarrage.** L'historique est vidé. Le run repart d'un bloc fait
   de l'identité `agent:<clé>`, de la tâche courante (sa fiche), du rappel (les fiches touchées
   par la passe, par leur nom, à relire par `get_fiche`) et du dernier échange en entier. Les
   renvois `#outil-n` restent résolubles par la `Session`.
4. **Le témoin, rouge d'abord.** Un e2e joue sur le faux modèle (`tests/common`, `fake_sse`)
   avec une fenêtre petite qui fait atteindre le plafond. Il attend trois choses : la passe de
   l'archiviste a écrit le L1 `compaction`, l'invite qui suit est sous le plafond, et l'identité
   y est. Il doit être rouge sur master, où seule la réduction aux renvois a lieu.

## À trancher avec archi

- (a) Où l'hôte tient la trace d'un run, et qui la donne à l'archiviste.
- (b) Le déclencheur : `ContextOverflow` seul, ou un seuil avant, par exemple une part de la
  fenêtre.
- (c) Le `/compact` de tty : un appel à l'hôte, ou un message dans le run.
- (d) La passe en synchrone (le run attend), ou en fond, le run repartant d'abord sur les
  renvois seuls.
