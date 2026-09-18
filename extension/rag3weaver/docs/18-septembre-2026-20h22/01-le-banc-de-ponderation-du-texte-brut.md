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

## 8. Ce que ça coûte

Un fichier de test, `tests/e2e_banc_texte_brut.rs`, sur rag3db natif sans
GPU (`HashEmbedder` pour le montage, ou le démon 278m pour des chiffres qui
vaillent) : deux bancs, un tableau, pas de seuil qui casse — un banc mesure,
il n'échoue pas. Une passe, et le pas C a ses chiffres.
