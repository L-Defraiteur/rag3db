# Sans service d'embarquement, l'hôte, `mcp` autonome et `serve` refusent un gabarit à vecteurs

- **État** : ouvert (11 octobre 2026, optimiseur)
- **Gravité** : blocage (le backend ne s'ouvre pas)
- **Atteignable en service** : oui, dès qu'un gabarit déclare du vecteur et qu'aucun service
  d'embarquement n'est donné, ce qui est le cas du gabarit de code du paquet npm
- **Touche rag3weaver** : oui, et seulement (`src/backend.rs`, `src/daemon_host.rs`,
  `src/bin/rag3weaver-backend.rs`)

## Ce que c'est

Un backend qui déclare du vecteur et ne trouve aucun service d'embarquement doit s'ouvrir en
plein texte seul, en le disant, avec les vecteurs en dette. C'est la règle depuis les défauts du
10 octobre : l'embarqueur est optionnel. Sur master, `PreparedBackend::open` refuse au contraire
un tel backend. Et trois portes sur quatre refusent avant même d'y arriver, en propageant
l'erreur de `connect_embedder` : l'hôte du démon (`open_on_disk`), `mcp` autonome et `serve`.
La porte qui s'ouvre sans service, avec l'embarqueur absent, n'existe que sur la branche
`paquet-npm`, et seulement pour les lignes JSON.

## La recette

Le gabarit `templates/backends/code`, un dossier d'une ligne, aucune des deux variables
`RAG3WEAVER_EMBED_SERVICE` et `RAG3WEAVER_SERVICE_EMBED`, puis :

```bash
RAG3WEAVER_DAEMON_DIR=$(mktemp -d) RAG3WEAVER_DAEMON_ADDRESS=127.0.0.1:0 \
  rag3weaver-backend mcp --manifest backend.json
```

Le client lance l'hôte, puis l'hôte refuse d'ouvrir le backend :

```
[mcp] hôte injoignable : l'hôte http://127.0.0.1:35729 n'ouvre pas ce backend : models.embed
(granite-278m) : ni `address` dans la déclaration, ni RAG3WEAVER_SERVICE_EMBED /
RAG3WEAVER_EMBED_SERVICE dans l'environnement — seul l'outil daemon_status est exposé
```

Vu par le témoin « MCP de zéro » de l'archive npm `0.0.1-alpha.3`, l'archive installée dans
un dossier vierge (`bindings/nodejs/test/mcp_de_zero.sh` sur `paquet-npm`).

## Le témoin

`tests/e2e_mcp_cycle_de_vie.rs`, cas 8, `un_backend_de_code_sans_service_s_ouvre_dans_l_hote` :
le gabarit de code livré, sans service, ouvert dans l'hôte lancé par le client. `list_files`
voit le fichier, `search_code` répond par les mots, `daemon_status` montre l'hôte en marche, et
le journal de l'hôte dit qu'il ouvre sans service. Le client du témoin retire les deux
variables du service, parce que l'hôte hérite de l'environnement du client qui le lance (voir
le ticket `2026-10-11-l-hote-herite-de-l-environnement-du-premier-client.md`).

## La cause

Deux endroits. `PreparedBackend::open` refuse en le nommant, et pose un embarqueur factice
quand rien n'est déclaré. Chaque porte appelle `connect_embedder()?`, et l'erreur remonte
avant `open`.

## Ce qu'il faut pour le fermer

- Porter sur master la porte de `paquet-npm` (13821a7d2 et 6788935a7) : `open` sans service
  pose l'embarqueur absent, jamais un factice. Il garde la phrase et la redit dans le reçu,
  `describe` et `index_state` sous `warnings`.
- Une seule méthode pour les quatre portes, `PreparedBackend::embedder_or_absent`.
- Le reçu de l'hôte (`open`, `describe`) porte aussi l'avertissement.
- Le cas 8 de l'e2e vert.
