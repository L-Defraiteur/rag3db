# Qualification IR pour la navigation des symboles

Porteur codeparsers, soutien au ticket
[exposer la navigation](../../../rag3weaver-archi/tickets/2026-11-12-exposer-la-navigation-dans-les-symboles.md).
Sept petites fixtures, six langages (C++ sur deux fichiers), producteur a730a9b.
Aucun nouveau module runtime, changement de schéma, gitlink ou ancien dépôt.

Recette publiée avant exécution : `recette.sh` construit l'exemple existant
voir_json du producteur figé, puis exporte ScopeFileAnalysis pour chaque fichier.
Une acquisition poste lourd, j8, timeout600s après acquisition ; target privé
unique target-codeparsers-clic (l'ancien a été supprimé), aucun GPU/service.
Preuves neuves hors worktree, source et hashes enregistrés ; arrêt au premier
échec inattendu, sans répétition inchangée. Si le créneau CPU est chargé,
la recette attend sous poste, la borne ne commence qu'après acquisition.

Ces sorties prouvent ce que parse_file extrait, **pas** la matérialisation
complète d'une base ni l'accessibilité d'un outil. Les projections code.rs et
le Plan outline seront relus séparément ; les omissions seront nommées, sans
inférer d'UUID ou de parent lexical à partir d'un simple nom.
