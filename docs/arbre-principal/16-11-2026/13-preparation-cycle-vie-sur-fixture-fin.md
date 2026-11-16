# Cycle de vie sur fixture FIN sans dette : préparation seulement

C7 autorise SOURCE/PILOTE sans RUN. Worktree/branche neufs
cx-arbre-cycle-vie-copie-20261116-r1. Ce document et le code ne donnent aucun
GO writer/provider/CLI/service, aucune nouvelle qualification LIVE.
FINaf2 a trois RO favorables ED/Recherche/Banc, étatRO seulement ; ancien
COPYA/auditEDD restentROUGES, pas de shutdown initial ni durabilité déduits.

## But et primitive publique

Nouvelle COPIE de TOUT le bundle debt-fin-ro-result-v1/copy, fixture déjà
debt0 seulement ; ni original, ni RETOUR, ni modification de la source FIN.
L'ELF CLI927/0e310 et SDK59ac sont inchangés. Pas de compilation, IndexNode,
rescan, purge, LOAD, recovery ni autorisation de recalcul manquant.

`lifecycle.json` garde le fournisseur code nommé/workspace réel readonly,
commands off, entities/relations vides, réactions/montres absentes,
catch_up_on_open=false. Un seul outil `observer_cycle_vie`, graphe d'un seul
SearchSourceNode indexed, exige[data,dense], **signals=[] explicite**,
strict/strict_signals, sortie source.meta ; aucun nœud de signal/recherche.
L'entrée string nonce est liée à query=$nonce par le contrat de gabarit
existant. Source927718 n'embarque la requête que si vector/sparse ; NONE
évite ce chemin. meta.query=self.query.clone() conserve l'écho public.

Admissiondense et calcul de requête sont distincts : la première PEUT payer
une dette. Avant tout writer, SQLglobaldebt doit être vide, et la copie doit
être exactement la fixtureFIN97247vecteurs déjà qualifiée. Une dette trouvée
refuse, aucun paiement autorisé pour faire passer ce test de fermeture.

Source appelle aussi spawn_fts_rebuild inconditionnellement. Préflight RO
des5FTS/IDs/gen/fichiers et des marques fts_pending:/fts_promesse exigé.
fts_promesse doit être vide (même espace refusé), pending suit le trim du
vrai Catalog ; clédupliquée/typeinconnu/refus natif arrête AVANT provider.
Aucune marque n'est effacée. Les invariants après fermer doivent également
refuser tout rebâti/delta ; source.meta ne garantit pas cette absence.

## Publication puis shutdown, même processus

La CLI JSON927 n'émet pas de request_id. Aucun faux écho n'est ajouté :
request.arguments.nonce→SearchMeta.query PUBLIC, result.run observé,
PID/start/exeSHA/cgroup et SHA manifest/graphe/base/profil constituent un
reçu LOCAL de transport, **pas un jeton/certificat durable de la DB**.
Un seul appel en vol, un seul enfant Popen ; aucune réouverture automatique
admise ni essai supplémentaire après échec.

`lifecycle_protocol.py` refuse clésJSONdupliquées, ok/mustReopen/refus,
validation non acceptée/errors/warnings, livraison autre que vide, run absent,
querynonce/targetScope/consistencystrict nonconformes, pendingCount non INT0,
alias pending_count mêmeconcordant, partial nonfalse/signals nonvides/warnings.
Types bool ne valent pas INT0/1. Le stdout brut est conservé avant jugement.

Le VRAI contrôleur persiste+fsync response-publication.json, le relit, puis
permet la requête shutdown. Il conserve la requête, le nonce/run et les SHA.
ACK exact public ok=true/result.closed=true requis, puis rc0 du MÊME enfant.
Ni EOF implicite ni nettoyage de descendants ne remplacent cet ACK. Toute
ligne extra/tronquée/oversize, réponse doublée, réouverture, timeout, fermeture
non0 ou exception de publication refuse. L'erreur initiale n'est pas absorbée
par un catchOSError. Le confineur seul nettoie après rouge sans l'attester.

Capture réelle attendue pendant le writer : exeSHA0e310, PID/start avant et
après observations, CGexact, environnement attendu/actual/diff AVANT assertion,
maps réelles lib+vector59ac. L'ACK ne suffit pas sans ces preuves du même PID.
Cette future capture ne se substitue pas rétroactivement aux mapsReaderFIN
non observées. Aucun processus réel n'a été capturé par la préparation.

## Phase provider explicitement coordonnée

Le backend code nommé peut joindre7878 à l'ouverture même sans dette ;
ne jamais prétendre « aucun provider » parce que seul meta est rendu.
Après clone/debt0/FTSprêts/digest vecteurs/profil : READY puis pause<=60s,
avec réserve de500s pour les contrôlesRO. Aucun GET/PopenCLI avant grant
Embar/C7 frais. Grant NOUVEAU Q/PROVIDER_GO.json, atomique/regularUID,
schéma lifecycle-provider-go.schema.json : neufchampsidentité granulaires
réelsgranite278m768, PID/start/exeSHA, refs/observed/validuntil ; ajoute
nonce, ready_sha256 et no_embedding_calls_authorized=true.

Profil attendu et ledger sont publiés AVANT contact ; santéHTTP200/usage0/0
et identitéPID avant/après, sans restart, autre modèle, chauffe ou requête
embedding de démonstration. max_chars=0 indique zérocorps à calculer, pas
une nouvelle capacité du service. Les santés existantes n'exposent pas de
compteur global de calculs : ne pas inventer « zéro RPC observé » à partir
d'usage0/0. La garantie du scénario est dette0précondition+chemin Source lu,
grant sans calcul autorisé, puis idempotence des données jugée ; toute
activité inhabituelle doit être refusée/consignée par la coordination Embar.

## Idempotence et lectures froides

14sources préparées, dont neufprimitivesFIN existantes et cinq fichiers
lifecycle/protocole/manifest/graphe/schéma ; carry8CC18+376/334/76refs épinglés.
P/Q/S neufs : lifecycle-copy-v1, lifecycle-control-v1, lifecycle-scopes-v1
sous ~/.cache/rag3weaver-build/bascule-live-20261116. --prepare ne copie que
contrôleurs/empreintes ; aucune DB/provider/CLI. SourcePC propre exacte et
tous pins requis avant future admission.

SourcesCOPY02, résultatCOPYA, copieaudit et FIN sont inventoriées SHA/modes/
dirs avant/après/final, aucun de ces quatre bundles n'est ouvert en DB.
Nouvelle copie entière reflink fidèle avant Reader ; exceptionSDK.readers0
uniquementnouvellecopie. Scans incrémentaux après poste, après pause et
juste avant writer ; UNKNOWNattribué fatal, globalinconnu resteinconnu.

Avant ET après writer : digest typé bitexact de TOUS97247vecteurs nouveaux,
UUID/marker/_text_hash/claim, Counter conservant multiplicité ; pas seulement
les469. Ce sont hashes typed, pas composants exportés. Après ACK/rc0 : aucun
WAL nouveau/changé, répertoires inchangés, autres fichiers que DBprincipale
stricts (exception exacte readers0 seulement). La DB principale peut avoir
écrit son journal opérationnel ; sa byte-identité n'est pas revendiquée.

Puis UNE seule requête de snapshot complet (pas de snapshot natif avant),
comparée au RAWbaseline déjà qualifiéFIN via contrat JSONcanonique ; les
champs469exclus du graphe sont tous couverts par digest vectorbit séparé.
Tous autres nœuds/relations/props/anciensvecteurs/config/gens/vis/bijection
et multiplicités restent stricts. FTS5/IDs/gen/fichiers identiques avant/après,
audit.finished globaldebt0/TOUS768finite/markers/cohort469/indexREADY, marques
pending/promesse absentes. ROphase ne doit modifier AUCUN fichier de la copie
après shutdown ; trois sources précédentes+FIN également inchangés.

## Budget proposé, pas GO

900distincts après vrai poste32G/HIGHmax/swap0/quota<=8, outer898+kill2,
common830, hashes/quiescence/publication inclus. Le snapshot completEDD a
coûté environ400s ; proposer600 aurait insuffisamment couvert ces contrôles.
Copie120/SQL90/FTS60 toujours min(cap,END−15). Writer cap180 ET END−500 ET
grantvaliduntil−20, laisse500sRO et santépost sous validuntil. READYpause
<=60 ET END−500 ; réserve tropcourte => refus avant writer, pas de reset.
FinalTERMINE/VERT0/restes[]/nettoyé/busabsent/ownedstartabsents/sourcepins,
deadline posthash/scans et immédiatement prépublication conditionnent le
futur LIFECYCLE_FIXTURE_OBSERVED. Firstred immuable ; aucun retry.

## Préparation vérifiée, limites

Neuf fonctionsCPU hermétiques vertes0.008s :4protocolepublique/étattransport,
2VRAIcontrôleur avec Popen+/proc FICTIFS (publication avant shutdown/ACK/
waitmêmeenfant ; OSError/PermissionError/ProcessLookupError de publication
propagent le même objet sans shutdown ni second Popen),3admissionFTS/
manifest/raccords. Aucun vrai processus, signal, DB/provider/native/CLI.
Ces témoins ne qualifient ni bindingnonce réel ni fermeture réelle.

Revue exacte/source+paquet et GO900 distinct requis AVANT tout run. Positif
éventuel : cyclepublic/shutdown observé et idempotence sur cette fixture,
pas preuve crash/durabilité, pas fermetureinitialeCOPYA ni original/RETOUR/
LIVEmigrés. Tickets restent ouverts jusqu'à arbitrageC7.

## État OPS distinct, relevé G17:30 seulement

PCservice7970 inactive/dead/MainPID0 MAIS socket active/listening : PAS froid.
Aucun HTTP n'a été envoyé pour le réveiller. PortablePID2698568/start82465764,
ELF0e310/source927/mapsSDK59, relevéG ; pas permission de serviceop déduite.
Masterb0 vs927 porte9Rust dont6produit (stable_order+rateadressehost), C++/
vector/locks/templates/gitlinks/entréesCLI-host inchangés selonG ; ELF927
n'est pas renommé master. RETphysiques conservés2posts ne sont pas nouveau
GOlivemigration. Anciencoldguard82d≠pin3ba reste HOLD ; ce pilote de fixture
n'utilise ni son anciennegate ni le busmanager. SDK/CLIréels épinglés927/59.
