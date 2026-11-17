# L’admission du corpus FS ne garantit pas la vue consommée

- **État** : ouvert (16 novembre 2026), découvert pendant la reprise native après les neuf juges mémoire.
- **Gravité** : moyenne ; une réussite de test peut masquer une vue vide, des lectures perdues ou un index réutilisé.
- **Famille** : validation
- **Pour** : lucivy
- **Atteignable en service** : non ; collecteurs et builders des tests de corpus.
- **Touche rag3weaver** : non directement.

## Parents et preuves

Parent qualification core : [profil core et diagnostics non isolés](2026-11-16-le-profil-core-par-defaut-inclut-des-diagnostics-non-isoles.md).
Parent lot et suivi : [indexed:false](2026-11-16-indexed-false-est-ignore-pour-un-champ-texte.md).
Les neuf juges mémoire de1ee5/6c701 sont verts et relus (preuves9d6451f721,
RO8040fe84a9) ; ce ticket ne les ferme pas et ne les rebaptise pas admission FS.
L’ancien correctif du corpus absent traitait l’absence de racine, pas ce nouveau
contrat de vue/erreurs/panel avant builder ; aucun ticket clos n’est rouvert.

Source1ee5, audit CODE+archives seulement, zéro contact corpus/ELF/Cargo :
- derived_in_ram lit le préfixe300 trié Linux2.6.0 fixé, historiquement sans
  mutex_lock ; son gate11 refuse avant builder. Jeu fixé NONadmissible pour
  parité, pas de nouveau run de refus prédit ni de sélection posthoc.
- v3_ground_truth_contains collecte au plus5000 fichiers dans read_dir nontrié,
  ignore erreurs d’énumération/métadonnées/lecture/UTF8 et peut retourner avec
  une vue vide. Arbre clone8507 empreinté mais vue ordonnée5000 nonobservée.
- Son builder est appelé AVANT les15requêtes du panel (distinct des11derived).
  run_panel peut compter0=0 commeOK. Ses oracles relisent le FS et utilisent
  skip/lossy/unwrap_or_default. Les octets comparés ne sont pas garantis être
  les mêmes que ceux indexés.
- V3_INDEX_DIR peut rouvrir un index avec une clé faible chemin/compte/options,
  ou supprimer/persister à un chemin fourni. Ce chemin est exclu du futur
  parcours de correction ; aucune suppression/reutilisation pour faire vert.

Voir recette/contrat16 et pins historiques sous
`../16-11-2026/preuves/native-preparation-1ee5/`. Aucune observation FS récente
ou vueeffective n’est inventée pour compléter ces pins.

## Contrat demandé par C7 avant correction substantielle

Vue ordonnée explicitement figée, identités/ordinals et empreintes de contenus
liés à une origine admise ; chaque document sélectionné lu UNE fois, erreur IO,
empreinte et UTF8 fatales. Un snapshot immuable sert au panel ET au builder ET
aux comparaisons ; aucun oracle de correction ne rouvre une racine mutable.
Refuser la vue vide et toute requête noncouverte AVANT scratch/index/report.
Le panel15 reste celui du cas natif, sans l’échanger contre11 ou modifier des
requêtes après résultat. Builder neuf sans capacité de reuse/persist/delete,
V3_INDEX_DIR et modes qui relâchent les assertions refusés avant effets.

Préparer quatre témoins red-first : vue vide, lecture échouée, UTF8 invalide,
panel noncouvert/0=0. Ils doivent atteindre le vrai raccord FS du consommateur,
avec backend stdfs pour le natif et spy déléguant les mêmes opérations pour les
petits fixtures/spies ; pas un helper isolé testé à la place du chemin réel.
Ordre figé, read-once, zéro builder avant admission et zéro appel reuse/delete
sont observables dans les mêmes témoins et dans un contrôle positif distinct.

## Travail et fin

Porteur lucivy ; RO Recherche sur contrat/raccord/spies avant implémentation
substantielle. Préparation seulement : aucun contact corpus, aucun ELF/Cargo,
service, Linux7.2 substitué, sourcefix substantiel, tag/main/PR par ce ticket.
Après RO, extraction de raccord préservant la baseline puis publication des
quatre témoins et vrai rouge avant correction, selon nouveau GO borné.
Fin : erreurs/absence nonconfondues au vrai consommateur, preuves des quatre
rouges puis verts, snapshot/panel admis avant builder et aucun effet persistant,
puis recette native distincte avec nouvelle source/ELF si le code a changé.
Les ELFs1ee compilés restent des pins historiques, pas une validation future.
Core complet,9HOLD/FS/ULP7.2/PR42 restent nonqualifiés jusqu’à preuve adaptée.

LivrablePREPA16+pinsA/Bséparés+contratJSON4vacuïtés préparés : toutmetadata
historique,aucunRootcorpus/ELF/contactPC/proc/Cargo/testRust/service. Action
RORecherche duCONTRAT/raccordconsommateur/StdFsSpy/4vraisrougesAVANTimpl
substantielle. Les signatures du plan sont proposéesNONimplémentées, pasAPI
existantefactice. C7 tranchetransportmanifest/politiquecaptureavantcode ;
aucunrunArefusprédit,source1eequalifiée9mémoireseulement resteinchangée.

RORecherche CONTRATexact355ffadd01 favorablebornée reçue sansblocagestatique :
vraiStdFs/Spyentréeavantbuild/report,4rougesbaseline+positif/panel15/readonce
mêmetamponcohérents. Spieseffetsréelspourreuse/delete/persist, flagsV3refusés
avantlecture mêmevide/0. AucunRustdiff/test/Cargo/ELF/proc/corpus/service.
Choixcapture/transportmanifestC7resteAVANTimplémentation/GO ; proposition
captureuniquepolitiqueactuellepubliéeavantbuilder,aucunereselectionaprèsrefus,
manifestedereplayensuite ; pastri5000silencieux. TransmisC7,pasFSqualifiée.

ROsecondaireBanc ccd9ebcf86 contrat355ff favorableprépa/4SHAconformes,
baseline1eeconfirméeparGit, pasbloqueursupplémentaire. SouscasIOenum/meta/read
distinctssirevendiqués, preuveREFUSEDadmise≠reportindexavantgate. Aucunimpl/
témoinCPU/Cargo/ELF/proc/corpusmêmemetadata/service. Les2ROcontratfavorables,
choixcapture/transportC7avantcodeetGOtoujoursrequis ;source1ee/neufmémoire/
core9HOLD/7.2ULPPR42inchangés.

Arbitrage C7 reçu17nov : CAPTUREINITIALEUNIQUEStdFs, ordreEFFECTIF/politique
versionnée/cap5000/exclusionsavantcorpus ;pastri/recaptureaprèsREFUSED.
TransportCapturePlan explicite+snapshotbuffersRAM ; sinkmanifestexclusifhashé/
fsyncavantbuilder ADMITTEDouREFUSED(pasreportindex), replaymanifestmêmespaths/
SHA sansreselection. Prépa extractionBASELINEvraiconsommateurB+spies/4vacuïtés
(IOenum/meta/readdistincts)+positif AUTORISÉE, fautesconservéesjusqu'auxrouges.
SourceRED+recette300/32Gquota4/T3552/ELFJSONavanttests àpublier/RO/GOdistinct
AVANTCargo. 0corpuscontact/natif/implgatecorrectifavant4rouges ;A/BHOLD,
SHA2devdepversionexacte/lockaudit/offlinepascryptoDIY. Nouveau lot propre
cx-lucivy-fs-admission àpréparerdepuis1ee5, mémoires9historiquesconservées.

RO Recherche reçue17nov sur source exacte8b9435363dbd8ec0e3973f2368b641189f0726eb
+docs c987957426 : favorable BORNÉE baseline/recette proposée, aucun blocage
statique établi. VraiB/seamStdFsSpy/panel15 inchangé,6négatifs4familles+positif
statiquement discriminants ; compilation plausible mais NONvérifiée. Delta
lock offline à archiver strictement. Aucun Cargo/rustfmt/test/ELF/proc/corpus/
service par Recherche. PasGO300 ni qualificationFS/native ; pilote concret,
RO et GO C7 distinct restent requis. Ticket reste ouvert.

C7 poursuit autorisation PRÉPARATION du paquet/piloteRED17nov sous ce ticket,
aucun Cargo/testnatif/corpus. Proposition300 conservée (release/offline mêmeT),
END physique unique incluant préflight/CommonMIN/final ; ledger2ELF copié sur
portable avant toutELF via exporteur autonome armé AVANT lancement, ACK
source/nonce/SHA sans dépendance humaine. Bail lancement15s, échec si poste
nonacquis dans ce délai ; aucune attente sans borne ni GO implicite. Six
rouges par noms/causes/traces, infraSTOP ; contrôlepositif. ROcouple puis GO
C7 distinct avant exécution. Code source8b94353 inchangé.

Paquet exact publié6a16c7220f : pilotea5c746/admit166d3f/export047111/
préflight79adf115. Action de RO couple confiée à Banc, avec Recherche en
complément sur budget/attribution des rouges/barrière autonome. Critères :
cohérence source8b,9copies,300physique/CommonMIN/final, ledger2ELF copié et
rehashé avant tests sans ACKhumain, six causes exactes/positif, infraSTOP.
Porteur exécution future Lucivy après GO C7 distinct ; aucun run autorisé ici.

RO Banc couple exact6a16c7220f favorable PRÉPA SEULE reçue : c3cba8417c,
rapport docs/banc/17-11-2026/03-ro-couple-fs-red.md. Trois AST/neuf copies PC
conformes, préflight79adf115 et export047111 identiques portable. ARM15/
physical300/ENDmin−12, ledger fsync/rehash/ACK avant toutELF, six causes et
positif, finCommon/scan/rehash/retourSSH cohérents à lecture. Lock ignoré et
delta explicite. Aucun test/pilote/proc/corpus/DB/service par Banc. Aucun
résultat/qualificationFS/read-once/global ; GO C7 exact distinct requis.

RO Recherche statique du couple6a16c7220f/source8b reçue favorable bornée :
admit166d/pilota5c/export047 et recette/préflight79ad relus. Arm avant launch,
physicalEND arm+300 incluant poste, Commonreste−12/finalpostCommon, ledger
copié/rehashé/ACK avant inventory, six causes/traces/compteurs stricts puis
positif, infraSTOP cohérents à lecture. DeltaLock strict et backups conservés.
Aucun Cargo/test/proc/corpus/contact/SSH par Recherche ; copies PC non rehashées
indépendamment par elle (Banc les a vérifiées). Aucun GO/runtime ni qualification
natif0=0/FSfix/readonce/sink/fullcore. Les deux RO du couple sont reçues.

HOLD C7 avant GO (452dd0ccfc) : export047111 n’a pas de watchdog physique
LOCAL. local_deadline démarre après ARM/launch ; fsync/hash/écritureACK/final/
retour peuvent bloquer hors borne. Les deux RO restent favorables à leur
périmètre lu, aucune garantie deux rôles300 acquise. Ancien Q/nonce/source8b/T
figés, NONconsommés. Préparation autorisée sur ce même ticket : contre-exemple
pertinent sur vraie entrée CLI à publier AVANT correctif, puis enveloppe
physique locale et durées restantes conservatrices vers PC, sans comparer
les horloges des hôtes ni étendre300. Aucun Cargo/index/corpus/natif ; pas de
nouvelle primitive de relais générique. Porteur Lucivy, RO ciblée et nouveau
GO après nouveau pin. La recette04 reste historique et NON lançable.

C7 n’admet pas le témoin315 pour l’instant : préparer un témoin COURT sur
vraie entrée CLI/main/args/hash/JSON, SSH/Popen simulés, sentinelle avant
réseau/écriture postARM ; exiger le watchdog physique avant cette frontière.
Rouge d’ordre/raccord, pas ERROR de fixture ni preuve OSkill/300 mesurés.
Témoin long300 reste NONJOUÉ ; publier le court avantfix puis RO/GOCPU60
distinct. Aucun clock virtuel ni copie047 remaniée, aucun vraiSSH/proc/T/
corpus/natif. Porteur Lucivy, même ticket, ancien paquet intact.

Témoin court préparé : test-fs-export-watchdog-entry.py SHAbe4913846a3d7b3f635306c426ddcc3ad1f4d628a6a6939d954b45c8eff0f836,
Q local cx-lucivy-fs-export-watchdog-entry-17nov-v1, copie047 inchangée.
Vrai __main__/args/préflight/SHA/JSON, ARM simulé, Path.open postARM arrêté
AVANT ouverture/écriture ; Popen interdit et execv timeout simulé, aucun
process/réseau/proc/T/corpus. Trace brute avant assertion d’ordre watchdog.
Action RO courte confiée à Recherche ; critères setup réel atteint puis FAIL
ordre (pas ERROR), pas prétention OSkill/300mesurés. C7 donne GOCPU60 distinct
après revue ; aucun test ni correctif encore joué. Source du témoin prépub.

RO Recherche ec4f97a83c/témoinbe491384 reçue favorable bornée : vraie entrée
runpy/argparse/SHA047/JSON, ARM seul simulé, Popen interdit, sentinelle avant
ouverture du log et observation avant assertion d’ordre. Sur047, ARM puis
frontière donnent le FAIL attendu après setup, pas un faux ERROR. Le double
execv vérifie primitive/TERM/kill5/durée<=300/réentrée ; aucune qualification
timerOS/kill/cleanup/ENDdeuxpostes. Aucun test/import/runpy/SSH/proc/T/corpus
par Recherche, copie Q local non rehashée indépendamment par elle. HOLD047
conservé ; GO C7 CPU60 distinct requis avant exécution. Aucun correctif.

GO C7-LUCIVY-WATCHDOG-RED-EC4-17nov60 consommé UNE fois : rc1,1FAIL/0ERROR,
0,110091704s physique, setup vrai args/préflight/SHA/JSON passé. Rouge attendu
ordre ARMmock puis frontièreécriture, watchdog absent ; aucune ouverture log/
réseau/process/PIDscan/Cargo/T/corpus dans le témoin. Bruts/rc/temps/deuxsources
archivés deux postes AVANTfix, PC cx-lucivy-watchdog-entry-red-17nov-v1 fichiers
444/cinqSHA conformes (SSH de transfert de preuves seulement APRÈStémoin).
Rapport17/08 et preuves/watchdog-entry-red. Aucun300mesuré/OSkill/cleanupqualifié,
315NONJOUÉ ; ancienQ/source8b/T intact. Préparationcorrectif peut suivre ;
aucun GO de vert/exécutionnative implicite.

Après rouge00c7 archivé deuxpostes : préparation v2 distincte, ancien047/Q/T
inchangés. Fixe proposée export-fs-red-ledger-v2.py SHA51a9b1e2cb6f3048580588ebc9dfa6d837dadfc2a011d2b78d99f4e380b268f8,
probes test-fs-export-boundaries-v2.py SHAc77827e75d39f1e9191b99fccca07089e20e6d1338df3e808250239d83ef5900.
Wrapper timeout depuis shell recommandé avant Python, worker parent exact/
CLOCK_BOOTTIME/ticks, END285 sansreset, duréePC=r−8 et ARMroundtrip<=8 sinon
STOPsanspasse, RPC bornés au reste, enfin cleanupenfantlocal. Parentméta et
process sont simulés dans probes ; aucun OS/SSH/proc réel par CPU proposé.
Trois probes : ordre inchangé, parent simulé vieux40s→durée<=237, fsyncerreur
postlaunch→terminate/wait enfantmock. Q privé v2/dev57/ino96299555, seules
empreintes/syntaxeAST vérifiées. Nouveau paquet PC non créé. Rapport17/09.
Action RO ciblée confiée à Recherche, Banc en complément si utile ; critères
raccord physique/sameEND/durée conservatrice/cleanup/probes discriminants et
pas claimOSruntime. Proposer GOCPU60 pour SEULS2nouveauxprobes sur047 (2FAIL
attendus/0ERROR, sansrejouerEC4), puis GOdistinct fixe3PASS. Aucun de ces runs
n’est joué ; aucune qualificationcouple/runtime/native implicite.

RO Recherche ciblée15b2e03f94/v251a9b1/probesc77827 favorable bornée reçue :
vraie entrée non réécrite, parentexe/argv/PIDticksBOOTTIME→END285 sansreset,
wire=reste−8/ARMRTT<=8 avantlaunch, RPCMINreste/finally enfantSSH cohérents.
Recette shelltimeout AVANT Python indispensable ; autoexec seul ne couvre
passtartup. Deux nouveaux probes047 discriminants à lecture : durée300>237,
fsyncerror sans terminate/wait→2FAIL attendus/setupgardé. V2all3PASS attendus
NONOBSERVÉS. Aucun import/runpy/test/proc/SSH/T/corpus par Recherche, Qpins
non rehashés par elle. Aucun OSkill/cleanupdistant/300 qualifié. PaquetPCv2
non matérialisé. GO distinct C7 avant chaque appel ; HOLD047 conservé.

GO C7-LUCIVY-NEW-RED-15B2-17nov60 consommé une fois : REFUS2ERRORharnais,
0FAILsémantique, rc1,0,088187572s. Signature Path.open(mode_arg,**kwargs)
du double reçoit mode nommé de pathlib.read_text puis le redonne positionnel :
TypeError multiple values'mode' avantsetup. Aucune observation créée. STOP0retry,
fixe51a9NONexécutée. Bruts/temps/rc/troissources conservés2postesAVANTcorrection,
PC watchdog-new-probes-error-17nov-v1/5SHArelus444 ; transfertSSHpreuvesseulement.
Rapport17/10. Défaut harnais à corriger sousce ticket (Lucivy), nouveaupin/RO/
GO requis pour retenter les deux nouveaux probes ; GO15B2 épuisé. Aucun rouge
nouveau ni vert acquis ; EC4ancien valide/oldQ/T/nativeHOLD inchangés.

Après archivage erreur313cba1e95 et avant toute relance, signature harnais
corrigée dans test-fs-export-boundaries-v3.py SHAa99f28c3de2506e05dd03452006e170392ea359a3221863593f2e17b644db8aa.
Double Path.open(mode='r') consomme le mot-clé une fois ; scénario extérieur
renommé scenario pour éviter masquage. Assertions order/duration/io-error
inchangées, exporteurs047/51a9 strictement identiques. Nouveau Q privé v3
UID1000/dev57/ino96308148 ; seulementSHA/AST, aucun import/runpy/test.
Action RO delta confiée à Recherche, puis nouveauGOCPU60 C7 pour seuls2cas
nouveaux047, attendus2FAIL/0ERROR. AncienGO15B2clos, aucun retry implicite,
aucun v2vert/315/native/target/corpus. Rapport17/11 et marqueurv3 préparés.

RO Recherche delta180a5628cf/a99f28 favorable bornée reçue : Path.open(mode='r')
consomme correctement l’argument nommé de read_bytes/read_text, scenario
extérieur distinct, forwarding conforme. Discriminants ordre/durée/cleanup
inchangés. L’avis15b2 n’avait pas détecté cette faute ; les2ERROR313c restent
setup, jamais rouges fonctionnels. Deux nouveaux probes047 :2FAIL/0ERROR
attendus NONOBSERVÉS. Aucun import/test/runpy/proc/SSH/T/corpus par Recherche,
aucune qualificationOS. GO15B2 épuisé, nouveauGOCPU60 C7 distinct requis ;
niEC4rejoué ni3PASSv2 autorisés. Harnaisv3 toujours nonexécuté.

GO C7-LUCIVY-NEW-RED-180A-17nov60 consommé UNE fois :2FAILattendus/0ERROR,
rc1,0,102601622s physique (suite0,006), setup réels args/préflight/SHA/JSON
atteints. Durée047=300>237 ; fsyncerror aprèslaunchmock laisse enfantmockactif
sans terminate/wait. Observations/bruts/rc/temps/troissources2hosts archivés
avanttoutesuite,7SHArelusPC444 watchdog-new-probes-red-17nov-v1. Rapport17/12.
0EC4rerun/0fixe51exécutée/0réseau-proc-signaux-processfixture/Cargo/T/corpus.
SSHseulementtransfertpreuvesAPRÈS.2ERROR313c conservés sansrequalification,
rougeEC4valide. GO180A épuisé ; trois verts51a9/a99 attendent GOdistinct,
aucuneOSkill/cleanupdistant/300/FSnativequalifiée. AncienQ/source8b/T intacts.

GO C7-LUCIVY-BOUNDARIES-GREEN-180A-17nov60 consommé UNE fois :3PASS/0FAIL/
0ERROR/0skip,rc0,0,102948729sphysique (suite0,013), fixe51a9/harnaisa99f28.
Setupvrai3cas, wrapperavantARM ; durée236,991636704<=237 parentmock40s ;
fsyncerror→terminate/waitmock et enfantmockinactif. Bruts/3obs/rc/temps/sources
archivés2postes,8SHArelusPC444 watchdog-boundaries-green-17nov-v1, rapport17/13.
0SSH/proc/signal/processréelparfixtures,SSHtransfertpreuvesAPRÈSseulement.
AucunOSkill/cleanupdistant/300/FSnativequalifié. RougesEC4/180A+2ERROR313c
conservés. PréparationPCQv2/recettecomplète/pins/RO maintenant autorisée par
C7 siVERT, mais aucun futurGO implicite ; ancienQ047/source8b/T inchangés.
