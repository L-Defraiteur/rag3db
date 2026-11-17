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


## RO totale Banc du prototype782b/pair03

[Rapport secondaire](../../banc/17-11-2026/01-ro-relais-sonde-rust.md) : dix pins conformes, Relay/Gate/delta pilote relus ensemble ; trois réserves groupées avant phase : TTL/arrêt de la file distante avant Common, handoff argv futur publié avant exec, fin locale sans recheck après publication et hardborne cleanup. Porteur Codeparsers ; témoins des vrais chemins à prépublier puis correction/ROdelta sous décisions C7. Aucun test ou processus réel lancé par Banc, P02 toujours consommée rouge, carry reste proposition non branchée.


### Banc — cf83228/P04, 17 novembre

[Relecture du correctif](../../banc/17-11-2026/01-ro-relais-sonde-rust.md) : P04 préparé identique deuxpostes/10copies chacun/profil81969 vérifiés sans exécution. Bootstrap aprèsexec et watchdogs physiques reconnus corrigés. HOLD natif : Relay.run coupe son RPC abort via cap expiré avant appel ; Gate.cancel peut publier launcher_absent depuis alive=[] si sa boucle finale n'a jamais pu jouer. Correctif/témoins groupés attendus ; aucun test rejoué, aucun GO déduit, ticket OUVERT.


### Banc — d45aefbd/P05 après correction des deux restes64d7

[RO totale favorable préparation](../../banc/17-11-2026/01-ro-relais-sonde-rust.md) : cleanup_cap distinct borné dans queue/phase/hardEND, réserve5 et UNKNOWN si épuisé, pas de réouverture nativeclose0 ; final_roles obligatoire horswhile/statusUNKNOWN et launcher_absent0alive0unknown. P05 prepared20d59962/10copies PC+local chacun/profil81969 conformes.36témoins NONJOUÉS par Banc ; GO CPU vert exact C7 puis natif distinct requis. P02–P04 nonréarmés/carrynoncâblé/BASEfecCANDc1cd conservés, ticket OUVERT.


### Banc — preuves CPU D45 vertes

[RO des bruts sans rejeu](../../banc/17-11-2026/01-ro-relais-sonde-rust.md) : manifestd8270ccf/4fichiers taillesSHA conformes PC+local,36OK/0FAIL0ERROR0skip/rc0/unecommande T58+K2. CPU hermétique36 qualifié sur D45 ; raccord SSH/natif et phases BASE/CAND non qualifiés/non autorisés. P05 reste préparation seule, ticket OUVERT.
