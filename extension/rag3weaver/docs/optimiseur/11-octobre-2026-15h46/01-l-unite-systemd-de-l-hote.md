# L'unité systemd de l'hôte, en option du paquet

**11 octobre 2026, session optimiseur.** La page C
(`docs/11-octobre-2026-arbre-principal/01-le-demon-possede-le-catalogue.md`,
§5 et décision 3) dit : l'hôte est **lancé par le premier client** s'il
n'est pas là ; une unité `systemd --user` peut venir **en option du paquet**,
installée à la demande, jamais nécessaire, **jamais par défaut**. Cette page
fixe sa forme avant le code.

## Ce que l'hôte offre déjà (master, 6c6a12d82)

- `rag3weaver-backend daemon [--address …] [--threads …]` sert au premier
  plan ; il ne lit aucun manifeste au démarrage (chaque backend se charge à la
  première requête qui le nomme).
- Il écrit `daemon.json` (adresse réelle, pid, **jeton tiré à chaque
  démarrage**, build) dans `$RAG3WEAVER_DAEMON_DIR`, sinon
  `~/.local/share/rag3weaver/` ; le client y lit l'adresse et le jeton.
- **SIGTERM = arrêt propre**, comme `POST /arret` : l'appel en cours finit,
  les bases sont fermées, `daemon.json` retiré, code 0.
- Un second hôte devant un hôte vivant **refuse de démarrer, code 1**, en
  nommant le pid à arrêter ; une erreur de configuration (adresse illisible,
  port pris) sort aussi en 1.
- Après un `-9`, `daemon.json` reste, périmé ; le client suivant le voit mort,
  le retire et relance un hôte ; les bases sont rejouées par leur journal.

## Ce que livrent les moteurs établis

| | PostgreSQL (unité de la documentation) | Neo4j (unité de la documentation) | rag3weaver (cette page) |
|---|---|---|---|
| niveau | système, `User=postgres` | système, `User=neo4j` | **utilisateur** (`systemctl --user`) |
| installée | par l'administrateur | par le paquet .deb/.rpm, ou à la main | **à la demande** (`rag3weaver-unit install`), jamais par défaut |
| démarrage | `Type=notify` (`--with-systemd`) | `neo4j console`, au premier plan | `Type=simple`, `rag3weaver-backend daemon` au premier plan |
| signal d'arrêt | `KillSignal=SIGINT` (son arrêt rapide), `KillMode=mixed` | défaut (SIGTERM) | `KillSignal=SIGTERM` (l'arrêt propre de l'hôte), `KillMode=mixed` |
| délai d'arrêt | `TimeoutSec=infinity` (une reprise après panne peut être longue) | `TimeoutSec=120` | `TimeoutStopSec=infinity` |
| redémarrage | aucun dans l'exemple | `Restart=on-abnormal` | `Restart=on-failure`, `RestartPreventExitStatus=1`, `RestartSec=5` |
| journal | le sien (`logging_collector`) ou journald | ses fichiers `logs/` | `append:` dans le même `daemon.log` que l'hôte lancé à la main |

## Les écarts, et pourquoi

- **Utilisateur, pas système.** L'hôte sert les manifestes et les bases d'un
  utilisateur, avec ses droits ; il ne doit rien demander à root. `WantedBy=default.target`
  pour qu'il démarre avec la session utilisateur une fois activé.
- **`Type=simple`, pas `notify`.** L'hôte ne parle pas `sd_notify` ; la
  disponibilité se lit déjà par le client (`daemon.json` portant le pid
  vivant, et `GET /sante` qui répond). Pas de seconde convention.
- **SIGTERM, jamais -9, et pas de délai d'arrêt.** Comme PostgreSQL : fermer
  les bases ne doit pas être coupé par le `SIGKILL` qui suit un délai
  dépassé. `KillMode=mixed` : SIGTERM au seul processus principal, qui joint
  ses fils lui-même.
- **Redémarrer sur échec, mais pas sur le refus.** `Restart=on-failure` seul
  bouclerait sur le refus d'un second hôte (code 1) ; `RestartPreventExitStatus=1`
  l'exclut — et avec lui les erreurs de configuration, qu'un redémarrage ne
  guérit pas. Restent relancés : un hôte tué par un signal (`-9`, un
  gestionnaire de mémoire) ou qui panique (code 101). Neo4j fait le même
  choix par `on-abnormal`, qui ne relance pas un code non nul ; on relance en
  plus la panique. `RestartSec=5`, et le plafond par défaut de systemd
  (`StartLimitBurst`) arrête une boucle de pannes.
- **Le même journal.** `StandardOutput=append:` et `StandardError=append:` vers
  `~/.local/state/rag3weaver/daemon.log` (ou `$RAG3WEAVER_DAEMON_DIR/daemon.log`) :
  celui que lit le client quand un lancement échoue, et que lit `daemon_status`.
- **L'environnement figé à l'installation.** L'hôte lancé par un client hérite
  de son environnement ; l'unité, non. L'installation écrit donc un fichier
  d'environnement (`~/.config/rag3weaver/daemon.env`) avec les variables
  posées à ce moment-là parmi `RAG3WEAVER_DAEMON_DIR`, `RAG3WEAVER_DAEMON_ADDRESS`,
  `RAG3WEAVER_EMBED_SERVICE`, `RAG3WEAVER_SERVICE_EMBED`, `RAG3DB_ROOT`,
  `LD_LIBRARY_PATH` — sinon l'hôte de l'unité et ses clients ne se
  trouveraient pas (un autre `daemon.json`), ou n'auraient pas d'embarqueur.

## La règle de cohabitation

- **L'unité tourne** : le premier client trouve l'hôte par `daemon.json` et
  ne lance rien.
- **L'unité est arrêtée** (ou désinstallée) : le client lance un hôte à la
  main, comme aujourd'hui.
- **Un hôte lancé par un client tourne quand l'unité démarre** : l'unité
  échoue en code 1 (le refus d'un second hôte), ne boucle pas, et son journal
  dit le pid à arrêter. Pour que l'unité prenne la main : `kill -TERM <pid>`
  de l'hôte manuel, puis `systemctl --user start rag3weaver-daemon`.
- Aucun double hôte : le refus existe déjà, l'unité ne le contourne pas
  (jamais `--address` dans son `ExecStart`).

## La commande du paquet

`rag3weaver-unit install | uninstall | status` (un script du paquet, `bin`
de `package.json`) :

- `install` écrit `~/.config/systemd/user/rag3weaver-daemon.service` (avec le
  chemin du binaire de la plateforme), le fichier d'environnement, crée le
  dossier du journal, puis `daemon-reload` et `enable --now`.
- `uninstall` fait `disable --now` (donc un SIGTERM, l'arrêt propre), retire
  les deux fichiers, `daemon-reload`.
- `status` dit si l'unité est installée, active, et ce que dit `daemon.json`.
- Linux seulement (macOS aurait un `launchd` à part, pas dans ce lot).

Le choix d'un script du paquet plutôt que `rag3weaver-backend daemon
--install-unit` : l'unité est une option **du paquet** (page C), elle
connaît le chemin du binaire de la plateforme que le paquet a installé, et le
binaire de l'hôte (côté de l'arbre principal, page C §7) n'a pas à apprendre
systemd.

## Les témoins prévus

Sous un nom d'unité et un `RAG3WEAVER_DAEMON_DIR` d'essai, jamais l'unité
réelle de Lucie :

1. l'unité démarre l'hôte, un client le trouve (un seul démarrage au journal,
   le pid de `daemon.json` est le `MainPID` de l'unité) ;
2. `systemctl --user stop` ferme proprement : `daemon.json` retiré, ligne
   « arrêté » au journal, code 0 ;
3. l'hôte tué (`-9`) est relancé par l'unité, et l'appel suivant du client
   rouvre sa session ;
4. un hôte manuel déjà là : l'unité échoue en 1, sans boucle ;
5. l'unité désinstallée, le client relance un hôte à la main.

Sources : [PostgreSQL, Starting the Database Server](https://www.postgresql.org/docs/current/server-start.html) ;
[Neo4j, Linux executable (.tar)](https://neo4j.com/docs/operations-manual/current/installation/linux/tarball/).
