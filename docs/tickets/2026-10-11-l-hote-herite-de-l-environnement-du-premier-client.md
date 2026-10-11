# L'hôte du démon hérite de l'environnement du premier client qui le lance

- **état** : ouvert (11 octobre 2026, vu en redémarrant l'hôte de Lucie)
- **gravité** : réponse fausse ou blocage selon le backend (un service d'embarquement qui manque)
- **atteignable en service** : oui, dès qu'un hôte sert des backends qui demandent des services différents
- **touche rag3weaver** : oui (et seulement)

## Ce que c'est

Sans unité système, l'hôte est lancé par le premier client MCP qui ne le trouve pas
(marche 4, `daemon_lifecycle::ensure`), et il hérite de **son** environnement. L'hôte de
Lucie avait été lancé par le client `memoire-longue` : il n'avait que
`RAG3WEAVER_EMBED_SERVICE=127.0.0.1:7980` (bge-m3), alors qu'il servait aussi
`code-rag3db`, qui déclare granite-278m sur 7979. Le même hôte, lancé par l'autre client,
aurait eu l'inverse. Ce qu'un backend peut faire dépend donc de l'ordre dans lequel les
sessions ont démarré — et rien ne le dit.

## Ce qu'il faut pour le fermer

L'hôte a **sa** configuration, un fichier à côté de `daemon.json`
(`~/.local/share/rag3weaver/`) : services d'embarquement, adresse, dossier des bases,
`RAG3DB_ROOT`. Il la lit au démarrage, quel que soit le lanceur (client, unité, main).
L'environnement du client ne sert que de repli, et `/etat` dit d'où vient chaque
réglage. L'`EnvironmentFile` de l'unité utilisateur de G en est déjà la forme pour
l'unité ; le fichier de l'hôte la généralise. Témoin : deux backends aux services
différents, l'hôte lancé par l'un puis par l'autre ; les deux embarquent dans les deux
cas, et `/etat` nomme la source de chaque service.

Le côté hôte est à l'arbre principal (frontière de la page C) ; à prendre après ses lots.
