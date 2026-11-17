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

## Vingt témoins CPU verts782b, trois réserves groupées RO avant correction

GO C7-PARSERS-RELAY-782B-17nov60, source exacte782b propre/poussée, UNEfois :
20/20vus exécutés/0échec/erreur/skip,rc0 en0,094184s externe ; bruts+manifest
2hôtes,manifest2a89eb01fdc1e3ffa25652f365ee9cf9d03662e7bf1bbaee2e013daf9e463567.
Rapport rust-probe-relay/02-resultats-cpu-et-reserves-ro.md.

Banc1cdd0babda relève trois défauts non couverts, inscrits avant correction :
1. Pas de TTL distant au SSHlaunch et abort non actif impossible : attente poste
   distante non attestée close en tuant seulement le SSH local.
2. PIDrelay-launch publié avec argv futur avant exec : course normale de snapshot.
3. Retour0 après finish/fsync sans recheck de phaseDeadline ; ITIMERsoft seul
   insuffisant pour erreur/cleanup. Refus relais tardif distinct du natif fermé.

À corriger ensemble avec témoins des vrais chemins prépubliés, puis RO totale
couple/pin et nouveau budget CPU C7 distinct. P03 non armé, P02 reste rouge.
Choix du futur protocole : Cargo --no-runJSON sur FEC/ownerBASE, cache permis,
pas de modecarry custom ni compilation forcée. Pas de nouvelle phase autorisée.

## Lot rouge dda9 : deux défauts observés, puis erreur de fixture, STOP

Sur dda9b7ea4d16f0f4b55e9233943a404a2eb6d381 publié/propre, lot CPU distinct
borné58+2 : launch sans TTL distant et abort queued exigeant actif sont chacun
vus FAIL unefois. Le cas bootstrap rencontre ensuite ERROR KeyError tool_root
manquant dans sa Fakepair, avant assertion comportementale. STOP immédiat,
aucun cas finish joué et aucun correctif produit encore appliqué.
Bruts+manifest conservés2hôtes sous rust-relay-red-dda9b7ea4d avant correction ;
lot0,293424s, pas de SSH/proc réel/Cargo/natif/autorité pendant le lot.

Correction de fixture seule : ajouter la clé factice tool_root requise par le
vrai Gate.launch. Pas de relance ; un GO CPU rouge complémentaire distinct est
demandé pour les deux cas restant à constater. Les deux rouges acquis ne seront
pas rejoués sous cette demande. Correction produit groupée encore différée.

## Rouges complémentaires57fac observés avant correctif produit groupé

GO C7-PARSERS-RELAY-57FAC-17nov60, source exacte57facb0b8b4ffaa09645948d0f327b99117a04c5
propre/poussée, deux cas restants chacunUNEfois sous58+2 : bootstrap publie
futureargv vu FAIL ; finishlate retourne0 vu FAIL.0ERROR/PASSinattendu,0,179711s.
Les deux FAILdda9 ne sont pas rejoués. Bruts+manifest2hôtes AVANTfix,
manifest62abd2eb60ce11324055510d0df7386c81f2a43725937efffe9e42c53e963699,
result3f693d1d2237e07caa09018cb1aac025eeb9389870136239ad13a1bff56ecc2f.
Les trois réserves/quatre témoins sont désormais vus rouges ; aucune réussite
corrective n'est encore revendiquée et aucun nouveau lot CPU/natif autorisé.

## RO totale64d7d45597 : deux restes groupés avant nouveau correctif

P04prepared7fec/profil81969/10copies conformes. Bootstrap après exec, hard2098+2
et fermeture postclose reconnus statiquement. Aucun nouveau CPU vert cf832 joué.
Deux défauts de fermeture sont consignés AVANT modification :

1. Relay.run except calcule cap() déjà expiré avant abort : aux expirations de
   file/publication, l'appel d'arrêt distant n'a jamais lieu.
2. Gate.cancel initialisealive=[] et ne relève les rôles que dans while(now<end).
   Un budget déjà consommé avant le premier tour peut donc annoncer
   launcher_absentTrue sans aucun relevé final, notamment queued sansscope.

Prépublier témoins des vrais run expiry/queued et cancel clock épuisée, puis
budget CPU rouge distinct C7, fixgroupé et nouveau CPU vert/ROtotale/pin.
Réserver l'arrêt DANS les bornes déjà proposées (queue300/phaseEND/hard2100),
ne pas renouveler le budget opératoire. Un relevé final vivant/UNKNOWN doit
refuser indépendamment du fait que la boucle de délai ait pu s'exécuter.
P02 consommée, autres préparations non armées. Aucun GO natif déduit.

## Trois rouges598f acquis avant correction des deux restes

GO exact C7-PARSERS-RELAY-RED-598F-17nov60, HEAD598f3851227071eb61241949ba56cae7a69d3844
propre/poussé. Trois cas ciblés chacunUNEfois-f sous58+2, trois FAIL attendus,
0ERROR/PASSinattendu,0,801432s externe. Les quatre anciens rouges ne sont pas
rejoués. Bruts+manifest2hôtes avant tout correctif,
manifest90fbc0af8a6c3fbd78c6cd9ba000f0397af97944f16ec4c452f532ab1ecb9546,
resultc4d79cd1ffb93b5e0806688fdb549522067b1258449d7131c0a3966a20a407df.
Aucun GO vert/natif implicite ; préparation de correction groupée seulement.

## Lot vertD45 : trente-six témoins CPU et RO favorable préparation

GO C7-PARSERS-RELAY-GREEN-D45-17nov60, source exacteD45 propre/publiée, UNEfois
commande58+2/-f depuis dossier publié :36/36exécutés,0FAIL/ERROR/skip,rc0,
0,187111s externe/0,095s unittest. Bruts+manifest2hôtes
~/.cache/codeparsers-notes/rust-relay-green-d45aefbdf8/,
manifestd8270ccf4d7ddb9ac1a425dbee6da5fc673e8928ea95e16bac27631df6688e0f.
Rapport03-resultats-qualification-cpu.md sur brancheDOCdistincte, D45/P05inchangés.
RO TOTALE BancP05favorablePRÉPA reçue ; pas de qualification SSH/native ni GO.
Tous rouges précédents et erreur de fixture conservés. Ce ticket reste ouvert
jusqu'à qualification du raccord réel sous allocation distincte ; P02-P04 non
réarmés, P05prototype jamais armé. Aucun carrycustom ni globale automatique.
