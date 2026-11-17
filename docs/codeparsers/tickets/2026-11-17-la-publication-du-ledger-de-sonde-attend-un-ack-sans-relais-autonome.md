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
