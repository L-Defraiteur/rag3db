# Le témoin de la fermeture entre deux appels d'un pas rougit par intermittence

- **État** : ouvert. Relevé par la session tty le 19 nov. 2026 ; pour archi (l'équipier).
- **Famille** : produit
- **Gravité** : faux rouge (instable). Il peut rendre rouge la lib d'une passe sans que le lot y soit pour rien.
- **Atteignable en service** : non (le témoin).
- **Touche rag3weaver** : oui (`src/daemon_host/teammate_tests.rs`, `a_backend_closes_between_two_tool_calls_of_a_step`).

## Ce que c'est

Le test rougit par intermittence, quand le module `daemon_host::teammate` tourne en parallèle, et passe seul.

- **Le matin du 19 novembre, au portable** (lib 3d706f709, `RAG3WEAVER_MOTEUR_ANCIEN=1`) : il rougit dans le module entier, sur `called Result::unwrap() on an Err value: Io(Os { code: 4, kind: Interrupted, message: "Interrupted system call" })`, à `teammate_tests.rs:634`. C'est la requête HTTP du témoin vers son hôte gardé (`agent_de_temoin().post(…).send(…).unwrap()`). Le module rejoué juste après était vert (46).
- **L'après-midi du 19 novembre, au portable** (lib 7c2b506ec, branche `cx-attente-chaine`, reposée sur master) : il rougit une fois dans le module entier (47 sur 48), puis il est vert seul deux fois, et vert dans le module rejoué (48). Le journal de ce rouge a été écrasé avant d'être lu : la panique n'est pas relevée.

## Ce qu'il faut pour le fermer

La requête du témoin ne doit pas mourir sur un `EINTR` : un signal reçu pendant l'appel système doit être repris, pas rendu en erreur. Il faut d'abord trouver d'où vient le signal dans le processus de test : un fil qui tue ou réveille un autre (`SIGCHLD` d'un enfant, un `pthread_kill`), ou le pilote du navigateur d'un test voisin. Le témoin, rouge d'abord : le module joué N fois en parallèle, sans un seul `Interrupted`.
