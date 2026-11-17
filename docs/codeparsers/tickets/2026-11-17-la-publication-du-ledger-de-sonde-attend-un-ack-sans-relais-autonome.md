# La publication du ledger de sonde dépend d'un relais de session non autonome

État : ouvert, défaut de coordination observé ; pas de correction/rejeu autorisé.

## Observation

GO C7-PARSERS-HIST-BASE-P02-16nov1800, outillagec1cd/P02f919, référencefec351 :
compilation seule rc0/33,04s, puis premierrouge peer publication deadline à
END−15 après environ1700,7s d'attente. Aucun build-peer-ack, aucun inventaire et
aucun test natif exécuté. Aucun candidat ni comparaison lancés.
Le pilote a refusé correctement ; le raccord de publication PC→portable→ACK
reposait sur le retour actif de la session, sans relais autonome démarré avant
l'entrée. La cause d'une indisponibilité prolongée de la session n'est pas prouvée
par les fichiers du lot ; ne pas l'attribuer arbitrairement au reset SSH observé.

## Attendu pour un futur lot explicitement autorisé

La publication des deux étapes doit être concrète et disponible pendant le délai
courant : ledgerbuild+copieELF vérifiés deuxhôtes avant toute invocation ; puis
ledgerruntime+inventaires avant test. ACK atomique, source/profil/hashes liés,
aucun transfert d'autorité/jeton, jamais d'ACK tardif pour réarmer un lot rouge.
La coordination doit se terminer ou refuser dans sa propre borne, conserver son
premier défaut et permettre la fermeture de toute activité qu'elle a lancée.
Composer les outils existants, préciser les rôles et limites avant nouvelle
allocation ; aucun worker/service ajouté ni test relancé par ce ticket.

## Preuves et état sûr

Rapport :docs/codeparsers/17-11-2026/rust-probe/01-reference-refusee-a-la-publication.md.
Bruts+nouvelELF conservés2hôtes sous rust-probe-pair-02/baseline ; classification
077e33f1dff091c3a583ae1131841354677cc57c6b49f8455a17751410166a71.
CommonTERMINE/code1/restes[]/nettoyé ; bus absent, audit postmortem ownedquiet.
TargetPC appartient désormais à BASE ; P02 est une archive consommée.
Aucun résultat sémantique de sonde à interpréter. Parent Rust reste ouvert.

## Préparation corrective autorisée par C7, avant code

Préparation seulement sur branche privée cx-codeparsers-rust-relay, issue du
rapport1be7dc3d97 ; P02 reste intouchée. C7 demande un relais autonome présent
avant le lancement distant, deux publications sous leurs bornes, contrôles de
phase active/PID-start/UID/scriptsSHA avant chaque transfert ou lien exclusif,
horloges PC interprétées sur PC et durée restante bornant le travail local.
SIG/erreur/timeout/fin de phase doivent arrêter ce relais et préserver le premier
rouge et les sorties brutes. Aucun ACK posthume ni réactivation P02.

Protocole et témoins factices du VRAI moteur de relais publiés avant implantation.
Aucun test n'est autorisé implicitement par cette préparation ; la revue doit
porter sur le couple relais/pilote, les pins et un budget correctif distinct.
Le carry de l'ELF0c565 compilé sous fec est seulement proposé avec chaîne de
provenance, END précédent et propriétaireBASE ; il ne sera jamais sélectionné
implicitement à la place d'une compilation prévue par un GO.
