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
