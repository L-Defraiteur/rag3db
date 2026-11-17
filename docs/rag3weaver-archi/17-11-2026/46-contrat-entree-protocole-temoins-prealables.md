# Contrat d'entrée : protocole et témoins préalables, aucune implantation

Après arbitrage C7 **93c44be2d6**, publication SOURCE/API/TÉMOINS seulement :
[protocole et portée](execution-entry-contract/README.md),
[API proposée](execution-entry-contract/execution_entry.pyi),
[31 fonctions de témoins hermétiques NON JOUÉES](execution-entry-contract/test_execution_entry.py).
Pas de module opérationnel ni de producteur livré. Ticket commun ouvert,
[source45](45-etude-source-producteur-images-executees.md) conservée.

Le protocole proposé pour RO est un parent naturel runner / enfant bootstrap
monothread / barrière au premier événement EXEC réussi / copie depuis l'objet
réel arrêté / comparaison avec catalogue préautorisé / publication CP avant
libération / detach / vrai wait. Stabilité des bytes dépend d'un profil local
régulier avec exclusion d'écriture à qualifier ; ni FD ni deux hashes ni EOF ne
prouvent seuls ce prérequis ou le succès d'entrée. Aucun fallback de chemin,
traceur persistant ou second GDB supposé compatible.

Les sources de démarrage historiques ont seulement été lues pour séparer capture,
libération et wait naturel. Aucun pilote, délai ou preuve n'est repris. SDK reste
indépendant. Le résultat appartient à l'invocation du programme revu ; aucune
continuité A ou attribution de chaque octet à un PID n'est promise.

API .pyi et témoins visent un FUTUR validateur pur, pas un adaptateur OS. Le
module importé par les témoins est absent ; aucun import/parse/test/CPU n'a été
exécuté. Publication avant implémentation et exécution, pas d'annonce rouge/vert.
Ce contrôle de modèles ne pourra produire une admission physique.

TTY **44ae9af3bcb697054467d472d8581e1117764828** relu : alignement documentaire
sur invocation/entrée, P17–P20 inconnus et non implicitement requis pour anglais,
P01–P16 racines à qualifier. La table TTY est la référence détaillée des vingt
spécifications, pas un inventaire de tests acquis.

Recherche a reçu la réservation du futur module partagé execution_entry.py et
des quatre sites e2e/raccord Cargo, et déclaré aucun chevauchement d'écriture.
Aucun fichier sous scripts/, appelant, canon2150, environnement, target ou service
modifié. La réservation ne constitue pas l'approbation du protocole physique.

Suite autorisée : RO du protocole/API/témoins et raccord publication/identités,
puis décision C7 avant implantation ; GO CPU distinct avant toute exécution.
Coûts/compatibilité hôte et stockage UNKNOWN. Pas de build/native/proc/signal,
slot, Q actif, ressource OS ou nouvelle autorité consommés. Diff documentaire
et sources de témoins seulement, vérification d'espaces Git sans exécuter tests.
