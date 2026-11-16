# Relevé de la trame du bus entre sessions Claude Code — le lundi 16 novembre 2026

Ce relevé est la partie 2 du relevé prévu par le ticket `2026-11-16-le-mode-pair-par-le-registre.md`.
Lucie avait dit oui à une seule entrée témoin, `algareth-temoin`, le temps d'un essai, puis retirée.
Claude Code était en version 2.1.322.

## Le montage

- **La socket.** Le script témoin `temoin.py` (pid 2148085) écoute sur
  `/run/user/1000/cc-socks/2148085.sock` (mode 600). Il journalise chaque connexion et chaque octet
  reçu, et ne répond rien.
- **L'entrée au registre.** Elle était posée dans `~/.claude/sessions/2148085.json` avec :
  - `pid` et `procStart` : ceux du processus de la socket (`procStart` est le champ 22 de
    `/proc/<pid>/stat`, comme pour les vraies entrées) ;
  - `pidDomain` : repris d'une vraie entrée ;
  - `cwd` : le dépôt ;
  - `kind` « interactive », `entrypoint` « cli », `status` « idle » ;
  - `name` « algareth-temoin », `nameSource` « rag3weaver » ;
  - `peerProtocol` 1, `peerFeatures` vide ;
  - `messagingSocketPath` : la socket témoin.
- **Pas de clé.** Les vraies sessions ont aussi un fichier `<pid>.<empreinte>.key` (600, 141
  octets). Le témoin n'en a pas eu. Ces fichiers n'ont pas été lus.
- **Les envois.** c7 a envoyé deux messages par `SendMessage` :
  - un message simple ;
  - un second avec `notify_when_idle: true`.
- **Le retrait.** Ensuite, l'entrée est retirée et le processus arrêté. SIGTERM laisse le fichier de
  socket : il est retiré à la main.

## Ce qui est vu

1. **Le listing ne demande que le registre.** `ListAgents` a montré « algareth-temoin [30ea0f] ·
   interactive · idle · started 1s ago » dès la pose, sans fichier `.key`. Le nom nu a suffi à
   `SendMessage`.
2. **Des sondes de vie.** Avant tout message, la socket reçoit des connexions qui se ferment sans un
   octet. Il y en a eu huit en une minute : les autres sessions vérifient que la socket répond.
   Une socket qui n'accepte pas fait sans doute disparaître l'entrée du listing (non vérifié).
3. **La trame.** Un message est **une ligne JSON terminée par `\n`, sur une connexion à lui**.
   L'émetteur se connecte, écrit, puis ferme. Il n'attend aucune réponse sur la socket, et il n'y a
   ni en-tête de longueur ni poignée de main. Les clés de la trame :

   ```json
   {"msgV":1,"msg_id":"<uuid>","type":"user",
    "message":{"role":"user","content":"<cross-session-message from=\"uds:/run/user/1000/cc-socks/<pid émetteur>.sock\" from-name=\"rag3db-5a\" from-mode=\"prompting\">\n<texte>\n</cross-session-message>"},
    "priority":"next","from":"uds:/run/user/1000/cc-socks/<pid émetteur>.sock"}
   ```

   - **L'enveloppe est faite par l'émetteur** : `<cross-session-message …>` arrive déjà dans
     `content`, avec `from-name` et `from-mode`. Le receveur n'a rien à composer. Il la passe telle
     quelle comme un tour utilisateur.
   - **`from`** est l'adresse de réponse : la socket de l'émetteur.
   - **`priority` « next »** : le message est servi au prochain tour d'outil du receveur.
   - **`msgV` 1** : la version de la trame. C'est un autre champ que `peerProtocol` du registre.
   - Les deux trames font 423 et 446 octets.
4. **Il n'y a pas d'accusé.** Côté émetteur, le retour est « in that session's inbox, not yet read
   by its Claude » dès que l'écriture réussit. Aucune notice de livraison (garde ou refus) n'est
   revenue. Cela suggère que la garde et le refus sont des messages que le receveur envoie lui-même
   vers la socket `from`. Ce point est à confirmer.
5. **Les avis de pause se lisent dans `peerFeatures`.** Le second envoi est parti, mais l'abonnement
   a été refusé par l'émetteur, avant tout octet : « notify_when_idle: that session runs a version
   without idle notices — nothing was subscribed ». Le seul écart avec une vraie entrée est
   `peerFeatures` vide (les vraies déclarent `notify_idle`, `reply_across_default_dirs`,
   `artifact_yield`). L'émetteur lit donc cette capacité dans le registre. La forme de l'avis
   lui-même (une trame du receveur vers `from`, ou le champ `status` du registre lu par l'émetteur)
   reste à relever.

## Ce que cela donne pour le mode pair d'Algareth

Pour recevoir, il suffit :

- d'une socket d'écoute dans `cc-socks` qui accepte les sondes ;
- de lire une ligne JSON par connexion ;
- de passer `message.content` au tour suivant d'Algareth.

Pour répondre, Algareth écrit une trame de même forme vers la socket `from`, avec sa propre socket
en `from`. Cette forme n'est pas encore vérifiée dans ce sens. L'entrée réelle d'Algareth tiendra
`status` (busy, idle) à jour, et ne déclarera `notify_idle` que si elle envoie les avis de pause.
Ces deux points sont la suite du relevé :

- la trame d'un avis de pause ;
- la trame d'une notice de garde ou de refus.

Chacun se relève par une seconde entrée témoin qui déclare `notify_idle`. Cette entrée demande un
nouveau mot de Lucie : son oui couvrait un essai, et cet essai est fini.

Le journal brut des connexions est hors du dépôt : il contient le texte des messages.
