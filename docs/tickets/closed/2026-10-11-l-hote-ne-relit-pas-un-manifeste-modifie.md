# L'hôte ne relit pas un manifeste modifié, et aucune route ne ferme un backend

- **État** : corrigé (11 octobre 2026, arbre principal) — trouvé en montant le MCP du code pour rag3db (session codeparsers)
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

## Correctif

Deux routes de l'hôte (`src/daemon_host.rs`), et l'empreinte du manifeste (son contenu, pas sa
date) notée à chaque ouverture :

- `POST /backend/open {manifest}` (et `/open`, son ancien nom) : ouvre ; déjà ouvert, rend tel quel,
  **sauf si le manifeste a changé** : le backend est fermé proprement et rouvert sur le fichier
  d'aujourd'hui, même base (`reread: true`). Un manifeste absent est un refus nommé
  (`manifeste absent`).
- `POST /backend/close {manifest}` : ferme proprement (réactions et montre, indexations, catalogue)
  et lâche la base ; le prochain appel rouvrirait. Refus nommés : `aucun backend hébergé pour ce
  manifeste`, et `backend tenu par une indexation` (n en cours), qu'on arrête par l'outil `index`.
- Un `/call` ou `/describe` servi sur un manifeste modifié porte `warnings` (« le manifeste a changé
  depuis l'ouverture… POST /backend/open le relit ») ; `/etat` dit, par backend, `open`, `opened`,
  `closed` et `manifest_changed`.

Témoins (`tests/e2e_demon_hote.rs`) : `un_manifeste_modifie_se_dit_puis_se_relit_a_l_ouverture`
(un outil ajouté au manifeste : absent et averti avant, présent sur la même base après la
relecture) et `fermer_un_backend_lache_sa_base_et_les_refus_sont_nommes` (la base lâchée s'ouvre
hors de l'hôte avec sa fiche ; les deux refus nommés). Le refus « tenu par une indexation » n'a pas
de témoin e2e : il demande une indexation assez longue pour être prise en cours, ce qui rendrait le
témoin dépendant de la vitesse du poste.

