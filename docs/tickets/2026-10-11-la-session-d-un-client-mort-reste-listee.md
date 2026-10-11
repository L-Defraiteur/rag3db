# La session d'un client mort reste listée par l'hôte du démon

- **état** : ouvert (11 octobre 2026, relevé par l'orchestration en jouant la marche 4 sur la base de Lucie)
- **gravité** : P4, réponse fausse (l'état dit une session qui n'existe plus) ; et une session qui ne se ferme jamais
- **atteignable en service** : oui, dès qu'un client MCP meurt sans fermer sa session (tué, ou la sonde de `claude mcp get`)
- **touche rag3weaver** : oui (et seulement)

## Ce que c'est

Un client mince ferme sa session en partant (`DELETE /session`). Un client qui meurt
sans partir — tué par SIGKILL, ou la sonde de `claude mcp get` qui lance le serveur puis
l'abandonne — laisse sa session à l'hôte. 40 s plus tard, `daemon_status` la montre
encore, ouverte (« mcp (pid 3306354) »), alors que les clients sortis proprement ont
disparu. L'hôte n'a aucun moyen de le savoir : il ne connaît pas le pid du client, seulement
un libellé.

## Ce qu'il faut pour le fermer

Le client donne son pid en ouvrant sa session ; l'hôte, sur la même machine (il n'écoute
que la boucle locale), ferme une session dont le pid ne vit plus, à chaque lecture de
l'état et à un pas régulier, et l'état compte ce qu'il a fermé ainsi. Témoin : un client
tué par SIGKILL, puis `daemon_status` d'un autre client qui ne le montre plus.
