# La garde des comptes en blobs reste à302documents pour301lignes

- **État** : ouvert, canonique arrêtée au premier rouge, attribution NON ÉTABLIE.
- **Pour** : Recherche, revue Hotfixs/ED coordonnée C7 avant action.
- **Famille** : produit
- **Parent** : [maintenance générale](2026-11-16-la-maintenance-du-plein-texte-ne-peut-pas-ouvrir-un-index-existant.md), ouvert ; [lecture sans writer](2026-11-16-lire-un-index-plein-texte-existant-sans-writer.md), cible verte mais canonique non qualifiée.

## Preuve exacte avant tout correctif

Source union9576079d62c3f46bdf241cd19c9ab30e9218333e, pilote9a82d4c8d331096bc681aefdfc53c918e0f995d2, native59ac9c86bc2b3529b590a7462ab7051e3ad26b8a complète. Passe canonique autorisée C7-FTS-CANON-9a82-16nov.

PremierFAILED actif : `e2e_tx_par_paquet_arret::la_garde_des_comptes_retire_un_document_sans_ligne_en_blobs`, `tests/e2e_tx_par_paquet_arret.rs:1018` : `la garde a rebâti l'index : un document par ligne`, left302/right301. Firstred1181259.031107475 ; watcher-premiermatch1181259.028946841 sur inode165196878/dev55,19231octets lus, SHA7c4752e610869b1cdd51b7a897a3e2aed6fcaf126c22b45c78354c44c3194c48. Aucun rejeu/après-rouge runtime, aucune exclusion ni résultat final inventé pour cette suite interrompue.

Logs PC : `~/.cache/rag3weaver-build/cx/recherche-fts-existing-canon-v1/{first-red.json,watcher-first-match.json,official/e2e_tx_par_paquet_arret.log,official/resume.log}`. Scope validation-ajgm4kao sous cx-scope-recherche-fts-existing-canon-v1 : ROUGE1/«descendants laissés après la commande», puis restes[]/nettoye=true/autorité terminée/bus absent. Scan indépendant983identités PID/start : alive[]/unknown[]. Archives PC/local fidèles en préparation `cx-archive-recherche-fts-existing-canon-v1`, y compris préfixe exact de première lecture hashvérifié et ELF copiés APRÈSrouge (pas d’attestation avanttests).

## Diagnostic RO seulement — faits et questions

Le helper de test appelle `la_garde_retire_un_document_sans_ligne(false)` : mode **Blobs**, pas le cas Files. Il crée sa base privée, synchronise, vérifie documents==lignes avant fantôme, ajoute `_node_id=9000000` par index_document puis commit et vérifie documents=lignes+1. Une recherche du texte fantomatique est attendue sans ligne rendue. Après une seconde synchronisation, l’assert de comptes observée échoue ; les assertions suivantes et le nettoyage explicite de fin de helper ne sont pas atteints.

`documents_et_lignes` lit le handle courant par `fts_handle(index_du_code)` et compte les lignes du nom physique `index_du_code`. Ces comptes seuls ne donnent PAS bijection IDsFTS↔offsets graphe, ni preuve que le nouvel existing-only a été appelé, ni régression attribuée. La nouvelle primitive est un fallback Files pour live_ids/dry-run sans handle ; ce constat statique n’innocente aucun entrant à l’exécution.

À établir avant tout correctif : table/cellule physique réelle et mapping public de ce cas, vivantsIDs vs graphrows/IDs, générations et commits avant/après, handle courant/cache/snapshot vs éventuel fallback, chemin de garde/rebâti réellement emprunté en blobs. Comparer Files séparément sans transformer un autre cas en preuve de celui-ci. Pas de bug nouveau affirmé sur302/301seul.

C7 demande revue source/preuves Hotfixs/ED ; éventuelle nativeDBRO sur **COPIE privée seulement**, recette600 à publier puis autoriser distinctement. Aucun writer, warmup, purge, rebuild, réexécution de juge ou baseLucie ; pas de correction automatique. F8/preuve/lib/NAV et suiteTXvecteurs NON JOUÉS dans cette canonique arrêtée. Créneau services/PG rendu à Embar après quiescence+health, sources/targets/preuves conservés.

Archives finalisées : rapport14,65fichiersPC/local réhash0écart manifeste1e5abe3d4b3830be24a9b09bccc9a73533db21127951d1017349e033bf2bb12d, deuxELF APRÈSrouge et octetspremièrematch SHA vérifiés. Créneau rendu et repris parG ; plus aucun appel partagé R. Fixture privée retrouvée e2e-gardes/PID1254532 owned terminé, conservationfilesystem seule pour éviter sa perte ; aucune ouverture moteur. AnalyseRO native toujours soumise recette600/GOdistinct, aucune correction/retry.

## RO Hotfixs source957, avant toute action

Piste statique précise : code_sync.rs1368–1369 bifurque vers synchroniser_par_generations quand generations et source_deja_indexee ; le corps1443–1600 n’appelle ni check_fts_counts ni rebuild_pending_fts, présents seulement dans le chemin classique1663/1682. register_code_schema active generations sur base neuve compatible (code.rs864–889) ; source_deja_indexee2027 lit les lignesScope/source, pas les vivantsFTS. Dans la branche générations, une empreinte inchangée continue sans réécriture1550 : hypothèse forte pour le fantôme conservé, **branche effectivement exécutée NON TRACÉE**. Le log brut confirme302/301 et ne contient aucun message garde/rebâti, pas une preuve exhaustive d’absence d’appel.

Ne pas confondre ces générations lignes/couches avec la génération FTS Files : la comparaison fts_generation/base/disque de Catalog1755+ est limitée à Files. Le cas observé construit BlobShardStorage(cellule/index,cache_base). with_fts_read1491 rend le handle courant s’il existe ; existing-only ne sert qu’en absence. Aucune preuve actuelle d’erreur native/générationFiles/cache stale. Si un rebâti était plus tard établi, examiner seulement alors drop_fts_index1377/Arc et rebatir…8657 ; ces causes ne sont pas attribuées ici.

Discriminant proposé RO sur copie : forme code/couche/rangs/source, table/cellule physiques réelles puis IDsFTS vivants↔offsetsgraphe, plutôt que seuls comptes. Aucun fichier produit changé, aucun runtime après rouge.

Fixture privée COPIÉE filesystem seulement : `~/.cache/rag3weaver-build/cx-archive-recherche-fts-existing-canon-fixture-v1/copy`,3fichiers/40255669octets, hashes sourceavant=sourceaprès=copie ; manifeste69b1734910cace8d0b05ff4d2a5b128d2594c9e2834eb136167aa462dd433192. PID1254532 vérifié dans le scope terminé puis absent. Ne pas ouvrir cette archive directement : éventuelle DBRO utilise une NOUVELLE copie et recette600/GO C7, aucune purge/reprise/warmup.

## Préparation contre-exemple/RO600, avant diff d’adaptateur

Rapport15 : jugeDOUBLE futur même sync_source sur formes supportées classique/générations, IDs/jetons/branche tracés dans fixtures séparées, aucune désactivation à chaud ni replayinchangé ; GO nativeDOUBLE distinct obligatoire. RecetteRO600 proposée sur NOUVELLE copie froide, outilC++readonlyPC48ae/SDK59 et ELF4.3.9 IDsd0dd qualifiés existants. Pas de cAPI ni toString(BLOB). Nouvel extracteur binaire typedSTRING/BLOB+matérialiseur à écrire/tester/relire AVANT GO ; réemploi open_blob_snapshot, filtrage tombstones, chemins/config/shards stricts. Selftest sansDatabase, aucun CREATE/writer dans RO. Source/copie/WAL hashstrict ; exception.readers exactregularUID0octet seulement, pas de nouveaux répertoires tolérés implicitement. Aucune exécution RO/native/Cargo/juge lancée.

Préparation concrète RO600 : extracteurC++typedreadOnly+matérialiseurframing+observations+lanceur publiés après testsprépub,8fonctionsPythonpuresVERT0.002s ; compilation/selftestnative/décodeur/DB restent NONJOUÉS. Rapport15 détaillecommandes/caps/identitéoutils/SQLdefaultScopeContent/sourcecopiehashstrict/exceptionreaderssansnouvdir. NOUVEAUGO600 demandé seulementaprèsROexacte ; aucunecréationPQS/native/target/Cargo parcespréparatifs. Contre-exempleDOUBLE reste proposé, aucun nouveaujugeRust écrit/joué ni correctifproduit.

## Réserve supervision Hotfixs b83 — AVANT diff

L’avis favorable aux contrats dump/lecture ne lève pas cette réserve : ancien launch effectuait hashes/git/composition après retour du timeout630 interne, donc hors borne ; refuser un statut tardif n’arrêtait pas ces lectures. b83 retiré avant toute admission/native. Correction prévue : poste→timeoutEXTÉRIEUR598s+kill2s couvrant compose ET confineur ET tous hashes/scans/publication, confineur interne550s pour marge ; lanceur extérieur ne fait que propager le rc, aucune finalisation après retour. Signature du vrai parenttimeout/poste vérifiée, arrêtpremierrouge/timeout = nonqualification, quiescence affirmative seulement après scope et scanowned complets. Aucun native/Cargo/PQS pour cette correction hermétique, nouveauSHA àRO/GO distinct600.

## Observation RO autorisée, sans correction

Recette exacte c5d sous GO C7-RO-BLOBS-c5d-16nov : copie neuve strictement inchangée, extractiontypedBLOB et décodeLucivy4.3.9 réussis. Graphe0..300, FTS0..300+9000000 ; FTS−graphe=[9000000], graphe−FTS=[]. OBSERVATIONseulement : branchehistorique nontracée, maintenance/réparation nonqualifiée. Rapport [16](../16-11-2026/16-preuves-diagnostic-blobs-ro.md), manifeste774SHA identiques2postes e9272f...3662. ScopeVERT0/TERMINE/quiet, poste rendu, aucune suppression. Ticket demeure OUVERT.

## Préparation DOUBLE autorisée après observation149a, AVANT code

C7 demande deux témoins minimaux par la même API sync_source, formes déclarées avant schéma classique/générations, bases privées distinctes mais paramètres/corpus/options identiques. Chaque témoin exige IDs physiques égaux et nonvides avant fantôme, commitFTS réussi après injection9000000 préalablement absent, puis égalité restaurée après synchronisation inchangée et conservation UUID/H/positions. Témoin cfg(test) local au fil autour du dispatch réel et appels garde/rebuild, sans nouvelle API produit ni modification du dispatch. Deux fonctions NONJOUÉES, pas de correctif ici. Données publiées avant toute exécution ; futur build/native DOUBLE et ancien-master distincts attendent RO+GO exact. Pas de canonical retry ni affaiblissement de la garde302/301.

## Réserve de diagnostic ED DOUBLE — avant delta

RO7749 favorable bornée ; sync_and_trace fait expect(Ok) avant de rendre TRACE, donc une erreur précoce peut masquer les événements. Corriger uniquement ordre : recueillir Result, vider/imprimer TRACE et Result, puis expect/assert. Deux exacts séparés sans parallélisme dans futur pilote (même curseur Snapshot/registre partagé). Aucun test natif ni compilation autorisés par cette remarque.

## Réserves pilote DOUBLE490 — avant correction

Hotfixs : (1) main-profile ne vérifie avantcontinue que basename/librag3db.so, SDKexact n'est vérifié qu'après exact0 ; exiger cheminmap canonique SDK59 ET empreintepin ledger AVANTcontinue. (2) cache_quiet accumule exe/cwd/FD puis attribue : PermissionError ultérieure peut effacer attribution d'un chemin déjàobservé sousT/source. Attribuer dès chaque lecture, conserver found/attributed sur erreur ; UNKNOWNattribuable refuse avantadoption.490 retiré avantallocation/run ; PCQv1 conservé, futurQv2 distinct ; aucunT muté. TémoinsCPU hermétiques à publier avantfix, pas de processus/proc/signaux réels.

## DOUBLE premier refus du harnais — avant tout nouveau diff

Pilote4d526/APP3ee uneinvocationGO C7 : buildrc0/ledgerELF268229be/invent2ignored0 ; CLASSICréelrc0/1passed0failed0ignored/tracecounts1-rebuild1/FTS[0,9000000]→[0] et UUID-H-positions conservés. Harnais ligne167 refuse nom+ok interrompu par événementsGDBthreads : premierrouge1189092.891744, GEN NONJOUÉ. Aucunretry ; nouveauGOdistinct nécessaire. Rapport19/manifeste62files2postes1a2e39e4...19324 ; scopeROUGE1TERMINE/restes[]/quiet127owned/postedéjàrendu. Défautcontrôleur connu NONCORRIGÉ àcecommit, ne qualifie pas DOUBLE ni branchegénérations/historique. État OUVERT.

## Continuation GEN seule proposée par C7 — avant nouveau code

Après69a280, classique acquis BRUT sans requalification scopeROUGE. Ne rejouer NIclassic NIcompilation NIgdb pour le défautregex. ClassificateurCPU strict sur vraislogs+command--exact/invent2dontnomunique/summary1-0-0/rc/tracesréelles, sans nettoyage des lignesFAILED/panic. NouveaupiloteGENdirectseul mêmesELF268229...147c/SDK59/ledger, targetowner3ee inchangé/SOURCES3ee+ancien4d figées ; reçuNEUF aprèsposte MINENDoriginal1190161.616250876, borneouteroriginale conservée, aucunreset. Nouvellepublication/RO/GOdistinct ; refus ou échéance→STOPconserver sansretry.

## GEN seule : vrai rouge du chemin générations — AVANT tout correctif

GO C7-GEN-BA5-16nov unefois, APP3ee/mêmeELF268/sansCargoGDBclassicrerun. Initialclassic/counts0/rebuild0 ; aprèscommitfantôme[0,9000000], secondpublicsyncinchangé TRACE=[dispatch:generations]seule/ResultOk/files_unchanged1 ; aucun pointcounts/rebuild parcouru, FTSreste[0,9000000]/graphe[0]/jetoninchangé. VraiPanicassert ligne110,firstred1189858.149915276 sousENDoriginal1190161.616 (846.534sdepuisINITIAL). RetourfinaljugeNONACQUIS/STOPsansretry. Rapport21/archive263SHA2postesff62f87d...927b2 ; scopeROUGE1TERMINE/4ownedquiet/posterendu. CompareCLASSICbrut19répareavecguard1/rebuild1. Attributiondifférentielle danscesfixtures établie ; histoire149a/baselineancienmaster/réparation/canonical NONqualifiés. Aucun correctif source appliqué, étatOUVERT.

## Préparation correctif commun autorisée C7 — AVANT diff

Nouvelle source depuis preuves8f813, anciennesAPP3ee/ELF268/fixtures/portées figées intouchées. Extraire un helperprivé de préparationFTS utilisé par classique et générations AVANTlectureliste/filtreinchangé/retoursanticipés : gardecomptesexistante, gardeFiles stricte, reconstructionpendingexistante. Aucun redispatch/fallbackalias/downgrade ni modification contenu/visibilité/H/identités. ConserverAPIverify_fts_files historique pour ses autresappelants ; varianteinterneResult poursync évite nouveau succès aprèserreurFiles. PropagationErr/pending avantreadsource sera jugée. Témoins spécifiques àprépublier (pendingmêmecompte, erreurpending bloquelecture, listeErraprèsgarde) AVANTimpl, tousNONJOUÉS. Les2jugesDOUBLEexistants restent le rougeGEN et le classiqueacquisbrut ; green/nouveaupiloteGOdistinct obligatoire. AncienEND est historique, jamaisprolongé/réactivé.

## Limite RO sourcebc952 et préparation du pilote vert

Hotfix favorable bornéBlobs : varianteFilesstricte propageErrrebuild_fts_step seulement ; ouvertureFiles conserveOption+note_index_failure/log, donc passtrictglobalFiles. Défautouverture àqualifier séparément si nécessaire, NONrésolu par ce lot. Pas de qualificationreadonly/in_transaction/planonly/multicellules. Pilote5exacts décritrapport23, sourceAPPbcfigée, budgetNOUVEAU1200proposé/freshGOdistinct ; aucuneautoritéancienne réactivée, aucunrun ou Tmuté.

## Vert ciblé cinq juges, ticket encore OUVERT

APPbc952/PILOTEaf11 unefoisGO C7-FTS-GEN-GREEN-AF11-16nov : buildrc0/ELFb23414d3/ledger512c2a avantmarkers/invent5/0ignore,5exactsDISTINCTS chacun1passed0failed0ignored etrc0. GENsurinchangé traceguard1/rebuild1 retire9000000commeclassic ; pendingcompteségaux/sourceErr/maintenanceErr validés. Rapport24/archive94SHA2postes57fbd028...eb3c ; scopeVERT0TERMINE/94ownedquiet/posterendu/Townerbcconservé. AnciennesportéesROUGES62+263/APP3ee/ELF268intactes. Pascanonical/master/globalFilesstrict/COUNTstrict/readonlyplanonly/multicellules qualifiés. PRÉPAUNIONNEUVEautoriséeC7seulement, BGE/PGnonréservés. ÉtatOUVERT.

## Préparation globale après ciblés ROOT, sans GO

C7 reprend préparation seule après0479 (8distinct/11rawverts), ROHotfix archive133 favorable. Fetchmastercourant b0 inchangé ; APP84 exact=union5f59+modulecfgtestfix, horsdocs autresproduits préservés. Nouvellebranche piloteglobalr2 ; ancien349b intact. Canoniquecommitted2150/Common6439 ; jamais recopier la copie locale divergente ni modifier cachecommun gelé.

Coordination : ne pas recopier race READYB5. Réutiliser lifecycle_protocol.py98ab exactSHAc67f072ba0134f97be6150547d80b8a23174937f2c964b0bdc30bec5e0309ec0 pour ready_bounds/phase stable/firstred/PIDstartCGUIDscript et rechecks. Son joblegacy lifecycle-provider désigne admissionprovider ; purpose explicite fts-global-models-and-pg, pas qualificationcycle ni noembedding simulé. Callbackc530 reste endpoint7878/noembeddinguniquement, non transplanté au globalBGE+PG. Contratowner global devra couvrir2providers+PG/nonce/READYsha/observed/lease et rechecks communs, ROgroupéeavantnouveauGO. Aucun appelhealth/PG/service en préparation.

## Réserve Embar sur globale333de822, avant correction

ROcontrat partagé favorable mais implhealth() valide la phase seulement avant la boucle2providers. Premierrouge/finphase entre deuxGET peut laisser partir le second sansrecheck ; nepasadmettre333de822/Qb8a327. Corriger read_phase_state PUIS clock/validate_ready_phase avant CHAQUEGET et PG, checksaprès ; transportwhole<=5s/preuvesRAWstdoutstderr/rc/timeout. Témoinsfictifs au vrai raccord secondGET/PGtardif/firstred/phasefinie/nonce/source/linkretardé avant nouveauGO, pas de réseau réel. PGcontrat doit lierowner/C7/allocation/DB/user/freshness/réservationactive ; disponibilitéN/S n'est pas identitéSQL ni permission. AttendreROED/Hotfix pour regrouper delta, sources/ancienpaquet conservés et globalHOLD. Nouveauownerpolicy2providers+PG resteàqualifier,aucuneAPIshared àchanger.

## Réserve Hotfix 7b9d9a8ecb, avant correction finale

Le pilote333 exige check_features.log dernièreligne EXACTCHECK_FEATURES VERT alors que le vrai scripts/check_features.sh APP84 émet CHECK_FEATURES VERT8/8 (séparé par espace). Fauxrougefinalcertain ; publier témoinPythonpur contre la vraie forme du suffixe avantfix. Ni scriptcanonique2150 ni sourceAPP àmodifier. Raccordowner toujoursabsent : producer._ready/noembedding/worker7878 ne doivent pas être appelés au global. ROEDenattente ; groupercorrectifs quand ses réserves sont connues, maintenir333/Qb8a327HOLD.

## Complément RO ED groupée, avant correction transport

ED confirme lesdeuxréserves et ajoute : timeouturllib5 ne borne paswholeGET+body ; response.read peut dépasserREADY/lease. Consumer doit réutiliser transportowner worker harddeadline/killreap+RAWstdoutstderr/rc/timeout, phasecheckavant CHAQUEcontact etaprès, pas simpletimeout de socket ni outerglobal seul. PGprobeN/S déjà annoncéeprotocolONLY n'établitniidentitéSQLniexclusivité. Embar consulté pour APItransport réutilisable avantcanon(admission) etaprèscanon(leaseactive/coreautorité,pasréadmissionREADY). AucuneAPIcommonàchanger,ownerglobalconcretattendu ; regroupementfix quand ce raccord est publié. Aucunautreblocageétabli parED ; aucuneexécutionreviewer.

## Frontière owner/consumer figée avant raccord globalr3

Branche/WTneufs cx-recherche-fts-generation-global-pilot-r3, ancien333/Qv2préservés. APIEmbarhealth_observation(ready,pins,provider,evidence,admission,lease_end) workerhard5/RAWkillreap, modeadmissionTRUEavantcanon etauthoritycore+leaseFALSEaprèscanon. Grant.owner_pins completSANSecrets + owner_pins_sha256 SHAJSONsort_keys/separatorscompacts/allow_nanFalse. Consumer vérifiecontexteAPP84/pilotfutur/scope/nonce/READYsha/allocationC7/sourcesprotocol98ab/c530/owner etproviderscohérents, evidencefinally. Aucun fauxflagnoembedding ; PGconsumerpingretiré, artefactGSQLRO+topologie+exclusivitéC7 liénonce/READYEND, validéowneravantgrant etchaquecontact ; outilPGconcretresteHOLDavantGO. Deuxnouveauxfichiersowner stricts attendus,aucuneditparRecherche ; correctionsgroupées F88/8 + phasechaqueGET + transportwholeaprès2contre-exemplesrougespubliés806398. CPUhermétiques<=60failfastaprèspublicationcode/témoins autorisésC7 ; aucunHTTP/proc/PG/Cargo/native.
