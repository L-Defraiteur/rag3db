# Attendre la disparition du port après l’arrêt du privé 7882

- **État** : ouvert — porteur optimiseur confirmé ; aucun nouveau lot ni runtime activé.
- **Famille** : produit
- **Gravité** : coût — faux rouge du wrapper après arrêt effectif, coordination interrompue trop tôt.
- **Atteignable en service** : non pour le produit ; script opérateur d’arrêt/restauration du banc.
- **Touche rag3weaver** : oui, exploitation du privé d’embarquement 7882.
- **Pour** : optimiseur
- **Signalé par** : banc, [mesure k10](../../tickets/closed/2026-11-11-mesurer-le-juge-local-avec-dix-fiches.md).
- **Ouvert le** : 12 novembre 2026

## Constat et preuves

Le 12 novembre, SIGTERM du seul PID 3273544/starttime 84847328 à
18:33:14.883 Paris par `~/.cache/rag3weaver-build/cx-service-7882-banc-2.py`,
SHA256 `d97e71c8e30c776732a6e5dd170af28fa7211aa0fae4c0d1a6ce5506e593fece`.
Le script a échoué ligne 68 sur `assert not listening(7882)` après avoir
considéré le processus terminé. Le premier relevé montrait encore le listener.

La chaîne s’est arrêtée, sans nouveau TERM ni retry de lancement. À 18:33:39.268,
contre-vérification indépendante : PID et port absents, trois autres services
inchangés, mémoire rendue. Remise explicite puis vérifications du banc avant
chargement. La mesure s’est terminée et 7882 a été restauré à 18:36:29.156,
PID 3431955. Le journal rouge initial reste conservé.

Preuves à luciepc : `~/.cache/rag3weaver-build/cx-service-7882-arret-2.log`,
`~/.cache/optimiseur-notes/banc-k10-12nov-7882-2/arret.json`,
`vram-apres-arret.json` et `retour.json` dans le même dossier.
Le bloc opérateur est conservé dans le ticket k10 fermé ; commits c84ddeb54
et 96d80615d sur la branche `cx-optimiseur-banc-7882-2`.

## Travail attendu et fermeture

Inclure l’attente de disparition du listener dans la même borne d’arrêt que
l’attente du processus, plutôt qu’un unique test immédiatement après.
Garder identité PID/start/SHA, zéro client, trois autres services inchangés,
absence de force-kill d’autrui et recette exacte de retour. Une expiration
reste un rouge nommé ; ne pas masquer une vraie occupation par un autre PID.

Fermer avec preuve CPU d’un listener qui disparaît avec retard et d’un port
qui reste occupé (expiration), sans arrêter de service réel pour le test.
Optimiseur accepte le portage ; ce ticket n’autorise aucune nouvelle coupure,
charge GPU ni nouveau passage du banc.
