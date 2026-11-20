# Le mode pair : Algareth vu et joint par les sessions Claude Code

- **État** : ouvert, le 16 novembre 2026 ; à faire par Codex (passation d'archi).
- **Famille** : produit
- **Pour** : archi (passation à Codex)
- **Gravité** : fonction manquante, voulue par Lucie (vision du 16 novembre, « Algareth pair des sessions Claude »).
- **Atteignable en service** : non, à construire.
- **Touche rag3weaver** : oui
- **Signalé par** : Lucie et c7, le 16 novembre 2026

## Où il en est, le 20 novembre 2026

- **Faits** :
  - lot 1, l'entrée du registre ;
  - lot 2, la socket, la réception et la durée de vie ;
  - lot 3, `send_to_session` ;
  - lot 4, l'affichage dans l'IDE ;
  - lot 6, l'essai réel, au oui de Lucie (page `20-11-2026/03-l-essai-reel-du-mode-pair.md`).
- **Tranché par le lot 6** : le registre de Claude Code lit le nom du fichier `<pid>.json`. Un
  processus ne porte qu'une entrée et qu'un nom.
- **Reste ouvert, lot 5** : la forme de l'avis de pause. `peerFeatures` est vide, donc l'émetteur
  refuse l'abonnement. Il faut la relever en journalisant l'avis qu'une vraie session envoie, avant
  de déclarer `notify_idle`.
- **Vient ensuite** (c7, invariant de Lucie) : un relais par pair, un petit processus enfant de
  l'hôte, qui tient son `<pid>.json` et sa socket, et passe les trames à l'hôte par HTTP sous jeton.
  Il sert à une session de tty, une fenêtre de l'IDE ou un équipier nommé. Témoins rouges :
  - deux pairs du même hôte sont visibles sous leurs noms ;
  - un message à chacun arrive dans la bonne boîte ;
  - la fermeture ne laisse ni fichier ni socket.

## Ce qui est su

Relevé d'archi du 16 novembre, sans lire les sessions vivantes : la découverte de ListAgents passe
par un registre, `~/.claude/sessions/<pid>.json`, et non par les sockets de `/run/user/1000/cc-socks/`.
Une socket seule n'apparaît pas, et personne ne la sonde. Clés d'une entrée : `pid`, `sessionId`,
`cwd`, `startedAt`, `procStart`, `version`, `peerProtocol` (1), `peerFeatures`, `kind`, `entrypoint`,
`pidDomain`, `messagingSocketPath`, `name`, `nameSource`, `nameSince`, `status`, `statusUpdatedAt`,
`updatedAt`. **La trame du protocole 1 n'est pas encore relevée.**

## Ce qu'il faut faire

1. **Le relevé de la trame**, avec le mot de Lucie (donné le 16 novembre pour une entrée témoin) :
   une entrée `algareth-temoin` sobre et vraie (`nameSource: "rag3weaver"`, pid et `procStart` d'un
   processus qui tient une socket journalisante, `status` vrai), un message de c7, la trame et
   l'accusé relevés, puis l'entrée et la socket retirées. Une page de relevé avec les trames.
2. **La conception du mode `pair` de l'hôte** : l'entrée du registre tenue par l'hôte pour chaque
   tâche nommée, une socket au protocole 1, la réception vers la boîte de la tâche (une demande
   passe par la porte, jamais une permission), l'émission au nom de l'agent, le retrait propre à la
   fermeture. La forme de ligne de boîte de tty (`from: "session"`, `peer`, `direction`, `delivery`)
   est la proposition de départ.
3. Les canaux de Claude Code : une page de préparation seulement, pour une relance future des
   sessions (pas maintenant : Lucie ne veut pas redémarrer les sessions).

## Comment le prouver

Le relevé est une page ; le mode `pair` a ses témoins rouges dans sa conception, à écrire.

## Taille et blocage

Relevé : 2 h. Conception : 4 h. Code : 3 jours. Toute écriture dans `~/.claude/sessions` attend une
confirmation directe de Lucie à chaque essai.

