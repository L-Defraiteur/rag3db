# Lucivy — 17 novembre 2026

État : préparation du rouge FS, aucun Cargo ni corpus contacté.

Suite du [contrat du 16](../16-11-2026/16-qualification-native-et-contrat-fs.md),
arbitrage C7 : capture initiale unique StdFs, ordre effectif, cap5000 et
exclusions versionnés, CapturePlan explicite et snapshot RAM. Pas de recapture
après REFUSED. Rejeu ultérieur par manifeste, mêmes chemins/SHA sans resélection.

## Reprendre

- Code : `/home/lucied/git_workspaces/cx-lucivy-fs-admission`, branche
  `cx-lucivy-fs-admission`, poussé `8b9435363dbd8ec0e3973f2368b641189f0726eb`.
- Documentation : `/home/lucied/git_workspaces/cx-lucivy-plan-shutdown`, branche
  `cx-lucivy-reprise-16nov`.
- [Préparation, limites et recette proposée](02-preparation-rouge-fs.md),
  [pins source/lock](03-fs-red-source-pins.json).
- [Ticket porteur](../tickets/2026-11-16-l-admission-du-corpus-fs-ne-garantit-pas-la-vue-consommee.md)
  ouvert ; revue statique demandée à Recherche puis arbitrage C7. Aucun GO Cargo.

La source 1ee5 reste qualifiée pour neuf juges mémoire seulement ; cette nouvelle
source n'est ni compilée ni qualifiée. Natifs A/B, core complet et ses neuf HOLD,
ULP Linux7.2 et PR42 restent HOLD. Aucun main/tag/publication/fusion.

RO Recherche reçue sur source exacte8b94353 et docs c987957426 : favorable
bornée à la baseline et à la recette proposée ; aucun blocage statique établi.
Compilation non vérifiée. Suite : pilote concret avec delta lock offline
archivé, puis RO et GO C7 distinct avant toute exécution.

Paquet concret préparé, sans exécution : [recette exacte](04-paquet-rouge-fs.md).
Q PC fs-admission-red-17nov-v1, W PC fs-admission-17nov/source8b94353 ;
préflight79adf115e7096644c32cf6be31286d89e03ce54c160fe5da538c933afeac15db.
Exporteur portable autonome armé avant lancement, aucune dépendance humaine
pour ACK ; timeout physique300 incluant acquisition/retour poste, END unique
CommonMIN/final. Neuf copies vérifiées et syntaxe AST uniquement, aucun
Cargo/ELF/proc/corpus. RO couple exact puis GO distinct restent requis.

RO Banc du couple6a16c7220f favorable PRÉPA SEULE : c3cba8417c, rapport
`docs/banc/17-11-2026/03-ro-couple-fs-red.md`. Copies/AST/bornes/barrière
autonome et attribution des juges cohérentes à lecture. Aucun run/GO déduit.

RO Recherche statique du couple6a16c7220f/source8b reçue favorable bornée :
admit166d/pilota5c/export047 et recette/préflight79ad relus. Arm avant launch,
physicalEND arm+300 incluant poste, Commonreste−12/finalpostCommon, ledger
copié/rehashé/ACK avant inventory, six causes/traces/compteurs stricts puis
positif, infraSTOP cohérents à lecture. DeltaLock strict et backups conservés.
Aucun Cargo/test/proc/corpus/contact/SSH par Recherche ; copies PC non rehashées
indépendamment par elle (Banc les a vérifiées). Aucun GO/runtime ni qualification
natif0=0/FSfix/readonce/sink/fullcore. Les deux RO du couple sont reçues.

HOLD C7 452dd0ccfc : absence de watchdog physique portable dans export047111.
Ancien paquet figé/nonconsommé, commande04 HOLD. [Contre-exemple CLI proposé
avant correction](06-hold-borne-physique-portable.md),300s réels/fauxSSH/fsync
bloqué/secours302. Aucune exécution du témoin ni correctif encore effectué.

Témoin315 nonadmis pour l’instant. [Témoin court du vrai raccord CLI](07-temoin-court-raccord-watchdog.md)
préparé/prépub avantfix, source047 intacte, SHA témoinbe491384. Seuls SHA/AST
vérifiés ; attente RO Recherche puis GOCPU60 distinct. Aucun timerOS qualifié.

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
