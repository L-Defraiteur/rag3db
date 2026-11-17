# Correctif groupé sorties/identité/flags — source seule

[Ticket FS ouvert](../tickets/2026-11-16-l-admission-du-corpus-fs-ne-garantit-pas-la-vue-consommee.md).
Source **13298f7db3443956ea01e088b0c014fd8c60c2ff**, lucivy branche neuve
`cx-lucivy-fs-output-fix-17nov`, parentbdf83ad. Quatre fichiers,140ajouts/
49retraits. [Pins complets](preuves/fs-output-fix-13298f7/source-pins.json).
WT dédié propre, aucune mutation arbreLucie/PC/target, aucun Cargo/rustfmt/
test/ELF/corpus/service/proc/DB. Diff lu, SHA calculés, seize noms textuels
conservés et git diff --check sans défaut. **Source non compilée ni exécutée**.

## Preuves avant correctif

Six défauts initiaux réellement rouges0f/positif, bb5707c6a5 ; cinq nouveaux
défauts sorties/ID/flags réellement rougesbdf/positif,5094f2b29e. Rapport28,
57+6 entrées publiques et18copies privées sources/outils/deuxELF en444
rehachées deuxhôtes AVANT ce diff. Deux RO preuves favorables : Banc d6c49ffecf
public+privé deuxhôtes ; Recherche publicGit+privéLOCAL, limitePC explicite.
Sources et bruts antérieurs inchangés ; GOE75 consommé, pas GOvert déduit.

## Frontière commune input et sorties

La primitive `InvocationPaths` unifie l’identité effective de l’entrée et des
sorties d’une invocation. Champs privés, valeur sérialisable : input_root,
manifest, reports. Le callback reçoit **snapshot ET chemins validés** ; le
vrai helper rapport choisit un slot de cette valeur.
Cela porte l’invariant « les octets de l’entrée ne servent jamais de sortie,
et deux sorties ne désignent jamais le même fichier ». Coût : résolution de
namespace/metadata avant capture et un petit objet par invocation, pas de
nouvelle logique moteur ou algorithme de matching.

`validate(plan)` précède désormais TOUTE résolution/metadata/réservation.
Flags invalides → ErrFlags sans événement FS ni reçu, conformément à30561.
Le témoin flag_presence historique est explicitement aligné : il garde le
refus précis et exige maintenant zéro événement et aucun reçu. Le nouveau
rouge causal invalid_flags reste inchangé hors arité de callback.

Tous les outputs sont déclarés ensemble. Résolution du plus long préfixe
existant de l’entrée, canonisation des parents **existants** des sorties,
comparaison entrée/sorties lexicales ET résolues, unicité des destinations,
identités UTF8 sans perte et refus des fichiers déjà présents. Aucun output
n’est réservé tant que TOUS n’ont pas passé. Un reçu existant conserve
RefusalManifest/AlreadyExists et ne déclenche aucune capture. Une racine
absente conserve son chemin résolu de parent, puis son refus IO.metadata/
NotFound et reçu REFUSED sur outputs sûrs dans le raccord CaptureFs.
Une erreur de résolution qui empêche de prouver la frontière retourne un
refus précis sans reçu ; elle n’est pas rebaptisée donnée admise.

La réservation du reçu emploie le chemin résolu ; body schema1 ajoute paths
aux pins/plan/panel/verdict/refus. Les refus de données Empty/IO/UTF8/panel15
restent publiés/hashés/fsync sur la sortie admise avant toute consommation.
La capture unique non triée, cap5000/politiquev2/exclusions et snapshot partagé
index/oracle de066 restent inchangés.

## Effets réels et identité

`with_v3_report` prend InvocationPaths+numéro de slot, vérifie le slot, ouvre
**create_new**, puis seulement appelle le builder et le panel. Aucun mkdir,
aucun File::create/troncature, aucun usage du chemin original aliasé. Une
création concurrente du fichier après prévalidation refuse encore avant le
builder par create_new. Le reçu ADMITTED reste antérieur à ce helper.
Le vrai callback natif ET le témoin sorties utilisent ce même raccord.

Le natif déclare V3_CORRECTNESS_REPORT comme troisième entrée d’invocation
(avec V3_CORPUS/V3_ADMISSION_MANIFEST), soumise aux mêmes contrôles. Défaut :
CARGO_TARGET_TMPDIR/v3_ground_truth_correctness.jsonl, parent déjà fourni par
Cargo ; plus de création implicite de lucivy-reports. Header et quinze lignes
sont JSON, écriture/sync avant assertions. Les diagnostics historiques gardent
leur propre chemin/modes, hors parcours de correction.

`document_ordinal` refuse None par **OrdinalError::Missing**, vérifie conversion
u64→usize et borne par OutsideView ; aucun fallback0. La vraie boucle search
utilise toujours ce helper avec expect : identité absente/invalide fait échouer
la comparaison. Some(0) reste valide, contrôlé par le positif distinct.

## Témoins et recette proposée suivante

Seize noms conservés : six refus initiaux +positif snapshot, flags/corpusabsent/
reçuexistant, cinq nouveaux refus +positif sorties. Les cinq assertions des
rouges nouvellement acquis restent celles de bdf ; seul l’appel du rapport
prend maintenant les slots admis. Le positif ajoute la concordance paths
manifest/reports dans le reçu. Aucun skip/affaiblissement des causes, deux
positifs excluent les refus généraux. **Aucun des seize verts observé**.

RO TOTALE SOURCE/contrat demandée Recherche+Banc sous ce ticket, puis nouveau
pilote GREEN/pins/Q/nonce/préflight/ROcouple. Proposition : mêmeT/seedDBDB/
releaseoffline--locked, deuxELFCargoJSON/ledgerdurablecopiefsyncACK avant toute
invocation ELF ;16 exacts, chacun1PASS/0FAIL/0ignored/15filtered/rc0. Dix anciens
nonjoués sous GOE75 devront être réellement contrôlés dans ce futur vert,
pas supposés passer parce que bdf avait compilé. Sauvegarder les deuxELFbdf et
l’historique avant compile, conserver double lock effectif et mêmes bornes/
Common/poste32G/retours réels ; corps secondELF/index/corpus encore exclus de
cette première proposition. Paquet exact et GO distinct avant toute exécution.

## Limites maintenues ouvertes

La résolution garantit la séparation dans le namespace observé ; ce ne sont
pas des FD de répertoire ancrés ni une preuve contre une mutation hostile des
ancêtres/montages entre validation et usage. Les créations exclusives protègent
contre les écrasements de feuilles. Cette portée doit rester explicite en RO,
pas se transformer en garantie OS universelle. Root/outputs inaccessibles ou
non identifiables refusent, jamais fallback/silence/résultat accepté.

Le replay depuis manifeste n’est toujours PAS implémenté. Ce diff ne qualifie
pas la construction moteur réelle, une parité corpus, l’exécution secondELF,
core/neufHOLD/A-B/ULP7.2/PR42. Ticket reste ouvert ; aucune livraison main/tag.


RO TOTALE SOURCE13298f7 Banc favorable PRÉPA publiée c7e2e5d381, rapportBanc03.
Quatre SHA Git conformes/seize noms conservés, diff complet relu : InvocationPaths
root/reçu/reports résolus/uniques/horsinput/absents avantreserve ; callback
snapshot+paths, rapport slot/create_new sansmkdir avantengine, None→Missing/
OutsideView sansfallback0, validateFlags avant toute résolution/metadata/output
et alignement30561 explicite. Cinq assertions rouges conservées et positif
renforcé. Aucun nouveau blocage statique identifié ; namespace observé sans
FDancêtres/garantiehostilerename et replay absent restent explicites.
Seize verts NONOBSERVÉS, aucune compilation/import/parse/test/PC/proc/DB/
corpus/ELF/target/service par Banc. Recherche encore attendue ; nouveau paquet
GREEN/pins/ROcouple/GO C7 distinct requis, dixanciens+sixnouveaux à jouer
réellement, barrières ledgerACK/doublelock/physique à conserver. Ticket ouvert.
