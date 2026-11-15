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

La précision de Lucie (par c7, à 12 h 0x) : l'archiviste ne travaille pas seulement à la
compaction. Il range **au fil de la session**, et la compaction n'est que le moment où l'on
relit ce qu'il a déjà rangé.

1. **Ce que l'archiviste reçoit.** La trace du run : l'historique de l'agent, avec les
   résultats entiers que garde la `Session`, rendu en markdown et découpé en sections. Une
   section correspond à un intervalle de tours : un échange (message reçu, réponse, appels
   d'outils et leurs résultats), ou un plan clos. Il la reçoit **au fil de l'eau**, à chaque
   section close, et pas seulement au plafond. Le sujet de départ est l'identité `agent:<clé>`.
   À la compaction, **seuls les tours qu'aucun L1 ne couvre encore** passent par le Decide :
   ce qui est rangé ne se retraite pas.
2. **Ce qu'il rend.** C'est `archive.mmd` tel quel : rejoindre, créer par sujet, ou rien.
   Chaque L1 posé dit **quel bout de conversation il couvre** : sa source est la transcription
   du run, sa section l'intervalle de tours. C'est déjà la forme de l'identité d'un Apport
   `{fiche, source, section}`, donc aucun champ neuf. S'y ajoutent un L1 `compaction` qui cite
   le run, un L1 `plan` par plan clos, et **le bilan** : un résumé très court de l'ordre dans
   lequel les choses ont été faites. C'est une liste ordonnée de pas, une ligne par pas, sans
   texte, chaque pas citant `fiche:<nom>` du sujet ou l'apport (le L1) qui le porte. Il est
   écrit à la compaction, ou tenu au fil de l'eau. Il prend la forme qu'il a déjà chez les
   hooks : l'apport de section `bilan` de la fiche `bilan : <tâche>`, que relit
   `src/claude_code_hook.rs:1666`. Tout est en `proposed`, dans la mémoire déclarée par
   le run.
3. **Ce que le run relit au redémarrage.** L'historique est vidé. Le run repart de l'identité
   `agent:<clé>`, de la tâche courante (sa fiche), **du bilan** (l'ordre des pas de la session précédente,
   chaque pas avec ses références, pour que l'agent se rappelle l'ordre, pas seulement les
   sujets) et des **L1 déjà posés, pas du texte**. Deux
   bornes tiennent :
   - **quels sujets rouvrir** : ceux où ses L1 viennent d'être posés ;
   - **combien de L1** : les siens en entier, ceux des autres bornés.

   Les tours restés non couverts (le dernier échange en cours) suivent en entier. Les renvois
   `#outil-n` restent résolubles par la `Session`.
4. **Le témoin, rouge d'abord.** Un e2e joue sur le faux modèle (`tests/common`, `fake_sse`)
   avec une fenêtre petite. Il attend cinq choses :
   - des L1 posés pendant le run, avant le plafond, avec leur section = l'intervalle de tours ;
   - au plafond, le Decide n'a reçu que les tours non couverts ;
   - l'invite qui suit est sous le plafond et porte l'identité et ces L1 ;
   - le L1 `compaction` est écrit ;
   - le bilan liste les pas dans l'ordre du run, une ligne chacun, chacun citant sa fiche ou
     son apport, et l'invite d'après le porte.

   Il doit être rouge sur master, où seule la réduction aux renvois a lieu.

**Le jugement se mesure avant de se croire.** La forme est posée ici. « Faut trouver
l'intelligence, c'est tout le but de notre recherche » (Lucie) : quoi ranger, où, et quoi
relire se mesurent au banc, avec Laya et Gemini, avant d'être tenus pour bons.

## À trancher avec archi

- (a) Où l'hôte tient la trace d'un run, et qui la donne à l'archiviste.
- (b) Le déclencheur : `ContextOverflow` seul, ou un seuil avant, par exemple une part de la
  fenêtre.
- (c) Le `/compact` de tty : un appel à l'hôte, ou un message dans le run.
- (e) Le rangement au fil de l'eau : à chaque section close (fin d'échange, plan clos), en fond, ou par lots de N tours.
- (d) La passe en synchrone (le run attend), ou en fond, le run repartant d'abord sur les
  renvois seuls.
