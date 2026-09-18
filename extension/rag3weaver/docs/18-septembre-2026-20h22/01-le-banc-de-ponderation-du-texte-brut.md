# Le banc de pondération du texte brut

**18 septembre 2026.** Conception, sans code. Une dette nommée le 30 août
(`docs/30-aout-2026-06h00/06-ou-on-en-est.md`, §3) : 2 326 passages
`texte_brut` sont entrés dans l'index ce jour-là, et personne n'a mesuré s'ils
font reculer les vraies réponses.

## 1. Ce qu'on craint, et pourquoi c'est plausible

Un passage `texte_brut` est du **texte libre** — un `README.md`, un `.toml`,
un `.sh`. Riche en mots, en phrases, en prose. Et nos requêtes sont en prose.
BM25 comme le vecteur l'avantagent : il *ressemble* à une question bien plus
qu'un `fn` de vingt lignes.

Le doc en aval du 30 août (`02-ce-que-ca-change-en-aval.md`, §4) le disait :
*« c'est le risque le plus concret de tout le chantier »*, et prescrivait la
mesure **avant** toute pondération — *« sans ce banc, on réglera un poids à
l'aveugle »*. Le banc n'a pas été fait. Il faut le faire avant le pas C du
repli des KB, qui posera la forme de la fusion par entité : c'est lui qui dira
si un facteur sert.

## 2. Ce que les bancs existants mesurent — et ce qu'ils ne mesurent pas

| banc | ce qu'il mesure | ce qu'il ne mesure pas |
|---|---|---|
| `e2e_catalogue_gabarits::brique_2` | trois questions françaises sur les **gabarits** (`product`, `user`, `conversation`), par signal, via `Catalog::search` | le code — l'entité est `Gabarit`, pas `Scope` ; aucun `texte_brut` n'y vit |
| `e2e_banc_qualite` | 45 questions sur **notre propre code**, MRR / R@1 / R@5, cinq modèles | **le cosinus nu** : il embarque des scopes extraits à l'aiguille et compare des vecteurs — ni catalogue, ni BM25, ni fusion, ni `texte_brut` |

Le doc du 30 août citait `brique_2` parce que le banc de qualité n'existait
pas encore. Mais ni l'un ni l'autre ne répond à la question : **la recherche
du catalogue, sur l'index du code, avec et sans texte brut, rend-elle encore
les mêmes fonctions en tête ?**

Le banc à écrire est donc **nouveau**, et il emprunte à chacun ce qui vaut :
les 45 questions du banc de qualité — elles visent des fonctions réelles de
`src/`, avec leur réponse attendue — et le chemin de `brique_2` — `Catalog::search`,
un signal à la fois, puis les deux.

## 3. Banc A — la recherche ne doit pas reculer

### 3.1 Le montage

1. Ingérer `extension/rag3weaver/src/` **par le chemin réel** (`analyze` →
   `ingest_code`), pas à l'aiguille : c'est ce qui fait entrer les
   `texte_brut` — les `docs/` ne sont pas dans `src/`, mais `src/` contient ce
   qu'il contient de non-Rust, et on ajoute **`docs/30-aout-2026-06h00/`**
   à la racine ingérée pour avoir de la prose française à côté du code.
2. Les 45 questions de `e2e_banc_qualite::QUESTIONS`, avec leurs fonctions
   attendues, importées telles quelles.
3. Deux configurations, **le même index** :
   - `sans` : `filter = scope_type != 'texte_brut'` — la recherche d'avant le
     30 août, reconstituée par un filtre ;
   - `avec` : pas de filtre — la recherche d'aujourd'hui.
4. Trois signaux : `VECTOR`, `BM25`, `HYBRID`.

Le filtre existe et c'est ce qui rend le banc bon marché : un seul index,
deux lectures. `scope_type` déclare ses valeurs depuis le 30 août
(`4ca187575`), donc le filtre se nomme sans deviner.

### 3.2 Ce qu'on lit

Par signal, pour `sans` et `avec` : **MRR, R@1, R@5** sur les 45 questions —
les mêmes mesures que le banc de qualité, pour que les chiffres se comparent.

Et une quatrième colonne, celle qui répond directement à la crainte :
**combien de fois un `texte_brut` sort devant la fonction attendue** (rang du
premier `texte_brut` < rang de la bonne réponse). C'est le nombre qui dit
« le bruit est réel » ou « il ne l'est pas », avant même de regarder le MRR.

### 3.3 Ce qui dirait que ça va, et ce qui dirait que non

| lecture | ce que ça veut dire |
|---|---|
| `avec` ≥ `sans` sur les trois mesures | le texte brut ne coûte rien — **rien à pondérer**, le doc 02 avait prévu ce cas (« rien du tout, et on regarde ») |
| MRR recule de moins de 0,02 | acceptable, à noter, pas à corriger |
| MRR recule de plus de 0,05, ou R@1 de plus de 3 questions sur 45 | le bruit est réel : **une pondération se justifie**, et le banc dit de combien |
| le recul est sur `BM25` mais pas sur `VECTOR` | c'est le lexical qui aime la prose — la correction est un poids par genre à la fusion, pas un rerank |
| le recul est sur `VECTOR` aussi | le rerank seul ne suffira pas : le cross-encoder verra les mêmes candidats |

Les seuils 0,02 / 0,05 sont des ordres de grandeur posés d'avance pour ne pas
les inventer après coup — la règle du doc 03 du 30 août.

## 4. Banc B — le gain existe, pas seulement l'absence de perte

Un banc qui ne mesure que la perte laisse la moitié de la question ouverte :
si le texte brut ne coûte rien mais n'apporte rien, il occupe de la place pour
rien.

**Cinq questions dont la réponse est dans un passage `texte_brut`**, et
nulle part dans le code — des questions qu'on pose vraiment à un dépôt :

| question | où est la réponse |
|---|---|
| « pourquoi les grammaires tree-sitter sont figées en 0.23 » | `codeparsers/Cargo.toml`, le commentaire — un `.toml`, donc `texte_brut` |
| « comment lancer les tests e2e avec une autre racine » | `run_e2e.sh`, en tête — un `.sh` |
| « quelle est la règle pour les identifiants et les commentaires » | le `README` ou un doc de décision |
| « qu'est-ce que le régime confort promet » | `docs/…/06-le-regime-confort-et-sa-moitie-manquante.md` |
| « quel est le seuil de documents pour commencer en 107m » | `docs/7-septembre-2026-22h51/01-…md` |

Mesure : **R@1 et R@5 en `avec`** — en `sans`, la réponse est 0 par
construction, et c'est le point : ces questions n'avaient *aucune* réponse
avant le 30 août. Cinq sur cinq à 5 est ce qu'on attend ; moins de trois dit
que le texte brut est indexé mais mal servi — découpage, ou rendu.

## 5. La contrainte de découpage, mesurée en passant

Le texte brut est découpé **comme du code** : `ChunkStrategy::Semantic`,
1000/100, parce que le découpage se déclare par entité et que `texte_brut`
est un `Scope` (trouvé le 30 août, `06-ou-on-en-est.md` §5). Un
`ChunkStrategy::Markdown` existe et respecte les titres.

Le banc B le mesure sans le trancher : si les cinq réponses sortent avec des
chunks coupés au milieu d'une section — le rendu le montre —, c'est le
découpage qui limite, pas la recherche. On le note ; la forme (une entité de
plus, un découpage par enregistrement) n'est pas à ce banc.

## 6. Ce que le banc alimente — et ce qu'il ne tranche pas

Le pas C du repli des KB posera **la fusion par entité** : `FuseResultsNode`
pèse aujourd'hui par *signal* (`weights: HashMap<String, f64>`,
`generic_search_nodes.rs:1020`) ; il pèsera par entité, dérivées comprises,
avec des poids portés par la config. C'est là que « un facteur par genre »
prendra sa forme, et ce n'est pas ce doc qui la choisit.

Ce que ce banc lui donne : **de combien** et **sur quel signal** le texte brut
pèse. Si le recul est de 0,08 de MRR sur `BM25` seul, le pas C sait quoi
corriger et de combien ; s'il est nul, le pas C n'a rien à faire pour ce
genre, et c'est aussi une information.

**Les deux bancs se rejouent tels quels après le pas C** : mêmes questions,
même index, mêmes mesures. Rien dans leur montage ne dépend de la forme de la
fusion — c'est la condition pour qu'ils disent, après, si le facteur a servi.

## 7. Ce qui reste hors de ce doc

- La forme de la fusion par genre — pas C, session architecture.
- Le rerank comme alternative : le banc dira s'il *peut* suffire (recul sur
  `BM25` seul) ; l'essayer est un autre chantier.
- Le découpage du texte brut en `Markdown` — mesuré en passant (§5), pas
  décidé.

## 8. Les chiffres

### 18 septembre 2026 — montage à vide, `HashEmbedder`

`tests/e2e_banc_texte_brut.rs` sur `lecteurs-csv`, rag3db natif, sans carte.
4 825 scopes, 0 en échec, ingestion 17,5 s.

**`src/` seul fait entrer zéro scope `texte_brut`** : 100 fichiers, tous `.rs`.
Les 6 texte_brut de l'index sont ceux que le banc ajoute (le dossier de prose
du 30 août, `codeparsers/Cargo.toml`, `run_e2e.sh`). Sur ce crate, la crainte
du §1 n'a donc pas de corpus pour se réaliser — elle en aura un sur un dépôt
qui a des docs à côté de son code, ce qui est le cas de rag3db entier.

| signal | lecture | MRR | R@1 | R@5 | texte brut devant |
|---|---|---:|---:|---:|---:|
| plein texte | sans | 0,209 | 4 | 17 | 0 |
| plein texte | avec | 0,220 | 5 | 17 | 0 |
| vecteur, hybride | — | 0 | 0 | 0 | 0 |

Banc B, plein texte : `sans` 0/5 (par construction), `avec` 2/5 à 5.

**Ce que ça dit** : rien encore sur la pondération — six textes ne font pas un
bruit, et le HashEmbedder rend des vecteurs sans sens, donc vecteur et hybride
sont à zéro par construction. Ce que ça prouve : le montage tient, le filtre
`sans` marche, et les deux bancs rendent leurs tableaux.

**Une observation, et sa cause corrigée** : en hybride, le top-3 est
*exactement* celui du vecteur seul sur toutes les questions ratées. J'ai
d'abord écrit que l'échelle des scores l'expliquait — c'était faux : la fusion
par défaut est **RRF, par rang, k = 60** (session architecture, 18 septembre).
Ce qui fait dominer le vecteur, ce sont **les poids par signal** — BM25 à 0,3
contre le vecteur — et c'est précisément ce que le pas C rend réglable par
entité. La passe 278m dira si ce réglage par défaut est mauvais pour le code.

**Le banc B tient sur ce que le banc ajoute**, pas sur `src/` : sans le
dossier de prose, `Cargo.toml` et `run_e2e.sh` ajoutés à la main, il n'aurait
aucune réponse à trouver. C'est voulu — sur rag3db entier, les docs sont à
côté du code et ce banc les représente.

### 18 septembre 2026 — `granite-278m`

Même montage, `RAG3WEAVER_BANC_MODELE=granite-278m`, carte TV. 57 s, 4 825
scopes, 0 en échec, ingestion 41,8 s.

| signal | lecture | MRR | R@1 | R@5 | texte brut devant |
|---|---|---:|---:|---:|---:|
| vecteur | sans | 0,346 | 10 | 24 | 0 |
| vecteur | avec | 0,346 | 10 | 24 | 0 |
| plein texte | sans | 0,209 | 4 | 17 | 0 |
| plein texte | avec | 0,220 | 5 | 17 | 0 |
| hybride | sans | 0,356 | 11 | 22 | 0 |
| hybride | avec | 0,356 | 11 | 22 | 0 |

Banc B (`avec`) : vecteur 3/5, hybride 3/5, plein texte 2/5 ; `sans` 0/5.

**Banc A : zéro recul, zéro texte brut devant une fonction attendue** — sur six
textes. Le montage tient ; la mesure sur ce crate seul ne prouve pas grand-chose
(§8, montage à vide : `src/` n'apporte aucun texte brut). Elle prouvera sur un
dépôt qui a ses docs à côté de son code — rag3db entier.

**Banc B : une erreur du banc, pas de la recherche.** Deux des cinq questions
visent les docs 01 et 03 du 30 août, qui ont déménagé dans
`codeparsers/docs/` le jour même : elles ne sont pas dans le corpus, donc
introuvables par construction. Sur les trois qui ont une réponse : **3/3 en
vecteur et en hybride**, 2/3 en plein texte. Corrigé dans le banc (le corpus
ajoute `codeparsers/docs/30-aout-2026-06h00/`) ; à rejouer.

**Deux données pour le pas C — des données, pas des conclusions :**

1. **Hybride ≤ vecteur à 5** : 22 contre 24 — **mesuré aux poids BM25 0,3 /
   vecteur 0,7**, les défauts du moteur. La session recherche a trouvé le même
   jour que ses appelants migrés fusionnaient à ces défauts là où le gabarit
   `search_base` dit 0,6 / 0,4 ; après son correctif, une fusion déclarée sur
   l'entité prime, sinon le gabarit fait foi. Ce chiffre est donc celui d'un
   réglage qui n'est plus le défaut : **l'hybride est à rejouer après la fusion
   de sa branche**, et le poids ira à côté du chiffre.
2. **Ce qui pollue le haut des listes n'est pas le texte brut, c'est le code
   lui-même** : `tests (namespace)`, `file_scope_NN (module)` — les scopes de
   fichier entier — et `validate_identifier` devant `validate_id`. C'est un
   facteur par genre *dans* `Scope` (`module` / `namespace` contre `function`
   / `method`) que le banc met sur la table. Le MRR de 0,35 en vecteur, face
   aux 0,84 du cosinus nu sur les mêmes questions (`e2e_banc_qualite`), mesure
   l'écart entre ce que l'embarqueur sait et ce que la recherche en rend — et
   cet écart n'est pas dû au texte brut.

### À rejouer

Après la correction du corpus, et sur rag3db entier — c'est là que le texte
brut existe en nombre.

## 9. L'étage qui perd la moitié — trois mesures pour l'isoler

Le chiffre qui compte le plus dans la passe 278m n'est pas celui du texte
brut : **0,35 de MRR par la recherche, contre 0,84 au cosinus nu** sur les
mêmes 45 questions et le même modèle (`e2e_banc_qualite`). Ce n'est pas la
fusion — le vecteur seul fait 0,346. Quelque chose entre l'embarqueur et le
résultat perd la moitié de la qualité, et **c'est ça qu'il faut mesurer avant
toute pondération** : pondérer un signal qui perd la moitié en route, c'est
régler le volume d'un haut-parleur débranché.

### Ce que les deux montages embarquent — ce n'est pas le même texte

| | banc de qualité (cosinus nu) | la recherche du catalogue |
|---|---|---|
| unité | **un scope à l'aiguille** : la doc au-dessus, la signature, le corps, coupé à 1 500 caractères | **un chunk** par champ de contenu |
| champs | tout ensemble, dans un texte | `content`, `docstring`, `signature` découpés **séparément** (`compute_chunks`, un flux de chunks par champ) — la doc d'une fonction et son corps ne sont jamais dans le même vecteur |
| le nom | dans la signature | dans `_title`, **pas dans `_text`** — `EmbedNode` embarque `_text` seul |
| découpage | un texte par fonction | `Semantic` 1000 / 100 : un corps long fait plusieurs chunks, chacun sans son nom ni sa doc |

Une question « laisser souffler la carte graphique » cherche `souffler` : au
cosinus nu, le vecteur porte le nom, la doc qui dit « souffler », et le corps.
Dans le catalogue, le chunk de corps ne porte ni l'un ni l'autre ; le chunk de
doc porte la phrase mais pas le nom ; et la signature — la seule qui porte
`fn souffler` — est un chunk minuscule à côté.

### Les trois étages, et une mesure par étage

Chaque mesure change **une** chose et garde les autres ; l'ordre va du plus
probable au moins probable, et on s'arrête quand l'écart est expliqué.

**M1 — le texte embarqué.** Même texte des deux côtés : indexer les 45 scopes
*à l'aiguille* du banc de qualité (son `corpus()`, un scope = un texte) comme
des entités simples d'un seul champ de contenu, chunking désactivé
(`chunked = Some(false)`), et chercher par `Catalog::search`, vecteur seul.
Si le MRR remonte vers 0,84, **l'étage qui perd est le texte** — le découpage
par champ et le nom absent du vecteur. C'est le plus probable, et c'est le
seul des trois qui se corrige dans `EntityConfig` : embarquer `_title` avec
`_text`, ou découper les champs de contenu ensemble plutôt que séparément.

**M2 — la recherche exacte au lieu du HNSW.** Même index que la recherche
réelle, mais le vecteur de la requête comparé à **tous** les chunks par
cosinus (`array_cosine_similarity` en Cypher, ou en mémoire depuis
`embed_check_hashes` + les colonnes), top 20, puis la même résolution au
parent. Si le MRR remonte, **l'étage qui perd est l'index** — le rappel du
HNSW (`efs`, `M`) ou le `search_limit = (limit + offset) × 2`, soit 20 chunks
pour 10 résultats. Peu probable à 4 825 scopes, mais bon marché à écarter.

**M3 — le classement des chunks avant résolution.** Même recherche, mais on
lit les 20 chunks bruts du HNSW *avant* `resolve_vector_chunks` : le bon
parent est-il là, à quel rang, combien de ses chunks ? Si le bon chunk est
dans les 20 mais son parent n'est pas dans les 10, **l'étage qui perd est la
résolution** — plusieurs chunks d'un même parent qui se marchent dessus, un
parent gardé au score de son premier chunk plutôt que de son meilleur.

### Ce qu'on lit

Un tableau, quatre lignes — la recherche telle quelle, M1, M2, M3 — avec MRR,
R@1, R@5 sur les 45 questions, vecteur seul, granite-278m. L'étage qui fait
bondir le MRR est celui à corriger ; s'ils bougent tous un peu, la perte est
répartie et il faudra les trois.

### 18 septembre 2026 — les quatre lignes, `granite-278m`

`tests/e2e_banc_etage.rs`, vecteur seul, 45 questions, carte TV, 47 s.

| ligne | MRR | R@1 | R@5 |
|---|---:|---:|---:|
| tel quel — `Catalog::search` sur `src/` (4 819 scopes) | 0,346 | 10 | 24 |
| **M1 — même texte que le cosinus nu, un chunk par fonction (67 scopes)** | **0,833** | **31** | **45** |
| M2 — cosinus exact au lieu du HNSW, même résolution | 0,354 | 10 | 25 |
| M3 — les 20 chunks bruts du HNSW, avant résolution | 0,179 | 1 | 16 |

M3 : le bon parent est dans les 20 chunks bruts pour 31 questions sur 45.

**Ce que ça dit.** M2 ≈ tel quel : le HNSW ne perd rien — un rappel exact
ne change pas le classement. M3 < tel quel : la résolution au parent *aide*,
elle ne perd pas ; l'ordre brut des chunks est pire que le résultat rendu.
**M1 rend 0,833 — les 0,844 du cosinus nu, retrouvés à travers le
catalogue.** L'index et la résolution sont hors de cause. Ce qui reste, c'est
le texte embarqué : le découpage par champ, le nom absent du vecteur.

**Un facteur confondu, nommé.** M1 change le texte *et* la taille du corpus —
67 scopes contre 4 819. Une part de l'écart peut venir des distracteurs, pas
du texte. La mesure qui les sépare, sans toucher à `EntityConfig` :

**M1b** — les 4 819 scopes de `src/`, chacun embarqué comme *un* texte
`nom + doc + signature + corps` assemblé côté test depuis `analysis.scopes`,
dans l'entité `Fonction` du M1. Même corpus que tel quel, même texte que M1.
Si M1b tient près de 0,83, le texte explique tout ; s'il retombe vers 0,35,
c'est la taille du corpus. À faire sur go — c'est une mesure de plus que les
trois accordées.

### 19 septembre 2026 — M1b, et la conclusion renversée

Même montage, tronc à `63d4b86b5` (branche recherche fusionnée, les bancs
passent par `Catalog::rechercher`), `granite-278m`, 69 s.

| ligne | MRR | R@1 | R@5 |
|---|---:|---:|---:|
| tel quel — `src/`, 4 820 scopes | 0,328 | 9 | 24 |
| M1 — même texte que le cosinus nu, **67 scopes** | 0,833 | 31 | 45 |
| M2 — cosinus exact au lieu du HNSW | 0,338 | 9 | 25 |
| M3 — chunks bruts avant résolution | 0,206 | 3 | 15 |
| **M1b — le texte de M1 sur les 4 820 scopes** | **0,385** | **9** | **28** |

**M1b renverse la conclusion de la veille — et c'est pour ça qu'il fallait le
faire.** Le même texte que le cosinus nu, sur le vrai corpus, rend 0,385, pas
0,83. Le texte embarqué vaut **+0,06 de MRR et +4 à 5**, pas la moitié de la
qualité. **L'écart 0,84 → 0,33 est la taille du corpus** : 45 questions
contre 67 candidats triés sur le volet, ou contre 4 820 scopes dont les
`tests (namespace)`, les `file_scope_NN (module)` de fichier entier, les `new`
et `execute` par dizaines. **Le banc de qualité mesure l'embarqueur sans
distracteurs ; il n'est pas un objectif pour la recherche** — ses 0,84 sont
ceux d'un choix entre 67 candidats, pas d'un dépôt.

Le §9 d'hier disait « l'étage qui perd est le texte ». C'était la lecture d'un
tableau sans son facteur confondu ; il reste écrit au-dessus, avec sa date,
parce qu'un doc qui efface ses erreurs n'apprend rien à celui qui le relit.

**Ce que ça change pour la décision.** Embarquer le nom avec le texte, ou
découper les champs ensemble, rapporterait ~0,06 — un gain réel, qui ne
justifie pas à lui seul de toucher aux offsets, aux surlignages et au contrat
chunk → parent. **Rien n'est corrigé dans `EntityConfig`** ; c'est à Lucie,
avec ce tableau-ci sous les yeux.

Le levier qui compte est **la pollution par genre dans `Scope`** — noté au §8,
remis en tête par M1b : les scopes de fichier entier et les espaces de noms
occupent le haut des listes sans jamais être une réponse. C'est un facteur par
genre, la lignée du pas C. La mesure qui le chiffre : tel quel avec
`scope_type` filtré sur `function | method` — sur go, en édition.

Et un point mesuré, d'abord donné sans explication : tel quel est passé de
0,346 à 0,328 (R@1 10 → 9) entre les deux passes, vecteur seul, même corpus à
un scope près. **Clos par la session recherche** (`85962b263`,
`nettoyage-apres-monolithe`) : une seule question a basculé, à +0,002 de
cosinus — la variance d'un HNSW reconstruit, pas le chemin `rechercher`.

### 19 septembre 2026 — G, le genre

Même passe, une ligne de plus : tel quel avec `scope_type` filtré sur
`function | method` (`FilterCondition::Should`), rien d'autre ne change.

| ligne | MRR | R@1 | R@5 |
|---|---:|---:|---:|
| tel quel | 0,328 | 9 | 24 |
| M1b — le texte | 0,385 | 9 | 28 |
| **G — le genre** | **0,385** | **12** | **27** |

**Le filtre par genre vaut exactement le changement de texte — +0,06 de MRR —
et fait mieux à 1, sans toucher à l'embarquement, aux offsets ni au contrat
chunk → parent.** C'est du côté recherche, un filtre sur un champ déjà indexé
et déjà déclaré. Les deux leviers ne corrigent pas la même chose et sont
probablement cumulables ; ni l'un ni l'autre ne rejoint 0,84, et il ne faut
plus le chercher — le reste est le corpus.

Ce que ça suggère pour le pas C, sans le trancher : **un poids par genre dans
`Scope`** qui écrase `module` / `namespace`, plutôt qu'un filtre dur. Un
filtre perd les cas où un scope de fichier entier *est* la réponse — un `mod`
de constantes, un fichier de config ; un poids les laisse remonter quand rien
d'autre ne répond. La ligne G est là pour mesurer l'un contre l'autre le jour
où le poids existe.

Une variance à connaître : M3 est passé de 0,206 à 0,241 entre deux passes
identiques — l'ordre brut du HNSW n'est pas déterministe à cette granularité.
Les lignes résolues (tel quel, M2, G) sont stables d'une passe à l'autre.

## 10. Ce que ça coûte

Un fichier de test, `tests/e2e_banc_texte_brut.rs`, sur rag3db natif sans
GPU (`HashEmbedder` pour le montage, ou le démon 278m pour des chiffres qui
vaillent) : deux bancs, un tableau, pas de seuil qui casse — un banc mesure,
il n'échoue pas. Une passe, et le pas C a ses chiffres.
