# Oracle final : préparation hors ligne du 16 novembre

Ticket : `docs/banc/tickets/2026-11-16-oracle-des-copies-finales-des-trois-bases.md`.
Aucun calcul joué par cette préparation. Attendre FINI G puis le créneau Arbre.
G seul possède les services. Le banc ne touche ni aux bases vivantes ni au produit.

## Entrées nécessaires par base

Arbre fournit les chemins des copies finales A, référence avant B et index neuf B′,
leurs manifestes et la source figée utilisée après REGLES7. Cette source doit
inclure les fichiers non suivis et les modifications locales : HEAD seul ne suffit
pas. Conserver l'inventaire des chemins inclus avec empreintes, les versions des
sous-modules, les exclusions, le manifeste et les règles effectives. Exclure les
secrets de toute copie ; ne pas copier aveuglément l'arbre de Lucie.

Avant ouverture, fabriquer une copie reflink neuve de chaque artefact de référence
vers un dossier neuf ; adapter uniquement les déclarations/gabarits copiés. Tous
leurs chemins de base doivent viser les copies privées, jamais les originaux de
service. Les chemins exacts restent à fournir : aucune déclaration n'est dite
prête à exécuter tant que cette vérification ne peut être faite.

## Provenance et outils disponibles

Source de la sonde historique et du jeu :
`3d422fad1145597b4b4bf3f2425c115b14ce3d23`, branche banc-parite-search-code.
Copies locales immuables préparées dans
`~/.cache/rag3db-banc-notes/oracle-final-16nov/tools/` :

- sonde_parite_search_code.rs SHA256
  `0bb347ec7db4ccd3884412e1ed827e85b94e9f62f39ce8e361b0ee64d22781b5` ;
- jeu-recherche-2a2.json SHA256
  `3d2fc0faa954bf41829b8abefaa8634f2906c39bb5323802c5693cbf658fa14d` ;
- parite_search_code.py SHA256
  `1f3be75a92b907e2458075d2f01d82f8a3d7f2ff9cb1bd8d140d01e4d0dae0de`.

À luciepc, outils existants relus par empreinte : vec-cmp.py
`59c0ad22e0457edfa37776652313e6544477ecfb9516d4d2042fa92ad5e180eb`,
et compaction/j_relevance.py
`66823e862434f574db9861668c8651f5cba8732700a3bc012c3abfd1685aac9a`.
Ils sont disponibles, pas nouvellement qualifiés sur les copies finales.
Le worktree rag3db-banc-vue est encore fe5d81f811 : il ne prouve pas le binaire927.
Ne pas exécuter les anciens lanceurs : ancienne lib373 épinglée, chemins fixes,
checkout forcé et suppressions ; leur logique est une référence de lecture.

## Recette après disponibilité

1. Prouver le lecteur : SHA exact des sources et binaires, BUILD_ID réellement lu,
   lib59ac complète annoncée par G, lucivy4.3.9 effective, sous-modules présents.
   Adapter la sonde seulement dans un worktree privé sur le code courant ; ne pas
   fusionner la branche historique entière. Compilation à qualifier au créneau.
2. Prouver chaque copie : forme, génération/rang visibles, manifeste et règles,
   modèle dense/creux configuré, colonnes/dimensions et vecteurs réellement écrits,
   dette0. Une absence de modèle ou un nombre de vecteurs ne suffit pas à prouver
   le modèle effectivement utilisé : vérifier journaux d'embarquement et identité
   du service après FINI G. Ne pas embarquer silencieusement pendant un relevé.
3. Sur A et B′, jouer séparément bm25, vector, puis tous signaux, même jeu de30cas,
   même k et mêmes filtres. `RAG3WEAVER_PARITE_NO_MIGRATION=1` ; exiger bm25 ou
   vector dans chaque régime, les deux en combiné via
   `RAG3WEAVER_PARITE_REQUIRE_SIGNALS`. Conserver les contributions par résultat.
   Avant/après B contre A est une évolution de réponses si le découpage change,
   pas une exigence d'identité de toutes les réponses.
4. Comparer identités de réponses/contenus, classement, rappel et premier rang.
   Le comparateur historique `reponses` est strict au bit : conserver son rouge,
   sans le transformer en vert par seuil improvisé. Mesurer séparément les
   vecteurs appariés par contenu : cosinus >0,99999 et maxabs <1e-4, bornes fondées
   sur les anciennes observations et à vérifier à nouveau, sans couvrir les
   contenus manquants ni les changements de rang. Pour le BM25 brut, la borne
   relative5e-3 concerne le départage lucivy sur mêmes contenus/ids visibles ;
   elle ne s'applique pas au score RRF de search_code, même en régime bm25.
5. Pertinence : union des réponses A/B′, bonnes réponses sourcées dans la source
   figée, étiquettes aveugles séparées de la parité. Conserver le juge et sa
   provenance ; pas de verdict sémantique nouveau sans jugement réellement joué.
   Pas de reprise N/F cloud implicite. Nommer séparément pertes nouvelles,
   contenus non comparables, ex æquo et bruit numérique.

## Créneau et limites

Prévoir après dette0 environ 10–20 min pour provenance et relevés si le lecteur
est déjà bâti ; un index neuf complet coûtait ~13 min au dernier banc, réserve
prudente20–30 min si B′ n'est pas fourni. Ces durées sont des estimations, pas
un délai autorisé ni une promesse. Qualification du lecteur et pertinence à
ajouter selon cache/volume ; annoncer avant acquisition. Une portée commune
32G avec preuves neuves et un budget global ; aucun verrou extérieur doublé,
aucun ancien GO réactivé. Les copies de parité sont conservées.

## Captures privées réalisées avant fermeture

Racine `~/.cache/rag3db-banc-notes/source-freeze-16nov/`, aux postes indiqués.
Chaque dossier contient `source/`, `inventory.json`, `inventory.sha256`,
`declaration-offline.json` et `declaration-proof.json`. La déclaration copiée
pointe sur sa source privée, a `database: null` et n'est pas exécutable. Les
modèles LLM y sont retirés ; aucun chemin de base vivante n'est fourni au banc.

| Poste / dossier | Fichiers | SHA256 de inventory.json |
| --- | ---: | --- |
| portable / portable-02 | 9074 | aecbedb04fb69cfcfe364268645ef339904668548daf626743d8fe44f1bb9401 |
| luciepc / pc-principal-02 | 6949 | 5911ae8f24a660c2c525095357571422c0601bc4dead535ce22e5a1aa24ae232 |
| luciepc / pc-lourd-02 | 7965 | 8f87e531ee2fadd37290437227d9ddeddeb59aa73db37d5b346495712cd9d512 |

Les bytes du working tree sont copiés, y compris non suivis non ignorés et
sous-dépôts initialisés. fuzzy-fst est présent dans pc-principal. Les HEAD de
codeparsers capturés sont respectivement c660893dadf401c050b2ddbee84765615f917d3b,
0c72f6812734306d2af8364f8b5c8798f7f3f614 et
353d2c439d57c7967364b4892e9051dc8fa979b3 : ce sont les versions du **code source
indexé**, pas la version du parseur embarqué dans le lecteur927. Ne pas les
confondre ni modifier les arbres pour les aligner.

Contrôle immédiatement après capture : tous les hashes source et copie sont
identiques ; HEAD et empreintes de git status inchangés pour chaque sous-dépôt.
La capture est STABLE_CANDIDATE, **pas corpus oracle qualifié**. Le script
`verify.py` conservé aux deux postes permet le contrôle après repassage :
`python3 <racine>/verify.py <racine>/<dossier>`. Il rend les chemins changés sans
lire `.env` ni `.vault` ; il ne certifie pas l'équivalence du scanner.

Limites à lever avant matched : admission par `.ignore`, exclusions Git/globales,
liens internes admis par WorkingTree mais non suivis par cette capture,
exclusion1MiB et lignes minifiées. L'inventaire indique les exclusions et ne
contient aucun fichier `.env*`, `.vault` ou administration `.git`. Aucun chmod
ni checkout n'a été fait dans les arbres sources. Les premières captures01,
incomplètes à cause d'un lien récursif, sont conservées ; la capture locale01
encore active a été arrêtée par son PID vérifié. Aucun service ni GPU touché.

### Diagnostic d'admission pendant la fenêtre G (sans Cargo ni hôte)

`admission-diagnostic.json` est conservé dans chaque freeze02. La méthode est
un substitut de diagnostic (`rg --files --hidden`, exclusions de dossiers,
secrets et plafond1MiB), **pas une exécution du scanner927**. Aucun contenu
commun ne diffère. Les absences sont uniquement des fichiers d'administration
`.git`, volontairement non copiés : 1 portable, 2 PCprincipal, 3 PClourd.
Les extras démontrent pourquoi git ls-files n'est pas l'oracle d'admission :

- portable : quatre archives `.tar.gz` de preuves sous
  `docs/coeur-cpp/12-11-2026/preuves/` ;
- PCprincipal : aucun extra dans ce diagnostic ;
- PClourd : `extension/lucivy/ld-lucivy/docker-compose.test.yml`.

Le seul lien trouvé dans les exclusions de chaque inventaire est
`tools/rust_api/rag3db-src`, répertoire interne, que WorkingTree ne parcourt pas.
Aucun lien-fichier interne dans ces inventaires. Les freezes02 restent intacts.
Les détails et SHA des diagnostics sont conservés aux postes concernés ; la
qualification finale exige le listing effectif de WorkingTree927 et l'admission
par le manifeste (dont minified_line_max_bytes), pas une égalité supposée avec rg.

### Outil autonome exact, exception RO autorisée par C7

Source publiée : `outils/source_admission.rs`, SHA256
`34dadd5c874bc319b03a622f7e0215c6835aa5d0c1811af543566f9d800652a1`.
Seul appel métier : `WorkingTree::new(root).list_with_exclusions()` ; aucune
référence à Catalog, PreparedBackend, connexion, ingestion ni lecture du contenu
des fichiers source. Le scanner lit ses règles d'exclusion habituelles. Sortie
TSV, chemins/raisons UTF-8 codés en hexadécimal ; compte final explicite, code
non nul si erreur. Le listing brut est distinct de l'admission du manifeste.

Préconditions bloquantes avant compilation : G fournit une rlib927 existante,
son dossier deps, SOURCE/build flags, compilateur et paramètres de lien59ac.
Vérifier ces provenances, hacher rlib, outils et sources, relever la capacité
fraîche puis acquérir le vrai poste lourd via la portée commune32G. Aucun
rebuild de G pour cet outil, aucun Cargo, aucune mesure exclusive. Une rlib
absente ou incompatible arrête cette voie ; proposer une alternative à C7.

Recette autorisée, à instancier seulement avec ces artefacts attestés : portée
commune `--mode lourd --delai 120`, preuves neuves, compile **une fois** par
`rustc --edition=2021 source_admission.rs --extern rag3weaver=<rlib927>
-L dependency=<deps927> <paramètres de lien59ac attestés> -o <binaire privé>`.
Pas de parallélisme Cargo ; un seul rustc, budget global de120s après acquisition,
arrêt/cleanup inclus. Le même budget couvre les listings si leur coût tient,
sinon arrêter et demander un créneau distinct, sans remettre le compteur à zéro.
Lancer ensuite séquentiellement le binaire sur les trois roots et leurs captures,
sorties privées hors target, jamais sur un fichier de base. Hacher le binaire et
conserver argv, codes, sortie, cgroup/plafond, verdict et quiescence. Ne pas
publier le jeton d'autorité privé. Pas de qualification de passe Rust produit.

À cette publication, **source non compilée/non jouée**, rlib927 attendue. L'accord
porte uniquement sur cet outil RO borné ; il ne réactive ni passe générale,
ancien GO, migration de base vivante ou nouveau service.

### Première entrée exacte : refus compilateur, arrêt conservé

Rlib927 vérifiée au SHA annoncé par G :
`aaad2d69589ffc3aea073d534ed1aa37da87431725456d2c5d2a71ea225305d5`.
Première compilation sous vrai poste lourd32G, portée120s : E0514, compilateur
trouvé1.98.0 contre rlib compilée1.98.1 (48a229cea, 2026-09-01). Aucun listing
n'a joué. Arrêt sans réessai, sans rebuild/Cargo/nettoyage. Verdict commun ROUGE,
code1, restes[], privé nettoyé. Preuves luciepc :
`~/.cache/rag3db-banc-notes/oracle-final-16nov/validations-scanner-01/validation-7cwclvjf/` ;
argv et compile.log sous `oracle-final-16nov/scanner-01/`.
Alternative soumise à C7/G : retrouver le compilateur1.98.1 exact existant et
attester sa provenance ; nouvelle recette/autorisation avant nouvelle entrée.
Les captures02 et la rlib restent préservées. Aucun oracle matched annoncé.

### Recette corrective publiée avant seconde entrée distincte

Compilateur **déjà installé**, sans installation ni réseau :
`~/.rustup/toolchains/1.98.1-x86_64-unknown-linux-gnu/bin/rustc`, SHA256
`859254978c0a0402c32f949f6de0d99aee73be8d15f45aac00ae1448aac51e74`.
`-Vv` réel : rustc1.98.1, commit
`48a229ceaefd4985c50990b14116b6d856af0985`, date2026-09-01,
host x86_64-unknown-linux-gnu, LLVM22.1.8. Correspond à E0514 de la rlib927,
dont le SHA aaad2d69589ffc3aea073d534ed1aa37da87431725456d2c5d2a71ea225305d5
est vérifié à nouveau. Source outil34dadd inchangée.

Nouvelle entrée autorisée distinctement par C7 : dossier `scanner-02/`, nouvelles
preuves `validations-scanner-02/`, même plafond32G/délai120 après vrai poste lourd,
un seul rustc `codegen-units=1`, sans Cargo/DB/réseau. Seule correction : chemin
absolu du compilateur et contrôle de son SHA/-Vv. Premier rouge conservé intact.
Le lanceur conserve argv, version, code et binaire hors scratch ; prochaine erreur
arrête cette entrée. Capacité fraîche observée avant entrée : ~52Gio disponibles.

### Seconde entrée : scanner exact PC joué, limites conservées

Verdict VERT, code0, restes[], espace privé nettoyé :
`oracle-final-16nov/validations-scanner-02/validation-i_eprqya/` à luciepc.
Binaire SHA256 `d6cbd33b7e0ee53ae0aac3dc22b023c74aa3801ff2f69f020f79189ed79e917d`,
lié depuis rlib927 par le compilateur1.98.1 attesté. `ldd` ne montre aucune
librag3db.so, ni dépendance manquante ; aucune base ni service ouvert.

| Listing exact WorkingTree927 | Original | Capture02 | Absents >1MiB | Métadonnées .git absentes | .env.example absent volontairement | Autres absents | Extras |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| PCprincipal | 7018 | 6949 | 66 | 2 | 1 | 0 | 0 |
| PClourd | 8049 | 7965 | 81 | 3 | 1 | 0 | 1 |

Les66/81 fichiers sont classés par stat seulement, sans lecture de leur contenu.
Le scanner admet aussi un `.env.example` : la capture a volontairement exclu tous
les `.env*`, sans les lire. L'extra PClourd reste
`extension/lucivy/ld-lucivy/docker-compose.test.yml`. Le contrôle des hashes de
source/capture et des HEAD/status passe avant et après chaque paire de listings.

Sous `scanner-02/` : sorties TSV originales et snapshots, compiler-Vv.txt,
compile-argv.json, compile.log/code, binary.sha256, comparison.json et
classification.json. SHA256 comparaison :
`8a88fd9206c5971c05dbdd8ca29df4ec54d1ad0bb9dbe8f00503d3eba286742e`.
Le listing **ne vaut pas ingestion** : plafond1MiB/minified du manifeste,
métadonnées, `.env.example` et extra doivent être départagés avant oracle matched.
Portable : listing exact encore NON JOUÉ ; réutilisation du même binaire autonome
proposée sous une nouvelle portée RO60s32G, sans compilation ni Cargo.

### Recette de portabilité avant troisième entrée RO

C7 exige un smoke sur dossier jetable avant listing portable, une simple copie
ELF ne prouvant pas sa compatibilité ISA. Même binaire d6cbd33b..., transféré
sans rebuild. Portée privée lourde32G, délai global60s après acquisition, aucun
Cargo : vérifier SHA, lister uniquement un dossier jetable contenant sample.rs,
vérifier sa présence dans le rendu et son code0 ; seulement ensuite vérifier
freeze02, lister root original et snapshot portable, revérifier hashes/états.
Conserver les codes/sorties même en refus ISA ; première erreur arrête l'entrée.
Aucun accès base/service. Dossier de preuves neuf `validations-scanner-03/`,
artefacts `oracle-final-16nov/scanner-03/`. Slot non conservé pour publication.

### Troisième entrée : portable joué après smoke ISA

Scope RO60s32G VERT, code0, preuves locales
`oracle-final-16nov/validations-scanner-03/validation-nntdkoeg/`. Même binaire
SHA d6cbd33b..., fixture privée sample.rs effectivement listée/code0 avant
l'ouverture de la racine source. Aucun Cargo, base, ingestion ou service.

WorkingTree927 liste9138 chemins originaux contre9074 dans portable-02.
Absents : 66 fichiers >1MiB (stat seulement), un `.git` d'administration et
un `.env.example` volontairement NON LU. Aucun autre absent. Quatre archives
`.tar.gz` de preuves apparaissent en plus dans la capture hors contexte Git,
comme au diagnostic. Hashes source/copie et HEAD/status inchangés avant/après.
Comparaison complète locale : `scanner-03/comparison.json`, SHA256
`f9235f13501022e5a2cdcda9f336e1cc051ccc89272b72ad1d0e03d4bfd5db20`.
Le poste est rendu avant publication. Les trois listings exacts sont désormais
joués ; le premier refus E0514 reste conservé séparément. L'oracle reste
NONMATCHED tant que manifeste/type d'admission de `.env.example` et extras ne
sont pas prouvés comparables. Ne jamais lire/copier `.env*` pour forcer le match.

### Huit contrôles hermétiques proposés pour la porte d'ingestion

Les trois déclarations privées relues portent working_tree/index code,
minified_line_max_bytes5000/code_file_max_bytes1048576, generated absent : la
politique par défaut de GeneratedPolicy est donc pertinente. Source publiée
`outils/admission_contract.rs` : huit assertions sur les primitives927, données
synthétiques uniquement, jamais de contenu `.env*`. Cas : métadonnée.git, archive
gz, yaml potentiellement admis, `.env.example` non refusé par le chemin seul,
code>1MiB, ligne de code>5000, prose longue non minifiée et tête binaire.
Recette : même lien rustc1.98.1/rlib927 attestés que scanner02, scope lourd32G120s
neuf ; une compilation et une exécution, première erreur arrêt. Aucun Cargo,
lecture source réelle, catalogue ou base. Accord C7 limité aux huit CPU
hermétiques ; ces cas ne qualifient pas le contenu inconnu de `.env.example`.

### Huit contrôles joués : ingestion et limite restante

VERT/code0/restes[]/privé nettoyé, portée120s32G libérée :
`oracle-final-16nov/validations-admission-01/validation-s5xl2ueo/` à luciepc.
Les huit assertions sur les primitives927 passent ; sorties et argv conservés
sous `oracle-final-16nov/admission-01/`. Aucun contenu réel ni `.env*` lu.

- `.git` sans extension et `.gz` sont refusés par admission sur chemin seul :
  les quatre archives supplémentaires du portable ne sont pas ingérées.
- Le plafond1MiB est un seuil de code ; la prose a déjà128KiB au verdict. Les
  fichiers manquants >1MiB ne sont donc pas une dette de copie de code ingérable
  avec ces déclarations. Les motifs de refus peuvent précéder ce seuil.
- Un YAML peut passer : l'extra docker-compose.test.yml de PClourd ne peut pas
  être effacé du raisonnement par son suffixe. Une source privée future peut
  utiliser une règle `.ignore` sur ce chemin exact, à rejuger par le scanner927 ;
  jamais modifier freeze02 ni l'arbre de Lucie.
- `.env.example` n'est pas refusé sur chemin seul. Son contenu reste inconnu et
  NON LU. Avant toute qualification globale, vérifier dans les **copies finales**
  les seules métadonnées File/Scope de ce chemin. Une absence d'ingestion
  effectivement prouvée peut lever ce gap ; une présence maintient NONMATCHED,
  sauf comparaison explicitement limitée à un périmètre commun déclaré et prouvé.

La qualification des trois bases se fait individuellement : racines, manifestes,
source figée et modèle/colonnes/dette0 propres à chacune. Ni le listing exact,
ni les huit cas ne sont un oracle de pertinence ou une preuve de migration.
