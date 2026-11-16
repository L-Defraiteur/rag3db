# Vertex : sept juges CPU verts, intégration encore ouverte

APP **c38fc3e4ddd8c806de904c7d08eb43b4988fec99**, PILOTE **441002d11bd07b7e8b77b0351d6e522ac5a9c6db**, GO exact C7-VERTEX-GREEN-441-16nov. Ancien rouge fe40/eb026 et preuve4d54/39SHA141a conservés ; aucun de ces rouges n'est repeint vert. Source et pilote sont restés propres et figés pendant cette allocation.

## Résultat réellement observé

Compilation CPUdaemon,openai-llm sans défaut, offline/locked par cargo_garde : rc0,36.78s au résumé Cargo. ArtifactJSON unique, inventaire des sept noms prévus, aucun ignoré parmi eux. ELF original/copie SHA **8cbcefea72d6f584cec7e2c87f42eaeda32c9026ba6c737767a200ae65395406**, **195368680octets** ; préfixeAPP9 retrouvé indépendamment dans la copie àoffset8049153. LedgerSHA **80d72ff5b456155fa03ee226c218c362a8e2223a6f02db531c5910ba50185e33**, copié avecELF et réhaché sur les DEUXpostes AVANTpublication1190179.820097180 et avanttoutjuge.

Sept exacts distincts, chacun **1réussi/0échec/0ignoré** :

- fixture_token_source_changes_its_cached_token_only_after_invalidation ;
- a_fixed_api_bearer_is_not_refreshed_after_unauthorized ;
- vertex_never_refreshes_or_replays_after_a_stream_fragment ;
- vertex_rejected_token_is_refreshed_once_before_opening_a_stream ;
- vertex_persistent_unauthorized_is_named_after_one_refresh_only ;
- declared_vertex_connector_retains_injected_source_and_refreshes ;
- dynamic_vertex_auth_debug_never_discloses_a_token.

Namespace openai_llm::vertex_refresh_tests. Lessept invocations rc0 se déroulent1190179.932874971→1190179.979972287. Chaque résumé indique1954filtrés : ce sont des exacts, PASune libentière. Aucun premierrouge danscette portée. Renouvellement et401persistant sont constatés avec UNinvalidation et les séquencesAuthorizationfactices attendues ; SSEpartiel n'est pas rejoué et BearerAPI fixe ne se renouvelle pas.

## Capacité réellement raccordée, limite des preuves

Le client conserve Arc<dynAccessTokenSource> et acquiertunjeton àchaquetentative ; HTTP401 avant200 invalideUNEfois,401persistant devientuneerreurnommée. La branche déclarative réelle est exercée via le même helper de résolution que connect_llm, avec constructeurduVRAIclientVertex HTTPfake injecté ; projet/région/modèle/origine/contexte et acquisitionzéro àconnexion sont assertés. L'implémentation TokenSource→capacité est compilée ; **from_env, lecturecredentials, échangeOAuth réel, cloud, F300 et durabilité des passes ne sont PASqualifiés**. Pas de reprise401dansSSE mêmeavantfragment ; le429SSEpréfragment préexistant n'est pas modifié par ce lot. Aucun appelAPI7970/7878/7879/PG/modèle/service partagé ici.

Aucune featureSDKnative ni exporterSDK, aucunGDB/mapsàmain revendiqué. LespinsSDK ontétérelus commeprovenance seulement ; CPU/native_runtime=false ne prétend pas prouver absence d'accèsGPU globale. Aucun déploiement ni intégrationmaster autorisé par cesjuges.

## Un seul target et une allocation distincte

UNIQUE Ttarget-cx-ed-stable-tie-identity, conservé depuisrougefe40. Captureprépa égaleAPRÈSvraiposte : archive39fichiers141a, oldfe40+piloteb026+gitlinkpropres, autoritéhistoriqueTERMINE/verdictROUGE1nettoyé/restes[]/busabsent, ELF53b5original/copies/archive, Tdev55/inode166664078/UID1000/mode0755 et marqueurpropriétaire,1936fichiersd'empreintes hashés. Freshscanoldowned/exe/cwd/FD avantmutation vert.

Deux appels RÉELS gardecommune rc0 : oldfe40 sansreprise, puis APPc38 avec REPRENDRE1. Marqueurappc38 attesté aprèsgarde ; aucun marqueur/invalidationPython ni secondeT. LesartefactsCargo duT peuvent légitimement être remplacés : anciencheminunit-tests a été recompilé en8cb ; l'ancienELF53b5 resteconservé dansProuge/archive/portable, il n'est pas réattribué àc38. Tfinal **1334273525octets**, conservé. Q1/Q2 etpilotesHOLD, ressourcesP/Q3/S etanciennespreuvesconservés.

Poste réel32GiB/HIGHmax/swap0, cpu.max500000/100000, jobs8maximum. Outer1198+kill2 aprèsposte, common1130 incluantcleanup ; INITIAL **1190067.356433272**, END **1191197.356433272**. TMPcourtdisque/étatsXDG-RAGfrères/CARGO_MANIFEST_DIRexact/envsanscredentials. Composé **CPU_VERTEX_CHECKED/rc0**, elapsed **113.024376022s<1130** ; hashELF/copielib/outils/SDK, sourcepropre, scans puis contrôleAPRÈSopenavantpublication compris. Lescaps individuelles sont plafondsintersectésEND, pas une promessedesomme de900+120+7×30.

Scope **validation-u8f4yvpv VERT/code0/restes[]/nettoyé=true**, autoritéTERMINE/busabsent. Scanfinalowned_alive[]/found[],12UNKNOWNglobauxnonattribués conservés, UNEexemptionFDposteancêtre strictementidentifiéeUID/PIDstart/exeBashSHA/argvexact ; aucunautreFD/W/T exempté. Aprèsretour, observationindépendante1190225.451827515 :104identitésowned,0vivant/0found/0exemption,cgroupetbusabsents ; les12UNKNOWNglobaux restentUNKNOWN, aucuneabsenceGPU déduite. Cetobservépostretour n'étend pas la duréeducomposé113s.

## Conservation et suite

**59fichiersPC/portable taille+SHA conformes**, manifeste **7a73c248158fe1587cb0045db06aca68f63589528b716416e37fbe2b38cba0a6** : [inventaire](vertex-token-green-v1-manifest.json). Archives ~/.cache/rag3weaver-build/cx-preuves-ed-vertex-token-green-v1 et suffixe-local, sousdossiersclient/controller/supervisor. ELF/copielogs/CargoJSON/profilsansjeton/capturesreprise/ledgerpubliés/receiptsPUBLICS/verdicts/scans conservés. autorite.json etjeton noncopiés danscepaquet.

Ticketouvert, propositioncorrectiveCPUqualifiée seulement. Revueindépendante puis candidatneuf/masterfrais etqualificationunion pertinente restentàarbitrerparC7 ; pas de GOmaster, F300, cloud, nouvelOAuth, retraitsource/target/branche ni nouvellepasse implicites. Le rougeinitial etses limites demeurentconsultables.
