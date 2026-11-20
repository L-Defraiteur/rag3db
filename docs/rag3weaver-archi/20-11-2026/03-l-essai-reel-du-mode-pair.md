# L'essai réel du mode pair (lot 6) — le vendredi 20 novembre 2026, 17 h 54

Lucie a dit oui dans la session d'archi : « oui go je t'autorise explicitement ». C'est le lot 6 de
`19-11-2026/05-le-mode-pair-en-lots.md` : une vraie entrée dans `~/.claude/sessions`, un échange avec
une session Claude Code, le relevé de ce que lit le registre, puis le retrait.

## Le montage

- **L'hôte d'essai.** Un binaire bâti depuis master 467b1b2c7 (mode pair, lots 1 à 3), en release.
  Il tourne à part de l'hôte de service, sur le port 7991, avec son propre HOME. Il est pointé
  explicitement sur le vrai registre (`RAG3WEAVER_PAIR_SESSIONS_DIR=~/.claude/sessions`,
  `RAG3WEAVER_PAIR_SOCKS_DIR=/run/user/1000/cc-socks`).
- **La tâche.** Un dépôt jouet, des gabarits de master avec `teammate.pair: true`, et une tâche
  `algareth` (`agent_name: "algareth"`). Le modèle est Gemini sur Vertex.

## Ce qui est vu

1. **L'entrée se pose à l'ouverture de la tâche** : `~/.claude/sessions/505245.json`, le pid de
   l'hôte. Elle porte `name` « algareth », `nameSource` « rag3weaver », `status` « idle »,
   `peerProtocol` 1 et `peerFeatures` vide. Sa socket `/run/user/1000/cc-socks/505245.sock` est en
   mode 600.
2. **`ListAgents` la montre** : « algareth [e73f25] · interactive · idle ».
3. **L'échange marche dans les deux sens.**
   - `SendMessage` de la session d'archi arrive dans la boîte de la tâche : une demande de
     « session », avec `peer` « rag3weaver archi » et l'adresse de réponse de la session.
   - L'équipier répond par `send_to_session` à cette adresse, avec « Bonjour rag3weaver archi, ici
     algareth. 2 + 2 font 4. » La session le reçoit comme un message de
     `uds:/run/user/1000/cc-socks/505245.sock`, `from-name` « algareth », `from-mode` « rag3weaver ».
   - La demande se clôt d'elle-même (`fait sans plan`), et l'entrée revient à « idle ».
4. **L'avis de pause** : `notify_when_idle` est refusé par l'émetteur (« that session runs a version
   without idle notices »), puisque `peerFeatures` ne déclare pas `notify_idle`. Sa forme reste à
   relever (lot 5).
5. **Le registre lit le nom du fichier.** Renommée de `505245.json` en `505245-b.json`, l'entrée
   disparaît de `ListAgents`. Remise à son nom, elle revient. Un processus ne porte donc qu'une
   entrée, et qu'un nom.
6. **Le retrait est propre.** La tâche rejetée par l'hôte retire l'entrée et la socket. Après
   l'arrêt de l'hôte, le registre a retrouvé ses 34 entrées, et rien ne reste de 505245.

## Ce que cela donne pour l'invariant de Lucie

Plusieurs pairs sur un même hôte (une session de tty, une fenêtre de l'IDE, plusieurs équipiers)
ne peuvent pas être plusieurs entrées d'un seul pid, ni une socket qui porte plusieurs noms. **Il
faut un processus par pair** : un petit relais, enfant de l'hôte, qui tient l'entrée `<son
pid>.json` et sa socket, et qui passe les trames à l'hôte (sa boîte, par HTTP sous jeton). C'est le
lot « un client de l'hôte demande sa propre entrée nommée » : il naît par ce relais.

## Rangé après l'essai

Le worktree, le target (1,8 Go) et le dossier de l'essai sont retirés.
