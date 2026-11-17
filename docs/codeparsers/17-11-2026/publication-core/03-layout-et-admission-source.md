# Assemblage du relais : candidat source, aucune admission physique

Cette préparation poursuit f494d8ecbe après les rouges C9/7ee conservés.
Aucun test/import/parse du code de lot, SSH de phase, processus, acteur, slot,
Cargo, target ou corpus n'est exécuté. Les preuves D45/P05/C9/P06 sont figées.
Le ticket du ledger autonome reste ouvert. Les SHA du candidat sont dans
`publication-capacity-pins.json` ; revue totale avant tout nouveau GO CPU.

## Une seule mécanique, des dispositions déclarées

`publication_relay.py` conserve les boucles, observations de processus,
bootstrap, copies, ACK, fenêtres et annulation. `publication_layout` déclare :

| Champ | Contrat |
|---|---|
|prepared|Fichier approuvé dans P (preuves) ou Q (contrôleurs), par exemple Q/configuration.json|
|execution_host|Hôte de l'autorité ; les durées seules traversent SSH|
|entries|Rôles coordinator/gate/launcher/driver/common/guard vers fichiers de Q|
|phases|IDs indépendants des labels Rust, chacun avec root/peer/scope et fichiers de rendez-vous|
|files|bootstrap/launch/authority/controls/outer_supervision/common_command/allocation/first_red/abort/classification/peer_ready/peer_supervision|

Root peut être P lui-même, scope P/supervision, peer une racine privée sous P.
Les noms des fichiers sont déclarés, jamais choisis dans un ledger. Les chemins
sont absolus, sous la racine approuvée, distincts et sans composant parent.
Le prepared et tous les contrôleurs sont vérifiés avant import des contrôleurs
et à chaque opération. Les entrées requises et publication_relay.py appartiennent
à la fermeture des SHA. Les CLI acceptent --prepared, phase/kind/file génériques ;
les plans et fichiers déclarés restent les seules valeurs admises ensuite.
Sans --prepared, le défaut CLI Rust est P/prepared.json ; aucune découverte.

Les deux publications Rust sont construites par son wrapper, ainsi que son
layout historique. Il n'existe pas de seconde boucle globale. Le futur
adaptateur Archi devra construire ses propres descripteurs depuis ses sources
revues : copier d089 ne constitue pas une preuve des producteurs réels.
En particulier les fichiers de recette (Q/launch.json) et les rendez-vous
observés de lancement sont des faits distincts, pas des noms interchangeables.

## Codecs de politique, observations communes

Le pilote approuvé fournit les données suivantes ; il ne fournit jamais une
réponse de vivacité au noyau :

| Fonction | Valeur |
|---|---|
|publication_launch_command(pair,phase,allocation)|argv attendu du lanceur après véritable exec|
|publication_launch_environment(pair,phase)|Surcharges autorisées de l'environnement de lancement|
|publication_deadline_info(pair,phase)|Rôle/argv du watchdog de phase, préfixe vérifié contre les limites|
|publication_control_roles(pair,phase)|Ensemble exact des rôles/argv indépendants du fichier controls observé|
|publication_phase_info(pair,phase)|Chemin autorité, identités Common/driver, entrées, argv, scope/controls approuvés et END original|

Le Gate vérifie PID/start/UID/exe/argv/cgroup, ascendance driver→Common, empreintes
et autorité active, avant et après la fenêtre. Le fichier controls doit porter
exactement les rôles annoncés ; l'annulation garde leurs observations finales,
y compris quand aucun tour de boucle n'a pu être joué. Les signaux restent
attribués par pidfd aux enveloppes prouvées ; quiescence inconnue reste UNKNOWN.
La propagation aux six rôles globaux est à revoir/qualifier avec leur véritable
adaptateur ; elle n'est pas prouvée par les fixtures Rust à deux contrôles.

## Bornes originales et fenêtre propriétaire

Le profil global à fournir reste : TERM7440/KILL15 avant ARM/file, queue120
DANS ce budget, Common demandé MIN(7200,reste physique−30), ENDop=ENDCommon−15,
réserve arrêt15 DANS Common. Le profil Rust garde 2098/2, queue300,
phase1798/2, Common maximum1750, marge30, ENDop−15 et réserve5.
`common_request_budget` partage la vérification du bootstrap/watchdog et borne
la demande entière (Common accepte des secondes entières). Le pilote Rust
l'appelle juste avant Common. Le Gate refuse un END qui excède la borne du
watchdog original moins sa marge ; il ne le raccourcit pas en silence.
Le soft END physique sert à l'admission, KILL n'accorde aucun temps de travail.

Chaque fenêtre propriétaire est créée une seule fois sur son chemin approuvé,
par son vrai rôle déclaré, après scellement du ledger et ACK antérieurs.
observe_window ne crée rien. Durée MIN(cap90,ENDop−maintenant), aucune remise
à zéro de budget global, aucune comparaison d'horloges absolues interhôtes.

## Résultat exact et suite

Les assertions des 36 témoins historiques et les trois contre-exemples restent les mêmes ;
les fixtures déclarent maintenant layout/limites et descripteurs attendus.
Une insertion répétée de recette hors setUp repérée en lecture a été retirée
avant tout jeu. Les huit témoins Window restent ceux prépubliés avant helper.
Aucun résultat rouge ou vert de ce candidat n'est revendiqué.

Revue totale demandée sur moteur, codecs/wrappers, fixtures et pins. Un éventuel
nouveau GO CPU doit nommer ce candidat et ses commandes, jamais réutiliser les
GO C9/7ee consommés. Le package global réel, ses producteurs de rôles, son
prepared et sa fermeture Q restent à fournir/revoir par Archi ; aucune activation
ni bridge opérationnel n'est livré par ce document. La capture d'image d'entrée
reste le périmètre Archi, distinct du relais de publication CP.
