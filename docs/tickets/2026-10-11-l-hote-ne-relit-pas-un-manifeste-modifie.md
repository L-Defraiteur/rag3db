# L'hôte ne relit pas un manifeste modifié, et aucune route ne ferme un backend

- **État** : ouvert (11 octobre 2026) — trouvé en montant le MCP du code pour rag3db (session codeparsers)
- **Gravité** : réponse trompeuse (l'outil répond selon un manifeste qui n'est plus celui du fichier, sans le dire)
- **Atteignable en service** : oui, à chaque correction d'un manifeste servi par un hôte qui tourne
- **Touche rag3weaver** : oui (`daemon_host.rs`)

## Ce que c'est

L'hôte charge un backend par chemin de manifeste à la première demande, puis le garde.
Si on corrige ensuite le manifeste, l'hôte sert toujours le premier état : le 11 octobre,
`estimate` rendait encore l'ancien `workspace` après trois corrections. Rien ne dit que le
backend servi n'est plus celui du fichier, et aucune route ne ferme un backend.

## Ce que j'ai dû faire à côté

Changer de chemin de manifeste (`backend.json` puis `rag3db.json`), **et** de base, sinon
l'hôte aurait ouvert deux backends sur la même base dans un même processus. L'ancien backend
reste ouvert, avec sa base vide, jusqu'au prochain redémarrage de l'hôte.

## Le backend abandonné reste ouvert

Conséquence directe, constatée le 11 octobre : le backend du premier manifeste
(`~/.config/rag3weaver/code-rag3db/templates/backends/code/backend.json`, base vide
`~/.local/share/rag3weaver/code-rag3db.rag3db`) reste chargé dans l'hôte, base ouverte, alors
que plus personne ne s'en sert. Il ne part qu'au redémarrage de l'hôte, qui tient aussi la
mémoire longue de Lucie. Une même correction (`locks_via` complété de `lockIndexForWrite` dans
`rag3db.json`) attend elle aussi ce redémarrage. La route `close` ci-dessous fermerait les deux.

## Témoin

Aucun. Il faudrait un e2e qui ouvre un manifeste par l'hôte, le modifie, et vérifie que l'appel
suivant sert le nouvel état ou rend un refus nommé.

## Ce qu'il faut pour le fermer

Au choix :
- l'hôte compare la date ou l'empreinte du manifeste à chaque appel et rouvre le backend
  (forme de `must_reopen`) ;
- une route `close` qui lâche un backend (réactions et indexations arrêtées d'abord, comme dans
  `daemon_host.rs:146-155`) ;
- au minimum, un avertissement dans la réponse quand le fichier a changé depuis l'ouverture.
