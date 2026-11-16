# Couple READY et callback owner concrets — préparation seulement

Suite de [recette15](15-contrat-ready-borne-producteur-consommateur.md),
sans nouvelle autorisation de RUN/health/slot/CLI/DB/service. Le couple16sources
dc276/Q413ed et ses P/Sabsents restent historiques ; B5rouge/lategrant et
65preuves restent figés. Ici, nouveau P/Q/S v3 et17sources, pas écrasement.

## Composant owner intégré sans réécriture

Embar c5306b42663d2c001578dcc64d621cac826bfad5, deux fichiers SEULS
lifecycle_provider_observation.py et test_lifecycle_provider_observation.py.
Neuf commits dc276..c530 portés ; diff final0 contre ces deux blobs.
SHAobserver95729d4d2eabaaf6b92fef72f2754c43cc1675c76107bffede86c893c90f8618.
Protocol98ab/SHAc67f072ba0134f97be6150547d80b8a23174937f2c964b0bdc30bec5e0309ec0
et producteur3aa restent octets identiques. ROED+Mémoire favorables au
composantfinal c530, pas au futur couple par transfert.

publish_observed_grant est le raccord owner réel : appelle le producteur
avec health_margin30 FIXE et le callback, jamais le défaut générique5.
Entrées nécessaires et à figer au GO/READY : hashesobserver/protocol/producer,
pilot du couple, allocation et c7_referenceIDENTIQUES, embar_reference,
expected_process{pid,uid,start,exe,exe_sha256}, identityNEUFchamps exacte.
Le propriétaire remplit les faits attendus depuis sa provenance autorisée ;
aucune valeur stale n'est présentée comme une santé fraîche avant READY.

Avant le contact : Readysha/nonce/provenancephase/autoritéactive/firstred/
borne sont vérifiés par le producteur puis par le callback. Il prend
PID/start/UID/exeSHA réel, refuse tout changement. Deuxétatsvivants R/S/D/I
sont admis entre échantillons, R→S n'est pas un changement d'identité ;
T/Z/inconnu et UID/start/exe changé refusent. Pas de lookup SDK ni modèleprobe.

Le callback fixe deadlineHTTP=MIN(now+5,ReadyDue−30). Le parent lance un
worker unique et borne sa vie entière par cette deadline : timeout=>kill+
reap. Le worker vérifie à son tour pins/Ready/phase avant l'UNIQUE
GET127.0.0.1:7878/sante, sans proxy/redirect/embedding/restart ; body<=65537.
L'identité9champs est jugée typeETvaleur (granite278m768réel/dense/Flex32,
executable/cap attendus), HTTP200/usagebaux0+en_cours0 stricts, PID revérifié
après lecture et callback encore dans sa deadline. Usage0 n'est pas un
compteur totalRPC ; aucun zérocalcul observé n'est inventé.

Leasegrant=MIN(END−480,observed+230), nonce/ReadySHA/noembedding, refs et
PID/start/exeSHA liés. Evidencebrute fsyncAVANT grant ; le producteur relit
Ready/phase/clock/protocol après observation et stagefsync avant link exclusif.
Le consommateur conserve ses rechecks avant premier health et ses caps de
lease pour le writer. Admission120/lease/END restent distincts.

## Conservation avant refus

P2transport traitée d23 : trace du worker avec PID/instantdépart/deadline/fin,
stdout/stderrhex, rc, timeout, capturesPARTIELLES TimeoutExpired puis sorties
finales après kill/reap, conservée dans evidenceowner même sur exception.
Oversize : imprime/flush status+préfixe65537octets et flagsoversize/prefix_only
AVANT refus, jamais présenté comme corps complet. Pas de corpsillimité.
L'evidence prégrant dit OBSERVED_NOT_YET_GRANTED ; elle ne devient pas à
elle seule une attestation de publication ou de consommation du grant.

Checks+link ne sont pas atomiques avec finphase/firstred ; rechecksconsumer
conservés. Pas de fsyncrépertoire/durabilitécrash déduits. Aucun test propriétaire
ne change les PID/services réels : tous stubs/fichiers/procfictif/Popenfictif.

## Témoins et pins

Source owner publié avant tests : rougeisolé14fonctions/7assertfails puis
fixpublié ; P2transport rouge21/3assertfails (ancienrouge3errors de raccord
API de test conservé, pas donné comme rouge fonctionnel), puis fixd23publié.
VRAIprocess_snapshot R→S/S→R rouge037b puis fixc530publié, T/Z/UIDstartinconnu
toujours refusés. Owner23+producer11+contrat4 =38fonctions vertes0.015s.
38FICHIERS de preuves owner sont distincts de ces38TESTS ; rehashPC+portable
parMémoire sansécart, manifestev3SHA
0632b09238471934840959ecfb6eff51fa750f56f7b6683a10d4aa20df136382.

Après intégration et publication de b94d502624 : suitecouple52fonctions
(9cycle+4contrat+5READY+11producer+23owner) verte0.040s, aucuneDB/health/
CLI/native/procréel/service. Les deux fichiers owner sont byteidentiques à
c530, hash ci-dessus ; les limitations des ROcomposants sont conservées.

## Nouvelle admission à préparer, pas GO

Paquet17sources à figer sous lifecycle-control-v3 ; nouveaux
lifecycle-copy-v3/lifecycle-scopes-v3, manifeste vise cette seule copieFIN.
AncienQv2 et tous vieuxP/Q/S préservés ; aucun grant précréé. Préparation
ne fait que copier les sources et hasher ; pas de fixtureDB créée.

Au futurGO : propriétaire utilise le callback CLI --ready/--destination/
--pins/--ready-sha256/--nonce/--evidence avec ces pins immuables, pas de GET
ad hoc ni d'estimation du délai depuis un message. Aucuncontact avant
READYcomplet/autoritéactive/nonrouge et marge suffisante. Tous contrôles du
cycle/digests97247/FTS5/snapshotPOST/finished/sources/quiet et budget900/
outer898+kill2/common830/ROreserve500 sont inchangés.

Source/paquet final, deuxROcouple et décision C7distincte restent requis.
Un éventuel positif ne concernera que cycle/shutdown de cettefixture ;
pas les anciensROUGES, durabilitégénérale, LIVE, originaux ou RETOUR72h.
