# Couple ciblé r3 : dispatch CLI pur avant watchdog

APP7425375d66a64b0782672225c59516f048682b00 inchangée. Le pilote est le HEAD
publiant ce rapport et le correctif CLI ; son SHA intégral sera inscrit dans
le nouveau Qr3 et transmis aux trois relecteurs après préparation effective.
Ticket existant complété93e63f9737 AVANT diff. Les anciens pins291/a642 et
Qr1/Qr2/paquets restent conservés/non lancés.

Réserve Banc6a1eea6695 : le vrai __main__ appelait configure_controller qui lisait
Q/launch.json AVANT le watchdog, même si launch seul était pur. Contre-exemple
prépublié0c3bdc2a31 : vrai dispatch __main__ extrait AST, vraies fonctions de
configuration/lancement et sentinelle Path.exists, faux subprocess uniquement.
ROUGE1/0.004s sous GO CPU60 C7 : événements Q-before-watchdog puiswatchdog.

Correctif : configure_controller ne fait QUE des affectations des chemins et
OUTER_SECONDS=TERM780 constant. Aucune lecture Q au préambule CLI ; les lectures
de launch.json restantes sont dans les modes déjà bornés --admit/driver. Le
watchdog --arm précède toujours tout le worker/précontrôles/log/fsync/file/poste,
Common/final/publication, avec horloge observée honnêtement APRÈS armement.
Correction postfsync9f3 et trois rôles/argv stricts conservés.

Qr3 `cx-archi-doc-bac-green-controller-r3` NEUF pour éviter de réécrire les copies
préparées précédentes. Même Pvide/Fabsent/source742/1T/archive223/SDK59. Recette
600MAX communes sous TERM780/KILL15 inchangée, deux buildsJSON/ELF/inventaires,
ledger+copies2hôtes publié AVANTmarker et deux juges/quatre invocations, positifs
Sandbox+HEAD ; aucun native/Vertex/D6/F8global/service/DB exécuté.

Le code et ses treize témoins sont publiés AVANT le rejeu CPU. La préparation
Git/copies/paquet r3 déjà autorisée avancera uniquement le contrôleur par FF ;
le nouveau pin et les preuves rouges/vertes seront réhashés aux deux postes.
RO totale/code+CLI+pin à Banc/Mémoire puis GO C7 exact distinct AVANT natif.
Aucune réussite CPU ou préparation n'est anticipée dans ce rapport.
