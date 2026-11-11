# Terminer l’archivage à blanc des documents sur copie

- **État** : ouvert, en attente d’une nouvelle consigne de l’orchestration pour lancer les cinquante documents ; aucun lancement demandé par cette régularisation.
- **Famille** : produit
- **Gravité** : planification ; disposer d’une proposition vérifiable avant toute écriture dans la mémoire réelle.
- **Atteignable en service** : non, l’essai doit rester sur une copie isolée.
- **Touche rag3weaver** : oui, outil archive du gabarit memory et outillage de mesure/proposition.
- **Pour** : session mémoire ; créneau de mesure à coordonner avec l’orchestration et optimiseur si requis.
- **Signalé par** : reprise du 11 novembre, [rapport §3.1 et §7.3](../11-11-2026/01-rapport-de-session.md) ; maintien explicite des cinquante documents en attente par l’orchestration.

## Preuves disponibles

Le refus 4xx se relit depuis `2d6a08a65`. Les alias de candidates sont corrigés et livrés sous `41526b29a`, ticket clos [du correctif](../../tickets/closed/2026-11-11-les-noms-des-fiches-du-juge-font-refuser-les-decisions-de-l-archiviste.md).

Avec les scripts exacts du correctif, binaire certifié d1345a115 et lib complète 29a7bc52a, le premier document traite sept sections sans erreur ni refus sur copie neuve, en 331,5 s totales. Ce temps n’est ni exclusif ni une mesure par section. Les [notes de preuve](../11-11-2026/04-alias-candidats.md) donnent le SHA256, les maps, les journaux et le dossier temporaire `validation-correctif-alias/`. Aucun classement sémantique des cinquante documents n’est validé par ce résultat.

## Reste à faire, après consigne

Vérifier la disponibilité du corpus figé et de l’instantané, recréer une copie cohérente et épingler les artefacts utilisés. Arrêter le protocole de mesure par section et le budget avec l’orchestration ; ne pas transformer les durées précédentes en comparaison de performance. Jouer le premier tiers prévu, rapports/knowledge dumps/conceptions, puis produire une proposition : sources gardées, sujets proposés, apports, ancrages, erreurs, appels et temps, avec contrôle de la qualité du rangement. Les tickets du corpus viennent ensuite selon l’ordre convenu.

Ne jamais écrire dans la mémoire réelle pour cet essai. La montée des gabarits et toute activation réelle restent suivies séparément dans [le ticket M8](2026-11-07-une-memoire-ne-sait-pas-que-ses-gabarits-sont-en-retard.md).

## Critères de fin

Rapport de l’essai autorisé et proposition relisible livrés avec provenance du corpus, copie, artefacts, vérifications et limites ; toute section non traitée est explicitement comptée. Aucun faux vert basé sur le seul code de retour. L’attente, la correction du 400 ou le seul premier document ne ferment pas ce travail.
