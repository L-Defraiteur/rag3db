# Borner la fenêtre nocturne de card0

- **État** : en cours ; entrée finale qualifiée et timer réel actif à luciepc pour le 13 novembre 07:54 Paris, 19 fixtures CPU et preuve réelle de descendants vertes ; aucun modèle enregistré, restitution réelle encore attendue.
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

## Mise en place et preuves du 12 novembre, 23 h 10 Paris

Sources/configuration publiées avant essais en 9040104bd, puis corrections
et probe CPU en 7a73a3327 / 55065defc / e1a58e97e. La première vérification
systemd via SSH a refusé un XDG_RUNTIME_DIR absent ; journal conservé puis
vérification passée avec `/run/user/1000` dont l’existence/propriété ont été
contrôlées. Aucun modèle n’était engagé.

Le premier probe CPU réel a montré que RuntimeMaxSec est accepté à la création
mais non modifiable par `systemctl set-property` sur ce poste. Chaîne arrêtée,
aucun modèle chargé, unité CPU et descendant disparus ; rouge conservé dans
`preparation/probe-systemd-cpu-v1-rouge.log`. La tentative de modification a
été retirée. Borne relative à la création ET cutoff absolu sont conservés ;
l’heure est relue avant exec et après l’énumération ROCr. Pas de replay inchangé.

Source corrigée `fenetre_card0.py` SHA256
`49f6399d09191b59083a1d31d7d9e1b8da710ece6fcec1a1aa106692e79b2d32`.
**18 fixtures CPU vertes** au portable et à luciepc (sous poste lourd) :
heures exactes, nuit suivante refusée, timer manqué/reconnexion, marqueur de
fermeture, source non qualifiée/modifiée, PCI changé, politique de groupe et
absence de restart, autre unité jamais arrêtée, descendant survivant rouge,
agent ROCr unique et cas sans demandeur. Les horloges fictives ne sont pas
exposées dans le CLI. Un reçu `pret.json` lié au SHA ferme l’entrée tant que
la préparation n’est pas qualifiée ou si sa source change.

Probe CPU réel v2 à luciepc, 23:09:37 : unité temporaire
`cx-card0-20261113-preuve-cpu-v2.service`, Restart=no, RuntimeMaxSec=2,
KillMode=control-group, ExecCondition de fixture `/usr/bin/true`.
Parent Python terminé, enfant `/usr/bin/sleep` démarré avec setsid (PID703538)
également terminé, cgroup vide. Aucune utilisation GPU. L’ExecCondition du
probe n’est pas une exception utilisable pour lancer un modèle : c’est un
nom distinct de la portée des modèles, aucun modèle ni recette enregistrée.

Seuls trois fichiers nouveaux installés sous `~/.config/systemd/user/` :
slice datée, cutoff.service et cutoff.timer, inchangés par rapport aux sources.
Timer **enabled / active / waiting**, **Persistent=yes**, NextElapse réel
**Fri 2026-11-13 07:54:00 CET**. Linger utilisateur actif. ExecStart du cutoff
pointe vers la copie stable hors worktree
`~/.cache/optimiseur-notes/card0-20261113/fenetre_card0.py`.
Aucun autre timer, service ou configuration GPU utilisateur modifié.

Entrée réelle `lancer /recette-inexistante` jouée hors fenêtre : refus horaire
attendu avant ouverture du fichier et avant toute commande GPU. Zéro unité
`cx-card0-20261113-modele-*`, zéro recette enregistrée. Ni rocminfo ciblé sur
card0 ni modèle exécuté pour cette preuve. La cartographie passive card0/PCI07,
card2/PCI04 et ROCm GPU[1]/GPU[0] a été relue, sans calcul.

Preuves à luciepc sous `~/.cache/optimiseur-notes/card0-20261113/` :

- `pret.json`, `preparation/etat-effectif.json`, `preparation/etat-timer.txt` ;
- `preparation/fixtures.log`, `sources.sha256` et `unites/` ;
- `preparation/verification-unites.log` et ancien `verification-unites-sans-runtime.log` ;
- `preparation/refus-hors-fenetre-qualifie.log` ;
- `preparation/preuve-descendants-v2.json`, `probe-systemd-cpu-v2.log` et rouge v1.

## Entrée pour un besoin ultérieur et suivi jusqu’au matin

Avant tout usage nouveau, déclarer le vrai modèle/consommateurs et la recette
figée, vérifier capacité/placement avec Banc/Embarquement. Aucun besoin déclaré
à ce stade ; cela n’appelle aucun chargement. Le profil initial est ROCm ; un
autre profil nécessite qualification, pas un numéro Vulkan deviné.

Une recette JSON contient nom privé, modèle, dimensions (null pour LLM),
consommateurs, profil `rocm`, durée maximale explicite `duree_max_sec`,
argv absolu avec --device ROCm0 et --no-warmup,
SHA256 de l’exécutable. Lancer uniquement par :

```bash
python3 ~/.cache/optimiseur-notes/card0-20261113/fenetre_card0.py lancer /chemin/recette-privee.json
```

Un résultat « démarrage demandé » ne prouve pas la disponibilité ou le placement
des poids et du calcul ; le consommateur doit lire le vrai processus/logs avant
ses requêtes, comme pour le banc précédent. Les PID/starttime et recette sont
consignés dans `<nom>-entree.json`, le cgroup appartient à l’unité retournée.
Le consommateur peut arrêter cette seule unité quand son travail finit.

À 07:54, le timer crée `ferme.json` et `restitution.json` après vérification
des seules unités enregistrées et de leurs descendants. Le registre est borné
à 20 s, l’arrêt systemd groupé à 30 s avec TimeoutStopSec=15 ; minute de marge
avant07:55. Si échec, le journal/refus reste rouge, pas de carte rendue affirmée.
Lire `restitution.json`, état cutoff.service et absence de cgroups/PID privés
après l’échéance, informer les consommateurs puis seulement clore ce ticket.
Le cas sans consommateur est explicite : aucune unité à arrêter et aucune
activité créée pour occuper la nuit. Aucun redémarrage ni nouvelle nuit implicite.

### Renforcement de la borne en cas de départ différé

La recette déclare une durée fixe positive ; RuntimeMaxSec reçoit cette durée,
pas un TTL qui pourrait se décaler après une longue attente du gestionnaire.
La durée doit tenir dans le restant tant à l’acquisition qu’à l’exec effectif
après sélection ROCr. Un départ retardé qui ne tient plus est refusé avant le
modèle. La durée systemd court déjà pendant la préparation de l’unité : sa
fin ne peut ainsi dépasser07:54, avec15s maximum d’arrêt et marge avant07:55,
même si le cutoff global rencontrait un problème. Aucun renouvellement de
budget. La préparation initiale est déqualifiée pendant cette révision (reçu
préservé sous preparation/pret-version-initiale.json), timer toujours armé.

Révision publiée en 986914692 avant essais, **19 fixtures CPU vertes** aux deux
postes (luciepc sous poste lourd), incluant le départ différé qui ne peut plus
tenir sa durée. Aucune nouvelle sonde GPU ni nouveau probe systemd nécessaire :
la borne initiale et la propriété de groupe sont celles du probe CPU v2 déjà vert.

**État final relu à 23:16:52 Paris** : source installée SHA256
`7d159bb45581a975cfdd08fee2180ea3e049f5eb14ca23faed134f307f182578`,
reçu `pret.json` requalifié sur ce SHA après les tests, timer toujours active /
waiting / Persistent avec NextElapse13nov07:54CET ; zéro modèle enregistré.
Entrée CLI réelle hors créneau de nouveau refusée avant lecture de recette.
L’ancienne source qualifiée18tests et son reçu restent conservés pour expliquer
la révision, sans les confondre avec la source finale.

Preuves finales : `preparation/fixtures-finales.log`, `sources-finales.sha256`,
`etat-effectif-final.json`, `refus-hors-fenetre-final.log`, `pret.json` ; mêmes
preuves de portée CPU v2. Le ticket reste ouvert jusqu’à l’échéance réelle.
