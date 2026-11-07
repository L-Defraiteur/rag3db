# Une recherche continue sur l'hôte après l'abandon de son client

- **État** : ouvert (7 novembre 2026), sans porteur. Relevé par rag3weaver archi au 3 ter des hooks.
- **Gravité** : coût. L'hôte brûle du CPU pour une réponse que personne n'attend plus.
- **Atteignable en service** : oui. Un client à délai court, le hook UserPromptSubmit à 1 à 3 s,
  sur une requête lente, le plein texte flou d'un long texte.
- **Touche rag3weaver** : l'hôte (`daemon_host`, le service de `/call`) et l'exécution d'un graphe.
- **Famille** : produit

## Ce que c'est

Le hook appelle `recall_fiches` sous un délai. Le délai dépassé, le client ferme sa connexion et
passe à la suite. L'hôte ne le voit pas : le graphe `search_structured` va jusqu'au bout.
Relevé au journal des runs de l'hôte en service, backend `c6dd17bae70f`, le 7 novembre :

| heure | délai du hook | nœud `bm25` |
|---|---|---|
| 17:07:39 | 3 s | 32 936 ms |
| 17:08:00 | 3 s | 13 158 ms |

Le run de 17:07:39 a tourné environ 30 s après que le hook a rendu la main.

## Ce qui est fait ailleurs

- **Côté hook (3 ter, archi)** : une requête de mots-clés tirée du prompt. Le plein texte retombe
  à quelques dizaines de millisecondes, et le cas devient rare.
- **Côté moteur (recherche)** : `docs/tickets/2026-11-07-le-plein-texte-explose-sur-une-requete-longue.md`,
  une borne du nombre de termes, mesurée.

## Ce qui reste

L'hôte devrait annuler un appel dont le client est parti : la connexion fermée se voit à
l'écriture de la réponse, trop tard. Elle se voit plus tôt si le service sonde le socket entre
deux nœuds du graphe, ou si l'appel porte un délai que l'hôte applique lui-même, un `deadline_ms`
dans le corps de `/call`. Le moteur sait déjà interrompre une requête
(`Connection::interrupt`, `setQueryTimeOut`). Reste à brancher l'un ou l'autre sur l'exécution du
graphe.
