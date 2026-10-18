# L'hôte par la socket de systemd

*Session optimiseur (G), 18 octobre 2026, à la demande de Lucie par l'orchestration.*

Le but : plus jamais un client qui démarre son propre hôte avec un vieux binaire.

## Ce qui change

- **L'unité** : `rag3weaver-daemon.socket` écoute l'adresse de l'hôte (127.0.0.1:7970). La
  première connexion lance `rag3weaver-daemon.service`, qui démarre le **binaire installé**
  (`~/.local/bin/rag3weaver-backend daemon`). L'hôte prend la socket passée par systemd
  (`LISTEN_FDS`) au lieu de se lier lui-même au port. On la pose avec
  `extension/rag3weaver/scripts/installer-l-unite-de-l-hote.sh`, qui reprend
  `~/.config/rag3weaver/rag3weaver-daemon.env` tel quel.
- **L'arrêt après inactivité** : sans requête ni travail en cours depuis
  `RAG3WEAVER_DAEMON_IDLE_MIN` minutes (30 par défaut, 0 = jamais), l'hôte s'arrête seul et
  rend sa mémoire. L'appel suivant le relance par la socket.
  - Ce qui le retient : une requête en vol (un appel, un flux de l'IDE ouvert), une
    indexation, un run en fond, des fichiers que la montre doit encore réindexer, un
    catalogue tenu.
  - Ce qui ne le retient pas : une session ouverte sans appel, une sonde `/sante`.
- **Les clients sous l'unité** ne lancent plus jamais d'hôte : ils se connectent.
  - Sans unité, l'autodémarrage reste le repli : Windows, et macOS (launchd, plus tard).
  - Un client dont le binaire n'est plus celui de l'unité (un autre inode) refuse de
    s'attacher, avec l'état « périmé » : « binaire mis à jour… relancez la session ».
  - Un 403 de jeton, celui d'un hôte relancé derrière la même adresse, fait relire
    `daemon.json` et rejouer une fois.
- **L'arrêt borné** : les réacteurs ont 30 s pour s'arrêter (`RAG3WEAVER_STOP_WAIT_S`).
  - Au-delà, le réacteur figé est nommé, son catalogue n'est pas fermé sous lui, et l'hôte
    sort.
  - Ce qui est validé est dans le journal du moteur, rejoué à l'ouverture suivante.
  - L'unité porte `TimeoutStopSec=90` en filet.
  - Ticket fermé : « l'arrêt de l'hôte attend sans borne une réaction figée ».

## Le jeton, stable d'un démarrage à l'autre

Sous l'unité, l'hôte s'endort et se réveille plusieurs fois par jour. Un jeton neuf à chaque
démarrage ferait perdre à l'IDE son entrée (son cookie) à chaque réveil : un 403, puis une
nouvelle URL à demander.

- Le jeton est rangé dans `daemon.token` (0600), à côté de `daemon.json`.
- Il n'est tiré que s'il manque ou s'il est illisible.
- `rag3weaver-backend daemon-nouveau-jeton` en tire un neuf sur demande. Un hôte en service
  garde l'ancien jusqu'à son redémarrage, et l'IDE se rouvre ensuite par
  `rag3weaver-backend ide`.

Un client qui reçoit quand même un 403 (un jeton renouvelé) relit `daemon.json` et rejoue
une fois.

## Ce qu'il faut attendre de la montre

**« La montre dort avec l'hôte »** : sans client et sans travail, l'hôte s'endort après 30 min,
montre comprise. **Pendant qu'il dort, rien ne suit l'arbre** : les fichiers modifiés ne sont pas
réindexés.

À son réveil, l'hôte rattrape tout ce qui a changé pendant son sommeil. Le backend ne se rouvre qu'à son premier appel. Sa montre rattrape alors ce qui a
bougé pendant le sommeil (la réconciliation du démarrage). Le premier appel paie ce
rattrapage, et sa réponse peut dire « une écriture de l'index est en cours » le temps qu'il
passe.

Pour qu'une base soit suivie en continu, poser `RAG3WEAVER_DAEMON_IDLE_MIN=0` dans l'env de
l'unité. **C'est un choix de Lucie.**

## Les témoins

- **Unitaires** :
  - l'hôte inactif s'arrête seul, et une sonde ne le retient pas ;
  - une requête en vol retient l'hôte ;
  - l'unité se lit et ne sert que son dossier de démon ;
  - un client périmé se voit ;
  - un réacteur figé ne tient pas un arrêt borné.
- **Sous systemd** : `scripts/temoin_unite_socket.py`, sur une unité d'essai à part
  (`rag3weaver-daemon-essai`). Quatre cas :
  - un appel sans hôte démarre l'unité, et le client ne lance rien ;
  - l'hôte inactif s'arrête seul, et la socket reste à l'écoute ;
  - un client resté ouvert continue après l'arrêt ;
  - un client périmé est refusé, avec son message.
