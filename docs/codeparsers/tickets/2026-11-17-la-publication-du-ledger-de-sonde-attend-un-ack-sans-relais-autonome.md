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

### Huit rougesWindow7ee observés AVANT helper runtime

GO C7-PUBLICATION-WINDOW-RED-7EE-17nov60, source exacte
7ee63776bbbd07f16bc854054281157ba2caae53/WTrouge propre/publié, UNE invocation
58+2 sansfailfast :8FAIL WINDOW_LIFECYCLE_UNSUPPORTED_BY_REAL_CORE,
0ERROR/skip,rc1,.094719s externe/.010s unittest. Bruts2hôtes avantimplantation,
manifest74fdc8cabd5d4cf96b565f19fd672ed8c631d440c85e64be624ca5b5e510c27d,
result020ea366206b83a887e37c39e0e6c04a6ab89e814d417fdf83da15b56453b062.
Aucun signal/process/proc/SSH réel parfixture ni Native/Cargo/target/corpus.
Helper commun préparé ensuite seulement, aucun nouveau CPU vert autorisé.

### Reprise de préparation après confirmation C7

Les helpers Window et limites restent WIP non exécuté. Porter les données des
fixtures vers les limites explicites et le codec de descripteurs Common, sans
remplacer les gardes du noyau ni changer les assertions historiques. La lecture
TTY confirme une frontière : copie/hash/ACK ne prouvent pas à eux seuls la copie
effectivement exécutée via PID/start/exe pré/post ; aucun producteur générique
de cette preuve n'est livré ici. Cette extension ne devient pas une nouvelle
implémentation TTY. Publier les pins puis RO avant toute demande CPU distincte.

Point ouvert de préparation : CLI/chemins/roles/bootstrap restent orientés Rust ;
le layout global P/Q/S d089 est une proposition documentaire, aucun bridge
armable. Revoir également les clamps queue/Common au budget physique original
et la fermeture des modules avant qualification. Publication intermédiaire ne
vaut pas demande d'exécution ni clôture du ticket.

### Assemblage source repris sur demande C7 après les RO communes

F494 reste intermédiaire non exécuté. Finir dans le noyau unique le layout
déclaré (prepared, racines de phase/peer/scope, fichiers de rendez-vous, entrées
par rôle), CLI sans labels Rust et vérification de tous les composants avant
leur import/utilisation. Les codecs Rust construisent des descripteurs/argv,
jamais une décision de vivacité. Garder les mêmes boucles et gardes communes.
Porter les fixtures existantes sans changer les assertions ; une insertion de
recette répétée hors setUp repérée en lecture est à retirer avant tout jeu.
Les adaptateurs globaux ne sont pas inventés : le layout d089 et les limites
restent une proposition à raccorder aux producteurs réels sous revue distincte.
Aucun test, import de code de lot, compilation, observation proc ou activation.

Candidat assemblé en source : layout générique P/Q/S, prepared configurable,
CLI IDs de plan, rôles/argv indépendants, closure modules avant import,
annulation mêmes chemins/observations, Common demandé depuis watchdog original
moins marge. Documentation publication-core/03-layout-et-admission-source.md ;
pins source/test publication-capacity-pins.json. Aucun import/parser/test joué.
Revue totale puis GO CPU distinct nécessaires ; rôle global/adaptateur/Q réel
encore absents, aucune qualification ni fermeture du ticket.

### RO totale Mémoire du candidat exact 558959a8de : trois réserves

Constat source reçu avant correction, sans import/parse/test :

- R1 : common_request_budget ne borne que le watchdog whole. Avec démarrage
  phase t0 et demande Common t60, 1750 s dépassent ENDphase1798. Borner la
  demande par les deux watchdogs et garder leurs marges avant admission.
- R2 : après finish/stop, publication finale contrôlée par phase_deadline seule.
  Conserver MIN(phase_deadline,hard_end local), y compris fsync et close finaux.
- R3 : le wrapper relay.py importe/exécute publication_relay avant verify_components.
  L'affirmation de fermeture avant import n'est donc pas établie pour le moteur.
  Prévalidation indépendante des pins avant exec_module ou contrat externe
  explicite à revoir, y compris le chemin d'import Gate.

Les huit SHA concordent et les méthodes36/3/8 existent ; cela n'est pas un
verdict CPU. Préparer les témoins réels groupés AVANT corrections, puis revue
du pin de rouges et GO CPU distinct pour observer ces refus. Le candidat558959
reste figé ; nouvelle branche guard-red réservée aux témoins et à ce constat.
Aucune des réserves n'est corrigée ni réputée levée par cette consignation.

Trois témoins groupés prépubliés dans publication-core/
test_publication_guard_counterexamples.py sur moteur558959 INCHANGÉ : demande
Common à phase+60, fermeture locale après hardEND mais avant phaseEND, et
remplacement des seuls octets du moteur avant import wrapper. Aucun import/test
joué. Le dernier cas ne convertit que la sentinelle exacte d'exécution non
épinglée en FAIL ; toute autre exception de montage reste ERROR. Rien corrigé.

### RO totale TTY558 reçue en quatre parties, avant nouveau correctif

Trois réserves groupées supplémentaires/étendues, sans exécution :

- Horloges : start ticks de procstat sert d'identité PID, pas d'origine garantie
  CLOCK_MONOTONIC. Linuxv6.18 fs/proc/array.c utilise start_boottime, BOOTTIME
  inclut la veille alors que MONOTONIC ne l'inclut pas. Les calculs de
  common_request_budget/starting/active/launch/relay_main et pilot.compose
  mélangeant ticks/CLK_TCK et monotonic sont à remplacer par une enveloppe
  d'origine attestée dans le même domaine. Prépublier un témoin de décalage
  avant correction ; les clocks synthétiques égales9000 ne couvrent pas ce cas.
- Fermeture avant import : confirme R3 Mémoire et l'étend aux entrées Gate et
  pilote, qui évaluent relay/moteur et legacy/physical_guard avant vérification.
  Un prévalidateur externe absent ne peut être supposé pour une CLI autonome.
  Prépublier les cas top-level altéré sur ces vraies entrées/helpers Q, garder
  l'ancrage de confiance explicite et aucune seconde boucle métier.
- RPC/récolte : timeout distant soft(T−.1)+KILL1 peut dépasser T, communicate(T)
  est suivi de TERM.5/KILL.5 dans reap ; stop final n'a pas d'échéance originale
  transmise. Réserver grâces/récolte DANS les bornes et propager la même fin.
  Prépublier un témoin fakePopen avec récolte lente, préserver première cause
  et UNKNOWN quand absence non attestée. Aucun faux VERT physique établi.

Sources primaires citées par la revue : Linuxv6.18 fs/proc/array.c do_task_stat,
man7 clock_gettime CLOCK_BOOTTIME/CLOCK_MONOTONIC et docsPython3.14 time.monotonic.
Pas de qualification du noyau ou de l'horloge de l'hôte par ces lectures.
Le pin9e653 et ses trois témoins restent inchangés, toujours NONJOUÉS ; ces
nouvelles réserves ne sont pas couvertes par leur seule sélection. Attendre
cadrage C7 des témoins complémentaires et budgets distincts, aucun fix anticipé.

### Cadrage C7 groupé, avant prépublication complémentaire

Conserver 558/9e653/842 figés et NONJOUÉS, pas de GO3 immédiat. Nouvelle branche
guards-grouped depuis842 pour sept témoins complémentaires : deux horloges
(start PID décalé BOOTTIME, bornes MONOTONIC indépendantes), trois imports
(Gate/moteur, pilote/legacy, pilote/physical_guard), appel RPC/récolte lente et
stop sans échéance transmise. Dix cas au total avec les trois précédents
INCHANGÉS, noyau558 INCHANGÉ. Aucun module de remplacement pour les boucles ;
seuls octets adverses, clocks/process records et Popen fictif aux frontières.
Les champs d'enveloppe d'origine des fixtures sont une proposition de contrat
à relire, pas une attestation produite sur l'hôte. Prépublication/pins puis
RO ciblée M/TTY avant GO CPU distinct. Aucun fix anticipé ni effet OS/SSH/natif.

Paquet source groupé écrit : test_publication_guards_grouped.py charge
explicitement les trois cas inchangés et sept nouveaux (10 exacts). Document
04-rouges-groupes-avant-correctifs.md et pins groupés décrivent les montages,
les enveloppes CLOCK_MONOTONIC proposées et les limites de ce modèle. Le moteur
et ses contrôleurs sont inchangés par rapport à558 ; pas de correction cachée.
Prévision dix FAIL non observés, zéro résultat CPU revendiqué. RO ciblées M/TTY
et cadrage C7 exigés avant une invocation future de ce seul paquet.

### GO ca1 consommé, motif de fixture inattendu : STOP sans correction

C7-CP-GUARDS-RED-CA1C-17nov60/d72aa, UNEinvocation ca1 exacte/propre, dixSHA
et Python frais. 10FAIL/0ERRORskip/rc1,.150396768s externe/.048unittest, MAIS
9motifs attendus et1FAILfixture : Gate.pin reste1*64 tandis que helper écrit
preparedSHA réel ; read_bootstrap refuse avant la comparaison clock visée.
Pas dix rouges précis, donc pas autorisation corrective conditionnelle acquise.
STOP0retry/0fix ; sources558/9e/842/ca1 restent figées. Rapport
publication-core/05-rouges-groupes-arret-sur-fixture.md. RAW+source+pins+argv/rc/
temps et manifeste16fichiers réhashés deuxhôtes AVANTtoutdiff documentaire :
manifest41c73731458fd15c1003c378257a38f8a8c326adfb64c02a6c79df2a29874b93.
Attendre cadrage C7 du raccord de fixture, nouveaux pins/RO/GO distinct avant
rejeu. Les défauts moteur restent ouverts. Nuance premièrecauseRPC consignée :
TimeoutError générique accepté ne prouve pas origine/immutabilité de la cause ;
renforcer ce témoin avant futur correctif/vert qui revendique cette propriété.

### Cadrage C7 a030139580 : préparation corrective de fixture seule

Preuves CA1C seize fichiers réhashés deuxhôtes et rapport33cf conservés AVANT
ce delta. Affecter base.gate.pin au même SHA réel déjà écrit dans bootstrap et
retourné par gate_fixture. Garder toutes les assertions et les dix cas ; moteur
558 inchangé. Mettre à jour uniquement pins/cwd/source du paquet et note de
reprise ; RO ciblées M/TTY puis nouveau GO distinct avant toute invocation.
Aucun retry CA1C ni correctif moteur autorisé par cette préparation.

### 8792 : dix rouges causaux conservés avant correction groupée

GO C7-CP-GUARDS-RED-8792-17nov60/dd1e consommé UNEfois source8792propre,
10FAIL précis/0ERRORskip/rc1,.268373769s externe/.094unittest ; clockatteint
starting761 aprèsSHAcohérent, pasleFAILfixtureCA1C. Dix-sept fichiers réhashés
deuxhôtes AVANTtoutdiff : manifestf7c622667ff7552402843e35a96fb79337defab907b68eb66c29f781cc86d074,
result2aad81d4b8b43cd7b571a7c0902b7b2b60d27d5f0932b730c16ad3bcc05bd5b5.
Rapport publication-core/07-dix-causes-rouges-conservees.md. Préparationcorrective
source ouverte conditionnellement, aucun vert/GO d'exécution supplémentaire.

Avant diff moteur, prépublier le témoin causal RPC complémentaire : communicate
TimeoutExpired fixture-ssh suivi d'échecs de reap fixture-reap ; l'exception
originale doit rester authentique, erreur de cleanup séparée/UNKNOWN attesté.
Cas ajouté sur moteur558 INCHANGÉ, sans aucune exécution. Observer son rouge
sous nouveau GO ciblé après revue, avant correction de cette causalité ; les
dix rouges8792 restent figés sans rejeu implicite.

### Rouge causal RPC conservé avant correction moteur groupée

GO C7-CP-RPC-FIRST-RED-F3D0-17nov60/003950 consommé UNEfois sourcef3d exacte
propre/pins13+Python frais :1FAILprécis assertIS objet fixture-reap≠objet original
fixture-ssh,0ERRORskip/rc1,.175979993s externe/.010unittest.19payloads+manifest
(20fichiersTOTAL) réhashés deuxhôtes AVANTtoutdiff : manifest
8c2fa5bc8e3b174ee98a827f665c68a3b181208624db29af4b4a82916be7f956,
result71cb48ee54647567b221b29a720ab74778650e02c2237af15e3c5c9478622352.
PréparationgroupéefixM/TTY ouverte ; aucuneexécutionverte autorisée. Origine
temps indépendante, deuxwatchdogs/admissionCommon, hardENDpublicationfinale,
fermetureavantimports et RPCgrâces/reap/stop/premièrecause/UNKNOWN restent les
critères source. Sourcef3d/8792/ca1 etpreuves restent figées. Les comptes
précédents17/16 désignaient les PAYLOADS :8792=18TOTAL,CA1C=17TOTAL.

### Candidat SOURCE correctif groupé, aucun nouveau test

Implantation préparée après preuves8792+f3d conservées : origines MONOTONIC
avant watchdog, liaison PID/start identité seule, deuxbornes Common, finalEND
MINlocal/phase, loader octetsépinglésAVANTimports/mêmesbytesexécutées, RPCgrâces/
récolte/stopdansEND et mêmeobjetpremièrecause+erreursecondaire/UNKNOWN. Loader
commun est une nouvelle primitive : une fermeture et un snapshot de bytes pour
relay/Gate/pilot, coût1contrôleur+préludesépinglés ; pasdeuxièmebouclepublication.
Fixtures portées avec vrais loader/pins/enveloppes ; assertpréfixetimeout conservé
à la frontière réelle arm→exec, pasretiré. Bundleproposé58=36+3+8+10+1.
Rapport09/pinscorrection pour RO TOTALE avant GO CPU distinct, aucun import/parser/
test exécuté. Source/modèles ≠qualificationorigine/OS/SDK/global6rôles/portLIVE.
Le ticket reste ouvert ; les défauts ne sont pas déclarés corrigés par ce code
nonjoué. Tous les anciens Q/source/proofs figés, aucun BASE/CAND/target/corpus.

### RO totale TTY e15 : première cause encore masquée par erreur de journal

Revue cumulative source/diff/loader/fixtures/bundle/doc09/pins16, sans exécution ;
les seize SHA correspondent et les améliorations RPC/clock/closure sont lues,
mais aucun avis global favorable. Réserve résiduelle avant correction :

- Relay.run except écrit first-red avec seul FileExistsError capturé ; un
  OSError de disque masque l'erreur initiale et saute abort avant finally.
- Le finally écrit local-stop-refused avec la même fragilité ; un échec de ce
  constat remplace failure alors que la cause initiale est déjà acquise.

Consigner puis prépublier les deux contre-exemples sur le VRAI Relay.run et
une panne sélective d'atomic_json (premier journal / constat de stop), avant
fix. Préserver le même objet d'erreur, tenter le cleanup malgré journal absent,
conserver les secondaires/UNKNOWN sans prétendre qu'une écriture a réussi.
Le candidat e15 reste figé/non exécuté ; attendre également les autres RO
totales pour regrouper les correctifs, puis nouveaux pins/RO/GO ciblés avant
rejeu ou vert. Rien corrigé ou exécuté par la présente consignation.

### Fin des RO totales M/TTY e15 : deux familles regroupées

Mémoire confirme les seize empreintes/source/disque/Python et l'absence de
jeu, mais demande deux corrections avant tout GO58 ; TTY ajoute le même
chemin stop/close à sa réserve causale. Regrouper AVANT diff correctif :

- Prepared : verify_entry hashe digest(prepared), puis rouvre read_text pour
  json.loads. Les données déterminant la fermeture peuvent différer des octets
  hashés. Lire une seule fois, hasher et décoder le MÊME snapshot. Prépublier
  substitution du second read sur la vraie fonction, sans thread ou effet OS.
- Première cause : aux deux journaux Relay.run s'ajoute SSHTransport.stop :
  failure de reap est mémorisée puis handle.close non protégé peut la remplacer.
  Prépublier aussi ce cas ; conserver l'objet initial et les secondaires/UNKNOWN
  malgré l'impossibilité d'écrire un constat. call seul ne suffit pas à composer
  cette garantie jusque stop et Relay.

Quatre témoins ciblés à écrire sur moteur/loader e15 INCHANGÉS : prepared
second-read, first-red journal, local-stop-refused journal, reap puis close.
Les assertions d'identité d'objet et de cleanup ne sont pas des marqueurs de
réussite d'écriture ; secondaires en notes/UNKNOWN si disque refusé. Rien encore
exécuté. Source/pins/RO ciblées avant GO distinct ; aucun58/ancienrejeu implicite.

Cadrage C7 HOLD58/3a4c24b3ca reçu : familles regroupées avantfix, sourcee15
INCHANGÉE. Ajouter le constat cleanup-refused après échec abort aux deux
journaux Relay : total CINQ cas ciblés (prepared1, Relayjournaux3, stopclose1).
Le futur correctif du lecteur préparé doit servir aussi relay_main/gate_main/
pilot.main, sans rouvrir un JSON différent après pin ou contexte prévalidé.
La seule conformité d'un préimport ne lie pas une relecture ultérieure.
Contrat de secours proposé pour les témoins : failure_record en mémoire garde
first_error/secondary_errors/remote_quiet UNKNOWN avant toute écriture ; même
objet exception réémis. Ce record n'est PAS une archive réussie ou un reçu LIVE.
Aucun patch du moteur/loader avant rouges précis conservés et nouveau cadrage.

### Cinq rouges résiduels exacts conservés avant fix unifié

GO C7-CP-RESIDUAL-RED-6F47-17nov60/d10a consommé UNEfois source6f47 propre/
pins18+Python frais :5FAIL5causes précises/0ERRORskip/rc1,.235670384s externe/
.036unittest. Quatre assertIS surmasquages journaux/close ; JSONB nonlié au
SHAa. Vingt-quatre payloads +manifest=25fichiers réhashés deuxhôtes AVANT
toutdiff ; manifest68d76af23699734d51df594d9cd71343dfbdb9d6a47f598c141d2e5fc3d1e42a,
result778c1434d1c421511536bf0188c97310dc60ca4a40ea30a769fe66566786b2a7.
Rapport11. Préparationcorrectivegroupée SOURCE ouverte : read_prepared commun
bytes/hash/JSONmêmesbytes pourloader/relay/gate/pilot, capturepremièrecause
AVANTIO etsecondaires/UNKNOWNmémoire qui n'empêchentpasabort/stop. Les63cas
proposés restent NONJOUÉS jusqu'àsources/pins/ROtotale etGOvertdistinct.
Ancienssources/preuves/Q restentinchangés, aucun35/58/anciensrejeu ni Native.

### Candidat SOURCE du fix unifié après cinq rouges6f47

read_prepared partage un seulFD/payload pourSHA/JSON, réutilisé loader/relay/
gate/pilot ; contexte/pin indépendants liés, paspinpromu depuisfichierdécouvert.
capture_failure/persist_failure conservent premierobjet+record mémoire avantIO ;
journaux/abort/stop/reap/close/échéance gardentsecondairesUNKNOWN sansmasquage et
continuentcleanup. Notes imbriquées conservées, pasfauxsuccèsarchivedepuismémoire.
Témoins5INCHANGÉS, sélectionseule58→63. Rapport12/pinssnapshot-cause pourRO
TOTALE avantGOvertdistinct. Aucun import/parser/test effectué, aucunequalification
Q/global6rôles/LIVE/native/SDK ni fermeture du ticket.
