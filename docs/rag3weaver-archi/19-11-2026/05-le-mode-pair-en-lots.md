# Le mode pair d'Algareth, en lots — le jeudi 19 novembre 2026

Le but : qu'une tâche nommée de l'hôte (Algareth) soit **vue et jointe par les sessions Claude Code
comme l'une d'elles**. Elle apparaît dans `ListAgents`, reçoit un `SendMessage`, répond. Ses
messages s'affichent dans le terminal et l'IDE comme ceux de Claude Code. C'est la vision de Lucie
du 16 novembre (`docs/orchestration/visions/16-11-2026-algareth-pair-des-sessions-claude.md`) et le
ticket `tickets/2026-11-16-le-mode-pair-par-le-registre.md`.

Ce qu'on sait du bus vient de deux relevés : `16-11-2026/02-releve-de-la-trame-du-bus.md` (le
registre, la trame d'un message) et `18-11-2026/04-releve-de-l-avis-de-pause.md` (l'abonnement aux
avis de pause).

## Les règles qui tiennent sur tous les lots

- **Déclaré, pas codé en dur.** Le mode pair s'allume par le manifeste, `teammate.pair`, éteint
  par défaut. L'outil d'envoi est déclaré dans le gabarit comme les autres outils. Les textes vus
  par le modèle sont en anglais.
- **Un message reçu est une demande, jamais une permission.** Il entre dans la boîte de la tâche
  comme une ligne de plus. Il ne passe aucune commande et n'accorde rien à la porte.
- **Le registre de Lucie ne se touche qu'à son mot.** Les témoins jouent sur un registre et un
  dossier de sockets temporaires (`RAG3WEAVER_PAIR_SESSIONS_DIR`, `RAG3WEAVER_PAIR_SOCKS_DIR`). La
  première entrée réelle, dans `~/.claude/sessions`, attend un oui direct de Lucie (lot 6).
- **Rien ne reste derrière.** L'entrée et la socket disparaissent à la fin de la tâche, à l'arrêt
  de l'hôte, et au démarrage suivant si l'hôte est mort sans les retirer.

## Lot 1 — l'entrée du registre, tenue par l'hôte

- **Ce qu'il fait.** Pour une tâche dont le backend déclare `teammate.pair`, l'hôte écrit
  `<pid>.json` dans le dossier du registre, avec les clés relevées le 16 novembre :
  - `pid` et `procStart` (champ 22 de `/proc/<pid>/stat`) de l'hôte ;
  - `cwd` : le worktree de la tâche ;
  - `kind` « interactive », `entrypoint` « rag3weaver » ;
  - `name` : le nom de l'agent, avec `nameSource` « rag3weaver » ;
  - `peerProtocol` 1, `peerFeatures` vide (lot 5 pour `notify_idle`) ;
  - `messagingSocketPath` : la socket du lot 2 ;
  - `status` « busy » pendant un pas, « idle » sinon, avec `statusUpdatedAt` et `updatedAt`.

  L'écriture est atomique (fichier temporaire puis `rename`).
- **Une question ouverte, tranchée par le lot 6.** Le registre a un fichier par pid. Un hôte qui tient
  plusieurs tâches en mode pair aurait plusieurs agents pour un seul pid. Le lot 1 n'en admet qu'une
  par hôte, et refuse la seconde en le disant. Le lot 6 relève si `ListAgents` lit le nom du fichier
  ou le champ `pid`.
- **Témoin rouge.** Sur un registre temporaire :
  - l'entrée apparaît à l'ouverture de la tâche, avec ses clés et un `procStart` juste ;
  - `status` suit le pas ;
  - l'entrée disparaît à la fin de la tâche et à l'arrêt de l'hôte ;
  - une entrée laissée par un pid mort est retirée au démarrage, celle d'un pid vivant d'un autre
    programme ne l'est pas ;
  - une seconde tâche en mode pair est refusée.
- **Taille** : une demi-journée.

## Lot 2 — la socket, et la réception vers la boîte

- **Ce qu'il fait.** Une socket Unix en mode 600 dans le dossier des sockets, nommée
  `<pid>.sock`. Elle accepte les sondes (une connexion fermée sans un octet) sans rien écrire. Elle
  lit **une ligne JSON par connexion**, puis :
  - `type: "user"` : `message.content` (l'enveloppe `<cross-session-message …>` faite par
    l'émetteur) devient une ligne de la boîte. La ligne prend la forme proposée par tty : `from`
    « session », `peer` (le `from-name`), `reply_to_address` (le `from`), `msg_id`, `priority`, et
    le texte tel quel. Une ligne `next` réveille l'équipier comme une demande.
  - `type: "control"`, `action: "notify_when_idle"` : l'abonnement est gardé (`from`, `msg_id`).
    Rien n'est envoyé tant que le lot 5 n'est pas fait.
  - Une trame illisible, trop longue (64 Kio) ou d'un `msgV` inconnu est refusée et journalisée.
    Elle n'entre pas dans la boîte.
- **Témoin rouge.** Un faux émetteur écrit les trames relevées le 16 et le 18 novembre : la boîte a
  la ligne attendue, une sonde ne laisse rien, une trame trop longue est refusée, et un message ne
  passe aucune commande.
- **Taille** : un jour.

## Lot 3 — l'envoi, au nom de l'agent

- **Ce qu'il fait.** Un outil déclaré, `send_to_session {to, text}` :
  - `to` est un nom du registre (lu, jamais écrit) ou l'adresse `uds:` d'une ligne reçue ;
  - la trame a la forme relevée, avec l'enveloppe faite par l'émetteur :
    `<cross-session-message from="uds:<notre socket>" from-name="<agent>" from-mode="rag3weaver">` ;
  - une connexion par message, sans attendre de réponse ;
  - l'envoi s'écrit aussi dans la boîte, en sortie (`direction` « out »), pour que le
    superviseur le voie.
- **Témoin rouge.** Une fausse session (un registre et une socket temporaires) reçoit la trame
  exacte. Un nom inconnu est refusé en nommant les sessions présentes. Une socket qui ne répond pas
  est dite.
- **Taille** : une demi-journée.

## Lot 4 — l'affichage dans le terminal et dans l'IDE

- **Ce qu'il fait.** Les lignes `from: "session"` (entrée et sortie) s'affichent comme dans Claude
  Code : qui écrit, à qui, le texte. Le terminal est le chantier de tty (ses tickets du 16 novembre),
  sur la forme de ligne du lot 2. L'IDE suit.
- **Témoin rouge** : celui de tty pour le terminal, une vue de l'IDE pour l'autre.
- **Taille** : une demi-journée de mon côté (l'IDE), et ce que tty chiffre.

## Lot 5 — les avis de pause

- **Ce qu'il fait.** À la pause suivante (fin d'un pas sans pas suivant) ou à la sortie, l'hôte
  envoie l'avis à chaque abonné, puis oublie l'abonnement. `peerFeatures` déclare alors
  `notify_idle`.
- **Il attend un relevé.** La forme de l'avis n'est pas connue : le 18 novembre, aucune session n'en
  a émis. Il faut la relever en journalisant celui qu'une vraie session envoie à une socket
  d'Algareth abonnée (lot 6).
- **Taille** : une demi-journée après le relevé.

## Lot 6 — le premier essai réel, au mot de Lucie

- **Ce qu'il fait.** Avec un oui direct de Lucie, dans la session qui le fait :
  - une tâche nommée en mode pair sur l'hôte de service ;
  - son entrée dans `~/.claude/sessions` ;
  - c7 lui envoie un message, l'agent répond, et la réponse arrive chez c7 ;
  - le relevé de la question du lot 1 (le nom du fichier contre le champ `pid`) ;
  - celui de l'avis de pause, si c7 s'abonne ;
  - l'entrée retirée à la fin, vérifiée.
- **Taille** : une heure, plus le temps de Lucie.

## L'ordre et le total

Les lots 1, 2 et 3 s'enchaînent en pile. Le lot 4 suit le lot 2. Le lot 5 attend le relevé du lot 6.
Total de code : environ deux jours et demi, sans le terminal de tty.

Le bus n'a pas de clé côté Algareth. Les vraies sessions ont un fichier `<pid>.<empreinte>.key`, et
le 18 novembre un abonnement venu d'une socket sans clé n'a peut-être pas été servi. Si le lot 6
montre qu'une session refuse de parler à une entrée sans clé, il faudra relever ce fichier. Cela
demande un mot de Lucie à part, parce que c'est un secret de Claude Code.
