# `indexed: false` est ignoré pour un champ texte

- **État** : ouvert (16 novembre 2026). Noté le 14 novembre au ticket du poids du plein texte, jamais ouvert à part.
- **Gravité** : faible, mais silencieuse. Un appelant qui déclare un champ texte `indexed: false`, pour le stocker
  sans le chercher, paie quand même son index entier (dictionnaire, postings, positions). Et rien ne le lui dit.
- **Atteignable en service** : seulement si un manifeste ou un appelant pose `indexed: false` sur un champ `text`.
  Aucun manifeste de rag3weaver ne le fait aujourd'hui.
- **Touche rag3weaver** : non.
- **Famille** : produit
- **Pour** : lucivy

## Ce qui se passe

`lucivy_core/src/handle.rs`, `build_schema`, branche `"text" =>` : le champ reçoit toujours
`TextFieldIndexing` (tokenizer `RAW_TOKENIZER`, `WithFreqs` en v3). Seul `stored` est lu, jamais `indexed`. Les
types `u64`, `i64` et `f64` lisent bien `indexed`. La branche `"string"` l'ignore aussi : elle pose toujours
`STRING`, donc un champ indexé. L'avertissement et la doc valent pour les deux.

## Ce qu'il faut faire (ce lot-ci)

- Un **avertissement** à la création, une fois par champ concerné, quand un champ `text` ou `string` porte
  `indexed: Some(false)` : `eprintln!("Warning: …")`, la forme que `handle.rs` emploie déjà (l. 231) ; ne pas ajouter de dépendance. En
  anglais, par exemple `field "<name>": indexed=false is not supported for text and string fields; the field is indexed`.
- **La doc** : la doc du champ `indexed` (`lucivy_core/src/query.rs:93`) et la section des champs de `lucivy_core/README.md` disent que `indexed: false` n'a pas d'effet sur un
  champ texte.
- **Ne pas** honorer `indexed: false` dans ce lot. Un champ texte stocké sans index change ce que les requêtes
  `contains` et les spans peuvent faire sur lui. C'est un choix de Lucie, à lui présenter à part, avec ce que ça
  coûte et ce que ça casse.

## Comment le prouver

- Rouge d'abord, dans un test neuf `lucivy_core/tests/test_text_field_indexed_false.rs` : créer un index avec un
  champ `text` à `indexed: false`, et exiger que la création le signale. Le moyen le plus simple de l'observer : une
  fonction pure qui rend les avertissements du schéma (`schema_warnings(&SchemaConfig) -> Vec<String>`), appelée
  par `build_schema` et testée directement. Rouge tant qu'elle n'existe pas ou rend une liste vide.
- Vert ensuite, puis `cargo test --release -p lucivy-core` en entier.

## Préparation du témoin, 16 novembre

Porteur lucivy ; branche poussée `cx-lucivy-indexed-false-warning`, tête
`07ccd12e484ea20f4ae85d23639f9da7316acbfd`, base main a1c0dfd.
Worktree portable `/home/lucied/git_workspaces/cx-lucivy-indexed-false-warning`.
Deux fichiers : handle.rs (API scaffold renvoyant liste vide, pas de correctif),
`tests/test_text_field_indexed_false.rs` (text/string false doivent avertir,
true/omis et champs numériques ne doivent pas ajouter d’avertissement).
**NON compilé, NON joué, rouge pas encore observé.** Aucun appel de cette API
ajouté à build_schema ; aucune sémantique d’indexation modifiée.
Prochain pas après créneau CPU : voir ce témoin réellement rouge avant de
remplir schema_warnings, l’appeler et compléter query.rs/README. Ne pas confondre
préparation du rouge et preuve exécutée. Aucun lourd pendant fenêtre G/Arbre.

Conception brève avant correctif :
`docs/lucivy/16-11-2026/04-conception-avertissement-indexed-false.md`.
APIpurepublique dans handle, avertissementparchamp/parappelbuild_schema après
validate ; pas de déduplication globale, de nouveau flag, ni d’effet d’indexation
modifié. Noms échappés, texte anglais ; docquery/README à compléter aprèsrouge.
Préparation du témoin corrigée en littéraux bruts Rust ; toujours NON compilé,
NON joué, aucun correctif tant que rouge d’assertion pas vu. Fenêtre G/Arbre
respectée, aucun nouveau lourd lancé.

Recette bornée publiée pour RO puis GOslot :
`docs/lucivy/16-11-2026/05-recette-rouge-indexed-false.md` et pilote
`outils/qualify-indexed-false-red.py`, sourcefixe0c795f99, budgetproposé600s après
vraiposte/32G-HIGHmax/j4, compilationoffline+inventaire+exact1configpure.
Rougeassertion attendu distingué compilation/inventaire/confinement ; aucun
lancement effectué. Corpus7.2/PR42 restent deux blocages distincts noncontournés.

Suite RO C7 : chemins concrets/PQS et4hashes copiés préparés sur luciepc,
précontrôle `preuves/indexed-warning-preflight-v2.json`, targetABSENT et P/scope
ABSENTS, Qneuf UID1000 et nonce de provenance. Pilote refuse les composants
symlink sansresolve, créeT/provenance exclusivement et abaisse j4 au quotaCPU
hiérarchique réel. Admission600 aprèsvraiposte avec END conservé, commande
littérale sansplaceholder dans recette05. Supportd395 copies privées, aucun
cache déployé modifié. Aucun Cargo/build/test joué ; attend admissionexactefinale.

## Rouge exécuté le 16 novembre, source0c795f99

GO exact C7 APP0c795f99/pilote86989d7ab5/Qv2nonce3552903efab590b44f44cd8002f7b028.
Compilationoffline réussie (release1m02s), exécutableSHA
`c2478e972de2295a79c41cd651461653f38706e02cccd2ceace45c49fe33b01b`,
inventaireexact1 puis UNtest : **0passé,1échec,0ignoré**, code101 à l’assertion
left:[] contre deuxmessages body/label. Ce n’est ni une erreur de compilation ni
un refus de confinement. first-red.json : expected_assertion_red_observed.

[Brut](../16-11-2026/preuves/indexed-warning-red-0c795/logs/test.log),
[manifeste](../16-11-2026/preuves/indexed-warning-red-0c795/SHA256SUMS),
[quiescence](../16-11-2026/preuves/indexed-warning-red-0c795/quiescence.json).
Portée ROUGEcode1/erreur=null/restes[]/nettoyétrue, autoritéterminée ; scanPID/start
vide et sourcepropre. END1180583.723555907, observationfinale1180124.091534846,
aucunreset. Quotaeffectif5CPU→j4,32G/HIGHmax. Target crééexclusivement, provenance
nonce/UID/source ; aucune adoption. Pas de retry, de corpus, de libcore entière,
de service ni de baseLucie. Rouge conservé avant tout correctif.
Reste déjàprévu : implémenter schema_warnings/call/doc selonconception04, puis
validationverte sur source/pilote/portée frais admis ; aucune réactivation duGOrouge.

## Correctif préparé après le rouge, NON qualifié

Branchelucivy cx-lucivy-indexed-false-warning, tête
`890c86af44989f3fea6c6fa62fd40d688b4084df` poussée. Fonctionpure implémentée,
text/string+Some(false) seulement, ordre champs et noms Debugéchappés ; appel
Warning aprèsvalidate dansbuild_schema. Doc FieldDef::indexed et READMEcore
explicites, sans changer l’indexation ni honorerfalse pourtexte/string.
Témoininitial conservé, plus juge nomséchappés/une ligne et juge schéma prouvant
text/string toujoursindexés, numérique false nonindexé. Trois tests préparés,
**aucune compilation ni exécution de cette source**. Ni PR ni fusion engagée.

Reste : RO diff puis nouvelle recette/portée verte source890, targetprivé
provenance0c+quiescence à admettre explicitement si réemployé, logs/reçu neufs.
Le GOrouge se termine avec sa preuve ; pas de second lancement par ce GO.
Suitecore complète toujours bloquée corpus7.2, pas déclaréeverte via ce lot.

RO ciblée demandée par C7 à recherche et mémoire, avant GOvert : source890c86a,
contrat configwarning/text|stringSomefalse, échappement des noms, ordre stable,
indexationschéma inchangée et docpublique. Preuve rouge0c conservée. Reste :
leur retour RO puis couplevert3juges, réemploi du seulTnonce3552/receiptancien
terminé/ELFrougeSHA conservé, contrôle identité/quiescence frais aprèsposte,
600s uniques, pas de libcore complète/corpus/ulp/PR42 inclus par ce seul juge.

Préparation verte : pilotesqualify/admit-indexed-false-green publiés60735fa9af
puis14cbf7af3d (basepartagée rouge vérifiée avantlecture, codeconfig890 inchangé).
Paquet privé PC cx-lucivy-indexed-warning-green-16nov-v1 copié, ELFrougeSHAc247
conservé dans red-test-elf avant toute compilation, Tnonce3552 non modifié,
WPC avancé propre890. **Précontrôle bloqué avant admission** : PermissionError
Errno13 sur /proc/1247/exe lors du scanmetadata global des processus UID1000.
Aucune tentativeCargo/test, aucun reçuvert créé ; preflight.json NON écrit.
Pas d’élévation ni de contournement. Reste C7 : décider critère de quiescence
admissible pour le réemploi avec cet accès /proc protégé, puis nouveaupréflight
et recettefigée avant GO. Le rouge0c et son autoritéterminée restent intacts.

Après arbitrage C7 : contrôle de réemploi borné, pas d’absenceuniverselle/proc.
Nouveaupilote/relevépartiel AVANTassert, UNKNOWNnonattribués conservés, owned ou
attribués(UNKNOWNinclus)bloquent ; nonce/UID/inodes/oldCommonTERMINE/ELFrougeSHA
et scansfrais avant/après. Trois cas synthétiques publiés puis joués3/0,
aucunCargo. Nouvelle recette06 et paquetvertQv4 figés (préflight12UNKNOWN,
aucunowned/attribuéactif), Tanciennonce3552 unique, jamaispurgé ; nouvelog/scope
encoreabsents. Source890 reste NONJOUÉE, demandeRO/GO600VERTdistinct avantCargo.

## Vert ciblé exécuté :3config, portée bornée

GO exact C7 source890c86af44989f3fea6c6fa62fd40d688b4084df / pilote
de36488bd948babf25548f78e80dcf7e89c5fe95, Qgreen-v4nonceb8c7e19b6185c7d2a4ac2aa690ed2bfe.
Compileoffline réussie6,24s, inventaire3 puis troisexacts **1/0/0 chacun**,
aucunretry. Le troisième log montre réellement les2Warning de build_schema et
vérifie que text/string restentindexés, numérique nonindexé.
ELFvertSHA d10152c78102ee701ef2b808fc2627d7e8044866fcbc5cebe9e5a23550958adc ;
ELFrougecopiéSHAc247 inchangé. MêmeTnonce3552/inode164536077/dev55/creatorSHA5a1671,
aucunepurge ni nouveauT. Sourcespropres aprèsrun.

[Bruts/manifeste](../16-11-2026/preuves/indexed-warning-green-890c86a/SHA256SUMS),
[finalCommon et relevépartiel](../16-11-2026/preuves/indexed-warning-green-890c86a/final-evidence.json).
ScopeVERT0/erreur=null/restes[]/nettoyétrue, autoritéterminée PID1575006/start118237703,
ownedduCommonvert absents ; scanaprès12UNKNOWNnonattribués conservés, aucune
absenceuniverselle/proc revendiquée. END1182974.134124184, contrôlefinal1182433.62054773,
pasreset. Logs PIDcargo1575020/start118237725, jobs4 réels sousquota et32G/HIGHmax.

Reste : RO preuves puis décision d’intégration/CI ; corecomplet/corpus7.2/ULP
restent horsqualification. Pas de fermeture de ce ticket sur la seule preuve
ciblée ; aucun nouveau lancement, PR42/fusionmain/tag ou service par ce résultat.

Suite C7 : préparation qualification adaptée au coreentier, pas nouveau ciblé
sans cause. Audit fetch : origin/main lucivy restea1c0dfd, candidat890 est déjà
son descendant (aucun commit entrant à intégrer, pas de rebase artificiel).
Banc sollicité pour RO des preuvescae3 sansrun. Reste : cartographie des tests
corpus/ignorés et recettebornée explicite avec provenance/cachesknownowner ;
aucun ABSENT_OK ni faux0ignoré, pas d’équivalence2.6/7.2 ni validationULP.

## Proposition de qualification core et RO banc

[Recette07](../16-11-2026/07-preparation-qualification-core.md) : candidat890
déjà union de maina1 ; deux pilotes proposés, syntaxeAST seulement, paquet
non admis. Proposition1800s/32G/HIGHmax/j<=4, profilcorepar défaut+doctests,
inventaires complets et ignorés nonqualifiés. C7 doit arbitrer Linux2.6.0
cacheprovenancé pour cette régression seulement avant finalisation du paquet ;
aucune extraction ni Cargo. ULP/corpus7.2 toujours en attente.
Banc RO favorablebornée3068ad7 : 28hashes conformes, 3exacts verts ; nuance
UNKNOWN12entrée/**13posttests**/12final tous nonattribués. Pas de rehashELF
ni nouveau run par Banc. Ticket reste ouvert ; lucivy porte la suite, C7 le GO.

Admission PREPA C7 reçue : Linux2.6.0 archive4d0299 et clone1b22409 admis
pour corewarnings seul ; copies privées/nonce/SHA/licence/sous-ensembles,
originaux RO, pas lecture.env/.vault et pas Cargo avant GO distinct.
Audit statique suivant : bench_vs_tantivy est un test par défaut, chemins
persistants /home/luciedefraiteur codés en dur, réemploi d’index existants et
remove_locks récursif ; impossible de le qualifier ici par racine privée via
les options existantes. À garder HOLD explicite, pas de lancement/skipvert.
Autre prérequis : test_luce_playground_search retourne avec messageSkipping
si playground/dataset.luce absent. Vérifier ce fichier exact sans substitution
de snapshot. Recette et pilote doivent rendre ces absences nonqualifiées.

Préparation corpus admise effectuée sans Cargo : Qcore-v1nonce
e14c4d52ea84cbb255feeb63679cfcb3, 15007 fichiers Linux2.6.0 sous racine
privée réelle /tmp/lucivy-cmp, 8507 blobs publics réguliers du clone1b22409
sousQ/corpora/rag3db. Archive originale hashée avant/aprèsinchangée ; cloneHEAD
inchangé, seules lectures d’objets suivis permis. .env/.vault/symlinks/gitlinks
exclus avant lecture des blobs. Licences/manifeste complet/sous-ensembles conservés.
Écarts de profil reliés au ticket distinct
`2026-11-16-le-profil-core-par-defaut-inclut-des-diagnostics-non-isoles.md` :
9targets HOLD proposés ; pas de fauxcorecomplet.

Paquet PC Qcore-v1 préparé, préflight23dd3516 ; recette08 publiée avec
commande littérale1800s/32G/HIGHmax/jquota<=4. APP890/piloteSHA2fa59faafe18/
admission3cde2f1e, Common6439/poste79e3 inchangés ; Tunique et ELFvertd101
réemployés knownowner, reçuvertTERMINE, scanspartiels12puis13UNKNOWNnonattribués
conservés. Manifestes complets Linuxd4faf3/clonebb7089 rehashés après copie ;
licences et sous-ensembles300/400 archivés. Zéro Cargo/ELF lancé, scope/logs
absents. 9targetsHOLD déclarés et ignorés nonqualifiés ; jamais coreentiervert.
Reste RO paquet/pilote puis GOdistinct C7 pour profil adapté, ou arbitrage
supplémentaire sur les trous. Critère de fin conserve ces limites explicites.

Action de relecture portée par banc : RO uniquement du paquet de préparation,
manifestes/empreintes, pilote/admission/échéance et neufHOLD ; aucunrun/Cargo/ELF,
aucunscan de base/service. Retour attendu lié à ce ticket avant GO C7 distinct.

RO banc de21b18 : cohérence PRÉPA favorable bornée,13SHA conformes ; pas
rehashphysique des corpus par banc. Point de lancement précisé dans recette08 :
**cwdQ exact horsW/T**, sinon autosignalementattribué par collect_scan et refus.
Pilote2fa59/preflight23dd inchangés, pas de nouvelle exécution ni GO reçu.
Rapport banc à pousser ; transmettre ce complément avant décision C7.

GO C7 exact reçu : allocation C7-LUCIVY-CORE-890-16nov, APP890/pilote2fa59/
admit3cde/preflight23dd, noncee14, ENDunique1800. ParentcwdQ obligatoire,
POSTE_MEM_MAX=32G, POSTE_SWAP_MAX=0, POSTE_CPU_QUOTA=400%, HIGHmax, jobs<=4.
Ces variables complètent la commande08 conformément au GO reçu ; aucun helper
ni pilote modifié. NeufHOLD/ignored explicites, full_corefalse. Premier rouge
imprévu : arrêt sansretry, aucun merge/main/tag/ULP7.2/PR42 autorisé.

Lancement effectif unique : Common PID3528029/start119072861, échéance
1192525.699990913, jobs4, cwdQ vérifié, memory.max32G/highmax/swap0/cpu4 réels.
Le shell distant fish a refusé la ligne de garde shell `set -eu`, puis a
poursuivi cdQ et la commande autorisée ; aucune seconde admission/lancement.
Ce diagnostic de shell n’est pas un test rouge. Ressources/cwd/autorité
effectivement observés et sauvés dans Q/launch-resources.json.

Premier ROUGE observé sous allocation C7-LUCIVY-CORE-890-16nov :
`test_derived_in_ram::derived_in_ram_answers_like_the_files`, log15,
0pass/1fail/0ignored, rc101. Les300 premiers fichiers réels Linux2.6.0 sont
chargés ; le panel strict mutex_lock ne trouve rien (assertion ligne170).
La condition de contenu du panel n’est donc pas satisfaite par ce sous-ensemble ;
aucune conclusion de régression du correctif warnings ne peut en être tirée.
Arrêt effectif sans retry ni nouveau ciblé, aucune assertion assouplie.
CommonROUGE101/erreurnull/restes[]/nettoyétrue/autoritéTERMINE, scopewbvzcmca.
Reste collecte finale quiescenceowned/empreintes et transmission C7, pas de
recomposition de corpus ou nouvelle passe autorisée. Le ledgerELF global a
été écrit après l’arrêt (62artifacts), pas une preuve de ledgerglobal avant
les tests ; les inventaires par target exécutée précédaient leur invocation.

Suite C7 aprèsSTOP : preuve RO sur les300 fichiers exactement épinglés avant
conclusion RAW ; ne pas étendre une absence de ce sous-ensemble au corpusentier.
Défaut de préparation identifié : l’admission vérifiait extension/taille/compte
et identité, mais pas les préconditions de contenu du panel (résultats nonvides).
Une future admission doit vérifier Source.dataset/PANEL sur contenus épinglés
AVANT tout test. Travail RO actuel : compter mutex_lock sur ces300 fichiers et
conserver hashcasenatif/source/ELF/panel/temps. Aucune correction de sélection,
aucun nouveau test ni redémarrage autorisé. Proposition par défaut : conserver
ce cas HOLD avec défaut de recette explicite ; choisir un autre sous-ensemble
nécessiterait une conception et admission distinctes, pas une assertion affaiblie.

Résultat final et preuves : rapport09-rouge-core-sur-le-sous-ensemble-linux.md.
Compile39.95s/62ELF,16inventaires,117passés/1échec/25ignorésobservés ;46targets
nonatteints/doctestsnonjoués, warnings3anciensvertsnonrejoués. CommonTERMINE101
nettoyé/restes[], propre890, ownedabsent,11UNKNOWNnonattribués final conservés,
observé1190873.393703287 < END1192525.699990913. Tmarkerinchangé/scratchvide.
RAW sous-ensemble300 : zéro mutex_lock littéral ET ASCIIcasefold, sans
réinspection globale des23514fichiers ; preuveparfichiercd103654 etcasnatifa03b6222. Aucune
absencecorpusentierdéduite. Admissiondataset/PANELàcorrigerAVANTfuturtest ;
propositionHOLDcecasavecdefautrecetteexplicite. Ledger62SHA tardif aprèsSTOP,
inventairesintercalésavantchaqueappel etnoncomplets : limite de protocole
conservée, pasrattrapage enpreuvepré-test. ROprépaBanc94a2cde reliée ; pasRO
exécution encore. Aucunretry ni nouvelle sélection sans décision C7 distincte.

Nouvelle préparation RO autorisée par C7 aprèsa05e : lire le contrat exact du
test et sa sélection hardcodée, localiser TOUS les termes du panel dans les
seules15007sources publiques Linux2.6.0 déjà admises. Distinguer300 vs corpus
entier et2.6 vs7.2 ; aucune lecture.env/.vault/base, aucun test/Cargo/exécution
de fixture. Pas de permutation posthoc pour verdir, pas d’assertion assouplie.
Livrable : contrat et données d’admission épinglées, proposition de test
d’admission vérifiant l’objectif RAM/Files et ses préconditions avant heavy.
Rouge pure/mock à concevoir puis RO avant toute correction de code/RUN ;
aucune autorisation implicite de cette correction ou d’une nouvelle passe.

Audit RO des onze requêtes terminé6.55s sur15007fichiers publics épinglés
(rehashés),12133admissibles. mutex_lock ASCIIcasefold :0documents du premier300,
64admissibles dans l’ensemble,75fichiers réguliers au total. On ne peut donc
pas attribuer ce manque à une absence dans Linux2.6.0 entier. Mutex :0dans300,
442admissibles ; mutex lock relax repérage ASCII normalisé0dans300/79admissibles.
Autres modes : témoins repérés mais locateurs incomplets, pasoracle Lucivy
(typos floues localisées via schedule/register, frontièresASCII/regexlocales).
Aucun fichier permuté, aucun test/Cargo rejoué, aucune preuve7.2 acquise.
Suite conceptionRO : composer les oracles de tests existants ground_truth
plutôt que dupliquer le moteur, admission Source.dataset/PANEL avant toute
construction ; cas pure/mock à proposer, nonjoués/noncodés produit.

[ConceptionRO10](../16-11-2026/10-contrat-du-panel-et-admission-du-corpus.md) :
objectifRAM/Files inchangé, admission Source.dataset/PANEL fondée sur QueryConfig
et oraclespursdéjàprésentsground_truth, avanttoutbuilder. Rougepure/mock décrit
maisNONcodé/NONjoué : count50suffisant accepteactuellement une vue sanstémoin,
doitrefuser sansconstruction ; ensuitecasfrontières/Unicode/hashes/privé.
AuditRO public conservé SHA518f47c410fb4b9479268e6f5d5390202cdd1e1d2f247e9d3f65fbc4d6e329ec,
outilb7886900 ; localisation11queriespasqualificationmoteur. Action deRO portée
parRecherche : vérifiercontrat/modes/oracles/conception10, lecture seule sans
Cargo/test/ELF/base/services ; retour relié au même ticket avantcode/RUN.
Pasdesélectionposthoc,2.6présenceailleurs≠admission/7.2toujoursabsent.

RORecherche sur3b48e94ddc favorablebornée, sansrun. Correction de conception
AVANTcode : les enveloppes des oracles existants ne sont pas toutes pures
(repo_path/lecturesFS/continue ouUTF8avecpertes). Extraire leurs cœurs sur
octetsadmisépinglés ; touteerreurlecture/hash/décodagefatale avantadmission,
pasaucunhitdéduitd’uneerreur ni réouvertureambiguë. Futurmocklectureéchouée
àprévoir, NONcodé/NONjoué. Document10précisé ; aucune modification du core.

ComplémentRORecherche portéavantcode : strictpréserveséparateurs maisfoldUnicode/
backmapoctets ; '_'séparateur,termanchor+exact,prefixanchorseul. Oraclefuzzy
réutilisefuzzy_spansmoteur : distinctcheminindex, pasindépendancealgorithmique.
RustRegexUnicode/spansnonvides≠PythonASCII. Rougefuture50sanstémoin/mockbuilder0
surpointENTRÉECONSOMMATEURadmission, pashelperisolé/APIabsente/modificationglobale
usablequantité. RefNONvideaprès3builds préservée. ToutresteNONécrit/NONjoué.
