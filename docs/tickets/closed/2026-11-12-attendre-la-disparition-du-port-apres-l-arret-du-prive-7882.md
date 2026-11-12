# Attendre la disparition du port après l’arrêt du privé 7882

- **État** : corrigé 62640bd42 — attente PID/port dans une borne commune, 14 filets CPU verts aux deux postes ; aucun service réel ni modèle activé.
- **Famille** : produit
- **Gravité** : coût — faux rouge du wrapper après arrêt effectif, coordination interrompue trop tôt.
- **Atteignable en service** : non pour le produit ; script opérateur d’arrêt/restauration du banc.
- **Touche rag3weaver** : oui, exploitation du privé d’embarquement 7882.
- **Pour** : optimiseur
- **Signalé par** : banc, [mesure k10](2026-11-11-mesurer-le-juge-local-avec-dix-fiches.md).
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

## Reprise du correctif

Lecture des règles de décision de `03-reprise-par-codex.md`, sections 2 bis et
2 ter. Nouveau worktree/branche `cx-optimiseur-attente-port`, base 41497ba4e.
Source historique figée : copie opérateur SHA d97e71c8 ci-dessus, conservée
inchangée dans les notes ; version du dépôt `docs/optimiseur/scripts/service-prive-7882.py`.
Le wrapper conserve sa recette, ses gardes et ses constantes historiques : il
ne s’agit pas d’une nouvelle activation prête à exécuter sur un autre PID.

Rouge CPU posé : le témoin exécute par AST les vrais deux blocs historiques
de fin PID puis assertion listener, avec horloge et observations contrôlées.
PID absent et listener du même PID visible une seconde : assertion rouge,
ligne historique 68, sans import/exécution du mode opérateur ni signal.
Preuve portable : `~/.cache/optimiseur-notes/attente-port-12nov/rouge.log`.

Correctif : `attendre_arret.py` observe identité/starttime et propriétaires du
port sous une seule deadline fixée avant SIGTERM. Il n’envoie aucun signal.
PID remplacé, autre propriétaire ou propriétaire inconnu refusent ; une
expiration et une erreur de sonde ne deviennent jamais une absence.
La recette, la santé, zéro client, les trois autres services et le retour
restent contrôlés par le wrapper ; aucun ancien journal n’est réécrit.

## Vérifications du correctif

Sources et recette figées par SHA256 avant les tests, puis empreintes relues
après tests. Portable et luciepc : rouge historique reproduit par le vrai
bloc AST, puis **14 tests CPU verts**, aucun sauté. À luciepc, la passe CPU
a été jouée sous `poste lourd`. Aucun ss de service, SIGTERM réel, mode stop
ou restore, modèle, base ou charge GPU n’a été utilisé pour ces tests.

Couverture : listener retardé après fin du PID ; budget commun processus/port ;
timeout processus ou listener ; port repris immédiatement ou pendant l’attente ;
PID réutilisé ; propriétaire inconnu ; zombie avec port libre ; erreur de
lecture ; observation qui finit hors deadline ; identité du processus CPU du
test ; parsing des propriétaires ss et propagation de ses erreurs.

Commandes :

```bash
WRAPPER_ANCIEN=<copie historique d97e71c8> python3 docs/optimiseur/scripts/test_attendre_arret.py AttenteArret.test_port_retarde_apres_fin_pid
# Ce premier appel doit échouer sur l’assertion historique ligne 68.
python3 docs/optimiseur/scripts/test_attendre_arret.py -v
```

Preuves aux deux postes : `~/.cache/optimiseur-notes/attente-port-12nov/`
avec `rouge.log`, `vert.log` et `sources-avant-tests.sha256`. À luciepc, les
trois sources exactes sont aussi conservées dans ce dossier ; l’ancienne
copie `~/.cache/rag3weaver-build/cx-service-7882-banc-2.py` reste inchangée.
Les journaux de l’incident, des retours et de l’audit des résidents restent
également intacts. Le correctif ne réactive pas 7882 ni 107m, désormais arrêtés.

Périmètre de validation : script opérateur Python seulement ; aucune source
Rust, moteur, UI ou gabarit produit changée. Pas de passe cargo ni de test
sur un service GPU revendiqué. La version de recette est historique et ses
PID/dossier de preuves ne doivent pas être réutilisés pour une nouvelle
opération sans préflight et coordination propres.
