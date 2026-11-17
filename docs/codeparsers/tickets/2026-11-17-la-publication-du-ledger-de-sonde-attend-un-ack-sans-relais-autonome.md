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

## Proposition commune demandée par Archi, hors outillage D45 figé

Après D45/36CPUverts et RO favorable P05, Archi demande une proposition de
point d'extraction commun pour quatre publications global/D6 (5ELF puis3x1),
sans troisième fork. Préparation documentaire/interface sur branche distincte
cx-codeparsers-publication-contract issue deD45 ; aucune modification D45/P05,
aucun run/CPU/SSH/natif ni extraction runtime exécutée. Ticket consommateur :
docs/rag3weaver-archi/tickets/2026-11-16-le-temoin-documentaire-vertex-attend-un-accord-sans-superviseur.md,
lu dans son worktree documentaire, avec ses admissions séparées.

Le contrat proposé doit préserver le plan ordonné et le verrou de phase, la
liaison source/ledger/ELFmap, les copies deuxhôtes et fsync/link avant ACK, les
END et réserves, le bootstrap aprèsexec et l'arrêt attesté. Rust garde son plan
strict build/runtime par défaut dans son adaptateur. Le module commun ne peut
être remplacé par une boucle de copie spécifique D6 ni des inventaires factices.
Le présent travail propose seulement l'API à relire et son emplacement, pas une
primitive déjà implémentée ou qualifiée pour D6. Contrat/sourcepins futurs et
budgets distincts nécessaires avant extraction, tests ou calibration réelle.

## Extraction commune autorisée C7, avant code

AccordPRÉPA1cc2b726c3 après API2c8d/manifestArchi7416. Codeparsers est porteur
unique de scripts/publication_relay.py. Branche neuve
cx-codeparsers-publication-core, D45/P05/Archi229bQ figés. Déplacer les vraies
boucles, gardes et arrêt dans un seul module ; wrapperRust sans seconde copie,
36assertions conservées. Séparer refactorconstant de capacité4publications.
Nouveaux contre-exemples du vrai D45 prépubliés avant cette capacité ; code et
tests à publier avant toute RO/GO CPU distinct, aucune exécution implicite.

Arbitrage global :7440TERM+15KILL avantqueue/ARM, queueMAX120DANSwhole ;
CommonMAX7200 et MINsoldephysique−30, ENDopCommon−15/réserve15DANSCommon ;
fenêtres90owneravantwait/MINENDop, aucun reset global ni comparaison absolue
interhost. ValeursRust inchangées. Raccord WindowEvent explicitement sous
PreparedContext.window_events approuvés et Lifecycle commun ; providerlecture
seul ne peut attesterliveness. Les sources et capacités futures sont à relire
avant adaptation/native ; aucun nouveau CPU/SSH/proc/natif/Cargo/target.

### Étape constante préparée et contre-exemples déjà prépubliés

Publication00e2084673 avant refactor : trois contre-exemples du vrai D45 et
contrat WindowEvent clarifié (sourceCoreBoundary, pas de noyau factice).
Étape suivante matérialisée dans scripts/publication_relay.py : Relay/transport,
GateBase/gardes/cancel/final_roles et vrais CLI déplacés ; wrappers Rust sans
boucle, fournisseur de lecture séparé ; onzième composantQ épinglé. Les36témoins
conservent leurs assertions avec points de patch sur le vrai module partagé.
Aucun CPU joué, capacité4/WindowEvent non implémentés. RO source/pin et budgets
CPU distincts nécessaires avant exécution, puis capacité seulement après rouges
et nouvelle publication ; D45/P05/ArchiQ229b inchangés.

### C9 constante verte et trois rouges avant capacité, preuves préservées

GO C7-PUBLICATION-CORE-C9-CPU36-17nov60 :36PASS/0FAIL/ERROR/skip rc0 .217897s,
manifest2hosts6fc717365b4fcf1d01476fd1e3a6ec458e4420a1d06decd9a5408609e3ee560d.
EnsuiteGO conditionnel distinct C7-PUBLICATION-CORE-C9-RED3-17nov60 armé après
copiepreuve2hosts/C9P06inchangés :3FAIL attendus/0ERROR/skip rc1 .122832s,
manifest2hostsce154c521ded16e3fe1e9a8999a0486769f89b84dd2da35b427b76396e63573d.
Rapport publication-core/02-resultats-constante-et-contre-exemples.md.

BrancheNEUVEcx-codeparsers-publication-capacity depuisC9 pour consignerAVANT
code et préparer la capacité ; C9/P06/D45/P05/Archi229bQ figés. Aucun CPU vert
nouveau ni native/transport qualifié par ces contre-exemples. L'expectedbinding
est un contrat statique ; ne jamais le confondre avec le SHA des faits observés
ou d'un patch futur. Plan deepfreeze, refs vers seuls ACK antérieurs et tous
modules épinglés restent des garanties à implémenter/relire sur le vrai noyau.

### Prépublication de huit témoins WindowEvent avant implantation

Après accordC7c7cabacd87 en continuité1cc2, ajouter les contre-exemples de
fenêtrage/owner/gardes avant helpers s'ils sont absents : huit cas du vrai Gate
réutilisent la fixture d'identités/clockD45, sans noyau factice/OS/provideractive.
API absente signalée FAILnommé ; rien encore exécuté. Types/validateurs de plan
commencent la préparation de capacité, mais aucune boucle4/Windowruntime livrée
à ce point. Nouveaux budgetsCPU/RO distincts nécessaires avant essais.

### Publication intermédiaire du plan consommé, sans nouveau résultat

Après trois rougesC9 préservés, préparation du rejet de fichiers supplémentaires,
des ACK hérités et du plan ordonné dans la seule boucle commune. DefaultRust
construit dans son wrapper, Gate/transport IDs partagés, bindings attendus et
faits observés séparés. Fixtures portées vers plans complets/headers, assertions
36+ordre/cardi5+1+1+1 conservés ; aucun CPU joué de ces changements.
Window runtime/global lifecycle restent absents, bridge non livrée/admission
impossible. WTrouge7ee figé attend GOdistinct huit casWindow avant helpers.
Aucun Native/SSH/proc/target pendant ces modifications, D45/P05/P06/229bQ intacts.
