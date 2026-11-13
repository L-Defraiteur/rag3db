# La relance d'une carte de l'atelier se dit partie quand le tour la refuse, et la décision se perd

- **État** : ouvert (13 novembre 2026), pour archi.
- **Famille** : produit
- **Gravité** : réponse fausse — l'IDE reçoit `relance: true`, le modèle n'apprend jamais la décision de l'utilisatrice
- **Atteignable en service** : oui, dès qu'une carte (accord, commande, chemin) est décidée pendant que le tour qui l'a posée tourne encore
- **Touche rag3weaver** : oui (`extension/rag3weaver/src/daemon_host/atelier_fichiers.rs` : `relancer_apres_la_carte` 305-317, 288 ; journal des décisions 70, 78-80, 279-286)
- **Pour** : rag3weaver archi (l'atelier qui lit et écrit, son auteur)
- **Signalé par** : everything declarative, relecture de l'atelier sur origin/master 73c7f15e1, à la demande de l'orchestration (13 novembre 2026) ; lignes de origin/master à cette date

## Ce que c'est

`relancer_apres_la_carte` rend `true` dès que le fil est lancé ; si `chat_turn` refuse ensuite (« un
tour est déjà en cours », `chat.rs` 472-473), le message part dans `eprintln` seulement. La doc de
la fonction (305-306) dit que ce cas « le laisse à l'utilisatrice », mais rien ne le lui dit. Même
famille : la décision est posée (70, 78-80) avant d'écrire `decisions.jsonl` (279-285) ; si le
journal échoue, la fonction rend `Err` alors que l'accord est en vigueur, sans trace ni publication.

## Le témoin à écrire

Une carte décidée pendant le tour qui l'a posée : la décision arrive au modèle au tour suivant (ou
est dite à l'IDE comme non relancée). Un journal des décisions en échec : ni accord en vigueur, ni
`Ok`.

## Ce qui le fermerait

Attendre la fin du tour ou mettre la relance en file, et rendre à l'IDE l'état vrai ; journaliser
avant d'ouvrir l'accord ; le témoin vert.
