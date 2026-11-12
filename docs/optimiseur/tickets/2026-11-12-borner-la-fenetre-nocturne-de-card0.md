# Borner la fenêtre nocturne de card0

- **État** : ouvert ; créneau autorisé par Lucie, entrée et restitution à mettre en place avant usage.
- **Famille** : produit
- **Gravité** : disponibilité ; les modèles temporaires doivent rendre le GPU du jeu à l'heure convenue.
- **Atteignable en service** : oui pour les processus privés nocturnes, aucune modification des services courants requise.
- **Touche rag3weaver** : non pour le code produit initial ; coordination des profils et ressources de validation.
- **Pour** : optimiseur ; banc et embarquement consommateurs.
- **Signalé par** : Lucie, autorisation de card0 de minuit à08:00 cette nuit.
- **Ouvert le** : 12 novembre 2026

[Répartition de nuit](../../orchestration/12-11-2026/02-lots-independants-pour-la-nuit.md).
Fenêtre Europe/Paris le13novembre2026 00:00–08:00, sans extension implicite aux
autres nuits. Card2 reste utilisable normalement. Préparer une entrée vérifiant
l'heure et le placement réel, une portée privée connue avec durée au reste du
créneau, et un arrêt effectif de ses seuls modèles avant08:00 (marge07:55).
Refuser les acquisitions tardives ; éviter qu'une unité puisse se relancer
après échéance. Garder modèles/dimensions/profils, PID/starttime/PCI et preuves.

Revérifier la cartographie physique et les consommateurs avant usage. Ni filtre
ROCm ni numéro Vulkan seul ne prouve le placement. Pas d'arrêt de jeu, bus de
session, compositeur ou service tiers. Pas de poids chargés pour occuper la carte.
Modèles existants, caches et données ne sont pas supprimés ou substitués.

Fin : entrée testée sans lancer de modèle hors créneau, vraie échéance/arrêt
des unités privées enregistrés, carte rendue avec preuve et consommateurs
informés. La simple consigne écrite ne clôture pas la restitution nocturne.

## Revue du garde975062659 avant premier usage

Orchestration a relu le script49f6399d et les unités, puis interrogé luciepc :
timer enabled/active/waiting, Persistent=yes, prochaine échéance réelle
13novembre07:54CET ; SHA installé égal au SHA déclaré par pret.json,18CPU
rapportés et aucun modèle enregistré. La restitution future reste à constater.

Une frontière interne reste à qualifier avant avis favorable : le mode CLI
`executer` (lignes239–258 de cette source) vérifie recette enregistrée, empreinte,
heure et exécutable, mais pas son appartenance à l'unité privée. Appelé directement
avec une recette déjà enregistrée et son empreinte, il peut atteindre execvpe
hors de l'unité dont le cutoff arrête les descendants. Il ne s'agit pas de
promettre une défense contre un utilisateur hostile du même UID : le garde doit
simplement refuser une invocation accidentelle de son entrée interne hors portée.

Réutiliser le contrôle de propriété d'unité et vérifier le PID principal réel et
le cgroup attendu avant toute commande ROCr/exec. Des variables d'environnement
ou le seul chemin de recette ne prouvent pas cette appartenance. Témoins CPU :
entrée interne hors unité/avec unité étrangère refusée avant commande GPU, entrée
attendue admise et descendants toujours arrêtés. Aucune charge modèle n'est
nécessaire pour qualifier ce delta. Garder le timer stable, publier la source et
réactualiser son reçu qualifié après preuve ; aucun usage card0 dans l'intervalle.
