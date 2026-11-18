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
   `src/claude_code_hook.rs:1666`. Ce fait a un seul rédacteur et ne change pas de place : la
   fiche de tâche cite le bilan. Tout est en `proposed`, dans la mémoire déclarée par
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

## Le branchement dans l'hôte, tranché avec archi

archi a répondu à 12 h 1x, après lecture du code de l'hôte. Le code de cette partie est à
archi : le déclencheur, la route, le découpage, l'appel de l'archiviste et la reprise du
contexte. La mémoire tient le gabarit de l'archiviste et le témoin.

- **(a) Où est la trace d'un run.** Un pas de l'équipier est un tour du chat de sa tâche.
  - Les tours sont persistés dans le `ChatFile` du backend (`src/daemon_host/chat.rs` : turns,
    `last_call` et la jauge), sous `RAG3WEAVER_CHATS_DIR` ou `~/.local/share/rag3weaver/chats`.
  - Les appels d'outils et leurs rendus entiers sont dans le journal des runs, un fichier par
    run (`journaliser_l_appel`).
  - **L'hôte** lit les deux et passe le markdown par sections à l'archiviste, qui ne lit pas le
    disque de l'hôte.
- **(b) Le déclencheur.**
  - Un seuil d'abord : la jauge a `used` et `max` par appel, et quand `used` dépasse 80 % de
    `max`, la compaction se lance avant l'appel suivant.
  - `ContextOverflow` reste le filet.
  - Si `max` est inconnu, seul le dépassement déclenche, et `absorb_under` garde son rôle
    d'aujourd'hui.
- **(c) Le `/compact` de tty.** C'est un appel à l'hôte,
  `POST /ide/chat/compact?backend=<backend de la tâche>`, qui rend la passe faite (fiches
  touchées, L1 `compaction`). Ce n'est pas un message dans le run : celui-ci passerait par le
  modèle et coûterait un tour au moment où le contexte est plein. La commande est refusée
  pendant un tour en vol, comme `POST /ide/chat`.
- **(d) Synchrone, avec une borne.** Au seuil ou sur la commande, le run attend la passe : il
  ne peut pas repartir au-dessus du plafond.
  - Au-delà de la borne (réglable, 120 s pour commencer), l'hôte retombe sur `absorb_under`
    (les renvois) pour ce tour.
  - La passe finit en fond, et le contexte se refait depuis les fiches au tour suivant. Le
    journal le dit.
- **(e) Le rangement au fil de l'eau** se fait en fond, à chaque section close.
  - Deux points ferment une section : la fin d'un tour de chat (l'échange utilisateur →
    réponse, avec ses appels d'outils) et la clôture d'un plan de l'équipier.
  - L'hôte découpe à ces deux points, puisqu'il tient les deux, et note dans le `ChatFile`
    « rangé jusqu'au tour N ».
  - La compaction ne range que ce qui reste depuis N : elle est courte.

Le témoin de la ligne 4 prend un cas de plus : **un échange déjà rangé ne se range pas deux
fois**. Il n'y a pas de seconde passe du Decide sur les tours déjà couverts par « rangé jusqu'au
tour N ».

## La référence à battre

Le mot de Lucie (par c7, à 12 h 3x) : le but est de **faire mieux que la compaction de Claude
Code**. C'est la référence à battre dans le banc : à budget égal, l'équipier compacté par
l'archiviste reprend sa tâche mieux qu'après le résumé de Claude Code.

## Le budget du contexte d'après

Une borne de Lucie (par c7, à 12 h 3x) : le contexte qui repart après la compaction vise
**environ 100 000 jetons** pour un modèle cloud, ce avec quoi Claude Code redémarre après sa
compression, et moins pour un modèle local.

- **Le réglage** est par modèle, dans le manifeste : `models.llm.restart_budget_tokens`, avec un
  défaut de 100 000 pour un modèle cloud.
- **La lecture** est faite par ED, dans son lot « models.llm en liste » (`ModelSource::budget_de_reprise()`) :
  - le champ s'il est déclaré, sur l'élément de liste comme sur l'objet unique ;
  - sinon 100 000 pour un modèle cloud ;
  - sinon, pour un modèle local, la moitié de sa fenêtre (`context_tokens / 2`), au plus
    100 000. Est local un modèle de provider `local`, de service à nous, ou de protocole
    `llama_server`.

  Le budget suit le modèle choisi par `/model`.
- **La composition**, par priorité, jusqu'au budget :
  1. l'identité ;
  2. la fiche de tâche ;
  3. le bilan ordonné ;
  4. les L1 de la session, les siens en entier, **dans l'ordre où ils ont été pris** (l'ordre
     des intervalles de tours), pas groupés par fiche ; la fiche de chaque L1 se dit en marge.
     Le bilan et les L1 racontent donc la même séance deux fois, à deux grains ;
  5. les L1 des autres sur les mêmes sujets, les plus récents d'abord.

  Ce qui ne tient pas reste au rappel.
- **Le témoin** mesure le contexte rendu et refuse au-dessus du budget. Il compte les jetons
  par le tokenizer du modèle, ou par une estimation qu'il nomme : sur le faux modèle, c'est
  `llm::fragments`, ce que `MockLlm` rend en `prompt_tokens`.

Le seuil (`compaction.ratio`, 0.8), la borne (`compaction.timeout_s`, 120) et le budget se
lisent dans `chat_state()["context"]["compaction"]`. Le témoin est
`tests/e2e_compaction_archiviste.rs`.

## Le réglage par défaut : S 50 (Lucie, 18 novembre 2026)

Lucie retient **S 50** pour la compaction par l'archiviste, contre F 25 %. Le banc le confirmera sur
sa F300 entière.

**Ce que c'est** (banc, `docs/banc/15-11-2026/02-jusqu-ou-l-archiviste-voit.md`, § 3 et 10) : des
fenêtres de **50 k jetons, sans chevauchement**. Chaque décision du Decide voit en plus de sa
section :

- sa place (« fenêtre n sur N, tours a-b ») ;
- les résumés des fenêtres d'avant (les L1 et le bilan composés par `compose_restart`) ;
- le bout de la fenêtre en cours qui précède la section.

Les sections restent par tour.

**Pourquoi.** Contre la N rejouée, S 50 tient le même rappel que F 25 % (0,84 contre 0,86), avec 14 %
de jetons en moins et un p90 plus court. La variante de consigne et la fenêtre de 300 k sont
écartées.

**Dans le code à venir** (le lot d'archi, puis le gabarit memory), c'est la valeur par défaut :

- la taille de la fenêtre, 50 000 jetons ;
- le chevauchement, 0 ;
- la consigne d'origine ;
- `context` d'`archive` porte la place, puis les résumés au budget, puis le bout de la fenêtre en
  cours.

Ces réglages vont au manifeste, avec ces défauts. Ce ne sont pas des constantes.

## Ce qui suit

1. Le mot de c7 sur cette page, puis Lucie, à qui c7 la montre.
2. Le témoin d'abord, rouge sur master, puis le code d'archi et le gabarit de la mémoire.
3. Le jugement (quoi ranger, quoi relire) se mesure au banc avant d'être tenu pour bon.
