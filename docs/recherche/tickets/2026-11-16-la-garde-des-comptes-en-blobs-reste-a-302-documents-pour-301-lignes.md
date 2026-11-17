# La garde des comptes en blobs reste à302documents pour301lignes

- **État** : ouvert, canonique arrêtée au premier rouge, attribution NON ÉTABLIE.
- **Pour** : Recherche, revue Hotfixs/ED coordonnée C7 avant action.
- **Famille** : produit
- **Gravité** : importante — un index plein texte peut garder un document sans ligne.
- **Atteignable en service** : oui, synchronisation classique/générations.
- **Touche rag3weaver** : oui
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

## Owner global : préparation bornée avant code

Portage Embar sur autorisation C7, base333de822. Deux nouveaux fichiers
`fts-generation-global-r2/global_services_owner.py` et son test ; aucun changement
au produit, au protocole98ab, au process_snapshotc530 ou au pilote Recherche.
Ce contrat global autorise seulement les embeddings des tests canoniques admis :
aucun flag lifecycle no_embedding ne sera injecté, et aucun appel au publisher3aa
ou à validate_grant_phase, réservés à cette autre politique.

Réutiliser ready_bounds/read_phase_state/validate_ready_phase/validate_phase_authority
et process_snapshot stricts. Pins externes immuables APP/pilot/scope/allocation,
nonce/SHA du READY, neuf champs typés de chaque provider7878/7879 et identité
PID/UID/start/exe/SHA. Contrôles avant CHAQUE contact, après les observations,
après fsync et juste avant lien exclusif du grant. GET santé directs seulement,
worker propre <=5s et marge admission30 ; stdout/stderr/rc/timeout et préfixes
bornés conservés avant refus. Usage0 observé ne prouve jamais zéro RPC.

PG5433/rag3weaver_test exige une provenance DB/user/OID exacte et une réservation
exclusive active, fraîche et liée C7/G au même job/READY/END : ni pingTCP ni zéro
session n'en tiennent lieu. La recette SQL RO et ses pins sont coordonnés avec G
avant intégration ; aucun reset, migration, drop ou contact en préparation.
La lease du grant couvre END commun original ; la borne d'admission ne prolonge
ni END ni le budget. Toute identité/autorité/réservation inconnue ou premier rouge
refuse sans publication ; READY, refus et anciennes preuves restent immuables.

Publier source et fakes discriminants avant test, puis rouge→fix→vert sous borne
CPU60 hermétique, sans processus/provider/PG/proc réels. Recherche regroupera le
couple/pins/Q avant RO et GO6000 distinct éventuel. Ce paragraphe n'alloue aucune
ressource ni exécution native. Ticket reste ouvert.

## G : protocole d'attestation PG RO future — préparation avant code

Autorisation C7, même contrat owner79cb, branche G
`cx-optimiseur-pg-ro-attestation-20261116-r1`. Aucun PG/Docker/health maintenant.
Inventaire source2150/APP84 : bootstrap5433/rag3weaver_test distinct des bases
par processus, aucun psql hôte présent sur le PC relevé, DockerCLI tué ne prouve
pas reap du psql conteneur. OID/imageID/binding runtime non relevés.

Publier protocole et fakes discriminants avant implémentation : format JSON
exact nonce/READY/job/END/allocation, SQL constante RO et son SHA, topologie,
identités et propriétaire, réservation/bootstrap/enfants distincts, sources et
raw partiels immuables. Valider avant tout grant la fraîcheur<=120 et le budget
whole5, rc/quiet/reap/backend postcheck. Transport préféré : client hôte privé
épinglé, qualification/outillage future G/C7 nécessaire. Pas de dockerexecprobe
présenté comme borné ; pas d'installation, création/DROP ou appel réseau ici.
Le producteur de pièces ne lance aucune probe : il refuse si qualification
outil/identités externes manquent. Aucun VERT transport réel ne sera déduit des
fakes CPU. Nouvelle RO nécessaire avant toute future exécution autorisée.


## Protocole G/PG offline publié, pas de transport qualifié

Publication avant code c438 (contrat),3ea6 (SQL constantes/fakes), baseline131189
explicitement non qualifiée ; rouge11tests/43discriminants conservé. Vérificateur
strict0a842, puis deux filets supplémentairesd856 : métadonnées malformées ou
brut manquant doivent conserver les autres bruts avant refus (deux rouges vus).
Correction conservation puis21/21fakes CPU verts en0,016s,0échec/0saut, borne60.
Le producteur offline archive les captures, refuse, fsync et publie exclusivement ;
il n'exécute PAS la probe. Les anciens résultats ne sont pas des tests PG :
les19 premiers fakes puis21 finaux sont des validations de contrat uniquement.

API et quinze champs d'allocation/cinq champs ready fixés dans
`docs/optimiseur/16-11-2026/pg-ro-attestation/01-protocol.md`. Relecture
postcanonique avec admitted_at fiable initial : pas de nouveau TTL admission.
Loader SHA/UID/type/taille/absence de liens et bruts voisins ; fake postcheck
borne backend attribué PID+nonce dans le même5, sans programme PG réel.

Source/pins runtime PG restent inconnus. Aucun client natif, Docker, connexion
SQL/health/TCP, processus réel de provider, service, slot ou ressource réservé.
Le plan retourne HOLD jusqu'à une RO outil/observateur distincte G/C7 : il ne
peut pas produire une attestation réelle depuis les seuls fakes. END/coldG/RET
anciens restent gelés. Parent FTS toujours OUVERT et globale non qualifiée.

Check_tickets sur base79cb relève quinze champs manquants préexistants dans
cinq tickets Recherche. Les trois du présent ticket sont complétés ici ; les
quatre autres tickets restent hors périmètre, aucun VERT global inventé.

## Réserve Recherche b554fb255b — AVANT nouveau diff PG

La publication G f142 vérifie la phase/horloge avant link mais pas après son
dernier fsync/écritureSHA. Une opération tardive peut rendre path/SHA après le
whole5. Reprise G du même signalement, sans doublon : vrais faux CPU qui avancent
l'horloge ou posent firstred AU DERNIER fsync, avant fix. Exiger refus conservant
les bruts et artifact_published honnête si le JSON existe déjà ; loader doit
rejeter tout JSON publié puis accompagné de refusal.json. Aucun transport lancé.

## Réserve publication traitée, outil PG toujours absent/non qualifié

Témoins0960f1f4ed puisb72ea924f0 publiés AVANT fix : trois rouges
(dernier fsync retardé, firstred après link, JSON publié puis refusé), puis un
quatrième rouge (refusal apparaît pendant lecture raw). Journaux conservés
final-publication-red.log et loader-late-refusal-red.log. Correctif G : checks
après TOUT dernier fsync puis phase/horloge avant retour ; refusal marker avant
et après loader. JSON déjà publié reste conservé avec artifact_published=true,
aucun retour admissible et aucune réadmission par présence de fichier.
Vingt-cinq faux CPU verts,0échec/0saut,0,010s souscap60 ; aucune probe réelle.

Alternative outil demandée C7 examinée FS seulement sur PC : aucun binding
psycopg/psycopg2/pg8000 viafind_spec, aucun libpq.so aux chemins usuels examinés,
ldconfig -p code0 sans entrée libpq, aucun header relevé. Rien chargé ni importé,
aucune connexion. Pas de client privé ni ABI qualifiés à déduire de ce relevé :
TOOL_UNQUALIFIED/HOLD inchangé, aucun wire/crypto nouveau, aucune installation.
FutureAPP6365396b6323b03dc3441cb1f650c8804e53bdeb fournie par Recherche : pins
app/pilot restent externes, source combinée NON JOUÉE, pas de grant réanimé.

## Alternative archives Rust — trace RO avant toute recette/client

C7 autorise l'audit d'archives immuables, pasCargo/rustc/native maintenant.
G927release-archive4806 n'a aucun postgres/tokio_postgres rlib/rmeta ; R archives
133/94/62/65 sans rlibs aussi. Trois packages sources (tokio-postgres0.7.16,
tokio1.53.1,serde_json1.0.151) recoupés aux `.crate` locaux/checksums lock090,
APIvraielue ; ceci ne fournit pas la fermeture binaire. CompilerPC1.98.1 hash859
observé en fichier, pas fausse provenance prébuild.
ED propose copieOUT_T depuis b472QUIESCENTE autoriséeC7 ; paquet nécessaire
confirmé, aucune lecture G sur son T ni alias vivant, attente manifest finalisé.
Rapport `docs/optimiseur/16-11-2026/pg-ro-attestation/02-rust-archives-ro.md`.
Aucun client/recette de compilation inventé avant paquet pinnable ; futur
transportRust n'est pas admis par v1/private-host-psql sans revue contractuelle.
HOLD outil maintenu,0PG/DockerAPI/slot/service/SDK/RET/compilation.

## Copie ED finalisée : préparation source standalone, AVANT code/recette

CopieOUT_T `~/.cache/rag3weaver-build/cx-ed-vertex-deps-b472-v1` finalisée parED,
manifestSHAde797438448c79ae54a1bc313003b0fdead4e37b70b180298ce21690b320c945,
sourceb472,14428fichiers/5,526,823,298octets. G ne lit QUE cette copie.
Audit de sélection74artefacts/fingerprints recoupés/SHA : aucune arête manquante
après résolution des vrais alias Cargo postgres-types serde_json_1→serde_json
etserde_1→serde_core. Directs : PG3179/Tokio64cf/serdeJSON452f. Lock090/compiler
fichier859/cache1.98.1 ; compatibilité/link et provenance compiler PRÉBUILD
restent NONQUALIFIÉS, nativebuild/out.a noncopiés. Audit privé ed-b472/audit-v2.json
SHAaa80f31a1e50550bcae6fd7126c0b3bd267f8360dbebedf43c3d6e63696ad394.

Suite PREPA C7 : publier vecteurs fake NONJOUÉS avant source Rust et recette.
Client standalone ne dépendra que des vraies bibliothèques déjà archivées,
NoTls/SQL2constantes, startupRO et timeouts/application_name nonce, deux sessions
avec drop/reap pilote de liaison avant postcheck. Parent hard5 et kill/reap
restent à qualifier sous futurGOoutil. Aucune intégration au transport v1,
aucun Cargo/rustc/native, SQL ou Docker exécuté pour cette préparation.

## Source candidate Rust après audit, toujours sans GO natif

EDOUT_T final audité74artefacts/SHA, sélectionPG3179/Tokio64cf/serdeJSON452f
cohérente ; aucun GlecteurT, rapportaa80f31a…ad394. Fakes a2c7fd0ac1 publiés
avant source/recette cedf2546b1. Client Rust standalone source + JSON --extern
concrets, NON COMPILÉS/NON EXÉCUTÉS, huit vecteurs NONJOUÉS. Aucun client natif
n'est qualifié : compile/typecheck/link, sysroot et produits natifs éventuels,
supervision extérieurehard5/reap et inspectFORMAT restent prérequis de futurs
GOoutil distincts. Pas de Cargo/C++/SDK/build, aucun SQL/health/DockerAPI.
Le nouveau transport n'est pas faussement déclaré private-host-psql v1 ;
helper pur et trois SHA de69/f949/2907 inchangés. Parent FTS ouvert/HOLD.

## Pilote de qualification Rust : nouvelle PRÉPA C7 avant code

C7 autorise maintenant pilote/runner/fakes/exactpins, SANS exécution. Archive
compiler/lib/sysroot et74dépendances à figer horsOUT_T, jamais lire le T ED.
Mode Rust supplémentaire de validation pure des requêtes CLI/DSN : aucun appel
connect ni runtime dans ce mode, phase PG réelle distincte sous futurRO+GO.
Publier huit vecteurs et raccords avant CPU ; compilation rustc UNEfois capcourt,
sortie exclusive, sansCargo/SDK/build.rs/C++, échec linkSTOPsansrepli.
Borne OS extérieure couvre entrée/stdin/startup/worker5/kill/reap/raw/fsync,
la borneintérieure4poststdin ne remplace pas5. Faux de retards/partial/quiescence
avant toute qualification native, paquet+RO total puis nouveauGO requis.
Postcheck futur = SELECT attribué sur PG EXISTANT, aucun serveur/backend lancé.
Armement Arbre/Embar de la futureglobale est distinct de cet outillage.

## Raccords de qualification matérialisés, toujours NON EXÉCUTÉS

Vecteurs8 CLI/DSN +huit faux initiaux publiésd3dfa3c777 AVANT runner9f350 ;
frontières OS fausses et contrôles pubc04bb3 ; proposition225d8. Dix faux
préparés au total, NONJOUÉS. Un seul compile puis huit parse-only autorisables,
aucun mode capture dans le pilote. Nouveau mode Rust --validate-request retourne
AVANT runtime/connect ; la future lecturePG reste distincte.

Entrées archivées G :317fichiers/660534109octets avec compiler+lib+sysroot+74deps,
manifestbf6b72be485227fadfdda40c133c5b52d4a8a31e7dba1855821c2b34f85d8e25,
READY39e9152c204d5c966307eef4586c7ef278031a82ee58e04c0c237eaeeaccadb4 ;
source et copie comparées globalement après copie, zéro alias T. Aucune de ces
archives n'a été exécutée. Treize pins hôte relevés, loader natif NONQUALIFIÉ.

tool_qualification.py exige GO fichier/SHA externe + vrai Common6439/poste,
sources/inputs frais, sortie exclusive/compile-used, premièreerreurSTOP. Phase
Python interne+Rustc/parser dans groupeOS privé supervisé ; stdin/startup/kill/
reap/captures/fsync inclus dans cap60compile/cap5parser, ENDoriginal borne tout.
Raw partiels/refus/identités conservés ; jamais considérer une qualification
JSON si first-red présent ou code/Common nonVERT. Dernières horloge/autorité
après toutes I/O avant retour. Ceci est SOURCE À RELIRE, pas preuve OS acquise.

Paquet+RO totale puis nouveauGO restent requis AVANT CPU/native. Aucun Cargo,
rustc, testCPU, binaire natif, SQL, DockerAPI, service, réservation, SDK/RET ici.
Les25fakes du modulePGpur acquis restent distincts ; sources/pins inchangés.

### 17 novembre — préparation physique du pilote outil, sans exécution

Code `808be0595ffd033e5ffd69d934eedfceee22d174`, paquet PC
`/home/lucied/.cache/optimiseur-notes/pg-rust-tool-qual-20261117-r1/packet-r1`,
descripteur SHA256 `b9b0e8ea52e83cf39da7969354f7d99c9e76584ce590fb9752a10838b382389c`.
Recette et pins complets dans
`docs/optimiseur/16-11-2026/pg-ro-attestation/03-tool-qualification-proposal.md`.
Dix témoins CPU et huit cas parser préparés NON JOUÉS ; compiler/sysroot et
74 dépendances archivés hors OUT_T. RO totale avant GO distinct ; aucune
compilation, SQL, Docker ou opération de service. Ticket toujours ouvert,
transport réel NON QUALIFIÉ, aucun accord global automatique produit.

### RO R du 17 novembre — réserve bloquante de borne parent

La RO statique du code808be059 relève que NativeBackend.run fait les hashes,
mkdir et fsync dans le parent. Son délai mesuré commence avant ces opérations,
mais seul le subprocess est interrompu : un I/O parent bloqué peut dépasser
5/60 secondes avant le refus. Le paquet b9b0e8ea reste historique NON QUALIFIÉ.
Correctif requis avant nouveau paquet/GO : superviseur extérieur sans I/O de
phase, englobant worker/préflight/compilation ou CLI/publication et retour ;
kill/reap du groupe attribué dans la même enveloppe. Témoins préparés à ajouter
pour blocage avant Popen et après retour natif. Aucun témoin exécuté, donc aucun
rouge runtime revendiqué ; défaut démontré par lecture du chemin d'exécution.

Correctif source préparé après consignation : superviseur fork/pipe extérieur
au worker portant tous les I/O de phase, groupe unique avec les sous-processus,
retour borné et refus si quiescence inconnue. Deux témoins de frontière ajoutés,
12 préparés NON JOUÉS. RO requise avant nouveau paquet ; l'ancien b9b0e8ea ne
porte pas ce correctif. Aucune qualification OS ou autorisation déduite.

### Seconde RO R — contrôle final encore après le verdict temporel

Sur2fe84b600c, le finally exécute encore stop_group/scan proc après le dernier
check avant return ; une lenteur peut rendre un résultat après le cap sans
refus. own_cgroup est aussi évalué avant STARTsupervise. Défaut consigné avant
correction : effectuer tout nettoyage avant le dernier check, puis retour sans
nouvelle observation ; capturer le cgroup attendu au setup global hors phases,
et le revérifier dans le worker supervisé. Ancien paquet toujours historique,
aucun témoin exécuté ni qualification revendiquée.

Correction préparée : finally avant dernier check temporel et retour ; cgroup
attendu injecté depuis setup global et revérifié dans le worker. AST uniquement,
aucun CPU joué. Nouvelle RO requise ; aucune garantie de latence du nettoyage
ni qualification native déduite.

RO R favorable bornée sur32df1b7fdc2fc4d10d8e100c2dcd3a741eef5153 : les deux
réserves statiques sont levées, pas la qualification OS. Paquet PC r2 créé
sans exécution, SHA256 b0795ccbb6f2d3aad0ad7b20bc19e571cd01c6071762c48a751bd45e58142780,
chemin voisin r1 `packet-r2`. Pins et limites dans recette03. Douze témoins,
huit cas CLI toujours NON JOUÉS. RO paquet r2 avant GO distinct ; ticket ouvert.

### CPU distinct C7-G-PG-CPU-32DF-17nov60 terminé

Une invocation exacte autorisée : 12 PASS/0 FAIL/0 ERROR/0 SKIP, rc0,
unittest0.014s/commande0.089162598s, douze sources inchangées avant/après.
Preuves identiques et SHA vérifiés aux deux postes sous cache privée
pg-rust-tool-qual-20261117-r1/cpu-32df-17nov60-01 ; receipt SHA256
6d594431f54494e36d931c074bd25deec5116fcb6a0ae333ae944a9fa3196906.
Ces faux ne qualifient pas superviseur OS ni transport ; huit CLI natifs non
joués. Aucun compiler/SQL/Docker/SDK/service ; aucun retry ni GO natif déduit.
Ticket ouvert, qualification réelle distincte toujours attendue.
