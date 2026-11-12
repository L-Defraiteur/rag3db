# Borner la fenêtre nocturne de card0

- **État** : en cours ; source et unités préparées sur cx-optimiseur-card0-nuit avant essais/installation ; aucun modèle card0 demandé par banc/embarquement.
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

## Source et recette avant essais

Banc et Embarquement confirment aucun besoin card0 à ce stade ; le premier privé
utile éventuel est granite-107m/384 sur card2. Aucun modèle pour occuper la nuit.
Source `docs/optimiseur/scripts/fenetre_card0.py`, ses fixtures et trois unités
temporaires datées ; aucun code produit ni lib courante changé.

CLI sans option d’horloge fictive : hors fenêtre refusé avant commande GPU ;
nouvelle demande refusée à moins de 60 s du cutoff. Profil préparé ROCm explicite
(ROCR=1, --device ROCm0 après filtrage, --no-warmup), autres profils refusés
tant que non demandés/qualifiés. Sysfs vérifié ; au vrai démarrage seulement,
agent ROCr unique BDF1792/PCI07. Ce n’est pas une preuve des poids/cache/calcul
du modèle : consommateur responsable de les qualifier avant requêtes/mesures.

Modèles enregistrés sous `cx-card0-20261113-modele-*.service` par recette/hash,
modèle/dimensions/consommateurs. Slice privée, KillMode=control-group,
RuntimeMaxSec au restant, TimeoutStopSec=15, Restart=no. ExecCondition puis
relecture d’heure avant exec ; PID/starttime et recette conservés.

Cutoff absolu **07:54:00 Paris** (06:54 UTC), pour laisser une minute avant07:55 :
timer unique Persistent=yes, AccuracySec=1s, sans délai aléatoire. Ferme le
registre définitivement, refuse nouvelles entrées et arrête seulement les
unités dont description/hash/slice et politique correspondent au registre.
Le cgroup entier et ses descendants sont contrôlés après arrêt. Aucun arrêt
global, du jeu ou d’un autre service. Un timer manqué ou une reconnexion ne
rouvre pas la fenêtre ; réveil tardif ne recharge aucun modèle.

Linger utilisateur relu actif à luciepc. Recette après publication des sources :

```bash
python3 docs/optimiseur/scripts/test_fenetre_card0.py -v
# À luciepc : copie hors worktree, systemd-analyze --user verify des unités,
# installation des trois seuls fichiers, timer actif et NextElapse relus.
# Entrée réelle hors fenêtre : refus attendu, aucun modèle lancé.
```

Prévoir aussi preuve CPU systemd privée de terminaison d’un descendant, sans
GPU. Garder configs/empreintes/état effectif du timer. Ticket ouvert jusqu’au
constat réel de restitution : cas CPU ou timer actif ne suffisent pas.
