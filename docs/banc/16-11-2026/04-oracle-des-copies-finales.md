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
