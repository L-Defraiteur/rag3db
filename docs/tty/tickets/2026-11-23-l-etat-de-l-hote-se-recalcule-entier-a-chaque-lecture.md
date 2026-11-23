# L'état de l'hôte se recalcule entier à chaque lecture

- **État** : ouvert le 23 novembre 2026, après une mesure du démarrage lent du terminal à la tablette. Il revient aux propriétaires de l'hôte.
- **Famille** : produit
- **Gravité** : chaque `GET /ide/etat` (ou `/etat`) coûte 6,5 à 7 s avec 84 backends. L'IDE le paie à chaque rafraîchissement, et toute fonction qui s'en sert pour une petite information aussi.
- **Atteignable en service** : oui, mesuré sur l'hôte de service de la tablette.
- **Touche rag3weaver** : oui (`DaemonHost::state`, `daemon_host.rs`).

## Ce qui s'est passé

Le démarrage du terminal prenait 15 à 20 s. Mesuré phase par phase le 23 novembre (84 backends) :
- `POST /depots/resoudre` : 6,9 s ;
- `POST /dossiers/ouvrir` : 7,2 s ;
- `GET /ide/etat` : 6,8 s ;
- tout le reste : de 5 à 545 ms.

Les trois passaient par `state()`. Pour chaque backend, `state()` lit l'état de son index (`index_states()`) et sa branche. C'est la lecture des index qui coûte.

Trois lots retirent `state()` du démarrage, sans toucher à `state()` lui-même :
- `host-resolve-light` : la résolution lit la table des backends ;
- `tty-fast-start` : le terminal ne relit plus `/ide/etat` au démarrage ;
- `host-publish-state-async` : la publication de l'état sur le flux part en fond.

## Ce qu'il faut

- `state()` ne relit pas les 84 index à chaque appel. L'état de l'index d'un backend se tient à jour quand il change (fin d'index, synchronisation, ouverture, fermeture), et `state()` le lit dans ce cache.
- Une lecture de l'état coûte moins de 100 ms avec 100 backends.
- Le témoin est rouge d'abord : un hôte de 100 backends ouverts, dont la lecture de l'état est mesurée.

## Pour mesurer

Le script de mesure phase par phase (les appels du démarrage du terminal, chronométrés par `curl`) est décrit dans ce ticket. Il suffit de rejouer `POST /depots/resoudre`, `POST /dossiers/ouvrir` et `GET /ide/etat` sur l'hôte de service.
