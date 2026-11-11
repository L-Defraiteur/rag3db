# 06 — La feuille de route

Décidé le 25 août 2026, contre l'idée de livrer une surface : **on ne livre pas,
il manque de quoi avaler le réel.**

> **Relu le 5 septembre 2026.** Ce qui suit a été écrit avant que le moteur
> devienne multi-backend ([15](15-le-moteur-cesse-d-etre-mono-backend.md)).
> L'ordre a bougé, plusieurs dettes sont closes, et **une était fausse**. Chaque
> point vérifié dans le code porte sa date ; ce qui n'en porte pas date du
> 25 août et n'a pas été revérifié.

## 1. Les produits, et pourquoi ils ne veulent pas la même chose

| produit | ce qui compte | ce qui ne compte pas |
|---|---|---|
| **Agent de code embarqué** (`npm install`, un binaire, MCP) | tout en processus, installation sans friction, **parole locale** | le LLM local — *le développeur a déjà Ollama ou une clé* |
| **Bibliothèque Rust** | le moteur, les traits, zéro dépendance imposée | les modèles, que l'utilisateur amène |
| **Produit cloud multi-tenant** | cloisonnement, élasticité, coût à l'échelle | la latence locale |
| **Substrat navigateur** (doc 36 §4 bis) | tout en processus, **sans choix** | — bloqué par la taille des modèles |

**L'asymétrie qui décide** : un développeur **a déjà** un LLM. Il **n'a pas** de
moteur de parole. Embarquer la parole différencie ; embarquer le LLM rattrape
péniblement ce que llama.cpp fait déjà mieux, avec de la quantification qu'on
n'a pas.

L'architecture y répond déjà sans qu'on l'ait cherché : `default = []` avec une
feature par brique, et les **poids téléchargés à la première utilisation dans
`~/.cache/rag3weaver/`, jamais empaquetés**. Un `npm install` reste petit.

## 2. L'ordre du travail

### Côté documents et code

**Ce qui reste vraiment ici**, au 5 septembre : la **lecture des documents**
(point 3) et le **graphe de normalisation xlsx** (point 4). Les points 1 et 2
sont faits ; ce sont eux qui ont le plus vieilli dans ce document.

1. ~~**`codeparsers` intégré**~~ — **fait, et sorti du dépôt** (3 septembre
   2026) : c'est aujourd'hui un sous-module git,
   `git@github.com:L-Defraiteur/codeparsers.git`, consommé par une dépendance
   par chemin optionnelle derrière la feature `code`. Ce qui suit est l'état de
   la première étape, gardé pour la trace.

   *première étape faite le 25 août au soir*
   (`25-aout-2026-18h58/03`) : crate réparé, `File` / `Scope` / `Library`
   avec `hashsafe`, `ParseCodeNode → CodeIngestNode`, notre propre
   `src/dataflow/` navigué, références attribuées au scope le plus interne
   (9 645 relations au lieu de 66 771, `25-aout-2026-18h58/04`). Reste
   l'incrémentalité de la résolution et `FileSource`. Le texte qui suit est l'état d'avant. 24 555 lignes, 78 fichiers, 12 langages, un crate
   à part **jamais référencé**. Le plus gros actif dormant du dépôt. Ce qui
   compte n'est pas le parsing mais `import_resolution`,
   `relationship_resolution` et `scope_extraction` : **ils produisent les
   arêtes**. Un fichier cesse d'être des chunks pour devenir des entités
   reliées — et c'est ce qui rend un graphe supérieur à un magasin de vecteurs
   pour du code.
   **Ce n'est pas une page blanche** : `l5/code-rag/` contenait déjà
   `codeparsersToEntities()`, `codeparsersRelationships()` et un `CODE_SCHEMA`
   (`File`, `Scope`, `Scope_Chunk` ; `DEFINED_IN`, `CONSUMES`, `PARENT_OF`).
   Avec `project` dès le premier jour.
2. ~~**`grep` et `read`**~~ — faits le 25 août au soir
   (`25-aout-2026-18h58/05`) : sur une `FileSource` (chemins virtuels,
   `WorkingTree` / `Snapshot`), annotés par le graphe (scope le plus étroit),
   péremption par hash ; puis `edit` (remplacement unique, préfixes de
   `read` tolérés, ré-ingestion du fichier) et `list`. Le modèle voit `edit`,
   `grep`, `list`, `read`, `search`, `search_expand`. Reste `GitRef` et
   retirer `special_ops`.

   **Au 5 septembre**, `search_expand` **n'existe plus** : il a été fondu dans
   `search`, qui a gagné un paramètre `relation`. La raison est mesurée et vaut
   d'être retenue — sur quarante appels d'outils de trois agents, `search_expand`
   avait été choisi **zéro fois**, alors qu'il était le seul à rendre le graphe
   de dépendances. Un agent qui hésite entre deux outils proches n'en prend
   aucun. Les onze outils offerts aujourd'hui sont `adopt`, `edit`, `grep`,
   `list`, `place`, `read`, `run`, `run_bg`, `schema`, `search`, `wait`.
3. **Lecture des documents** — pdf, docx, pptx, html, csv. Sans lib lourde : les
   formats Office sont du ZIP + XML. L'OCR livré couvre les PDF scannés.

### Côté recherche — fait le 25 août au soir

La relecture du graphe de recherche (doc 52 de `../23-aout-2026-20h33/`) a
montré que tout ce qui pèse était figé dans le code ou la config. Livré :
**signaux étiquetés**, **fusion N-aire** avec port `signals` en fan-in et poids
par nom, `BM25SearchNode(fields=…)`, **`RerankNode`** (remplace après la
fusion, module en `boost` dedans), vecteur via `SearchBackend`, `result_mode`
sur les nœuds de signal, gabarit `weighted_search.mmd`. Le « boost de titre »
est une topologie. La KB **garde l'index et perd la recette**.

Reste, dans l'ordre : **`Catalog::search` devient un gabarit** ; le titre d'une
entité simple indexé en BM25 ; `column=` sur le vecteur (seconde colonne
d'embedding à l'ingestion) ; un E2E de fusion inter-KB.

**Mesuré le 5 septembre : le monolithe n'a pas fondu, il a grossi.** 409 lignes
d'un seul corps, dans un `catalog.rs` de 6 373. Le chemin composable existe bel
et bien en parallèle — les nœuds génériques, `search.mmd`, les gabarits
`simple_*_search.mmd` — mais `Catalog::search` **ne l'emprunte pas** : elle
appelle directement les primitives. Ce sont donc toujours deux chemins à
maintenir, et c'est le prix qu'on paie depuis dix jours sans le compter. Chaque
correction de recherche de cette semaine a dû être portée aux deux, ou l'un des
deux a été oublié — c'est arrivé.

### Côté backends — **l'axe qui n'était pas dans ce document**

Il n'y était pas parce que personne ne l'avait dit à voix haute. Lucie l'a dit
depuis : *« une boucle étrange qui ne sert que kuzu ne sert pas les projets
réels »*. C'est passé devant le reste, et c'est fait pour PostgreSQL — les cinq
organes, la reprise après incident comprise. Le détail et ce qu'il a coûté sont
dans le [15](15-le-moteur-cesse-d-etre-mono-backend.md).

Ce qui suit sur cet axe :

- **Le sparse reste à nous** sur tous les backends, donc le contrat de décalage
  reste nécessaire. Sa formulation juste n'est pas « un backend rend des
  décalages » mais **« un décalage est nécessaire là où un index Rust vit à côté
  des données »**.
- **Neo4j en dernier, pas en deuxième.** Il parle Cypher, donc il masquerait les
  fuites de représentation qu'un backend SQL révèle. Il restera bon marché —
  c'est pour ça qu'il peut attendre.

### Côté catalogues

4. **Le graphe de normalisation de tableurs** — la porte d'entrée de toute la
   moitié KB ([02](02-les-deux-moities.md) §5). La spécification de février
   existe, **et la pratique la corrige sur quatre points**
   ([03](03-normaliser-des-tableurs.md) §9). À concevoir avec, dès le premier
   jour : les **phases persistées**, les **deux représentations**, le
   **renversement d'échelle**, et une décision explicite sur l'identité des
   lignes.
5. **Le rapport de validation à l'ingestion** — la culture `meta.warnings`
   appliquée à l'entrée, sous la forme aboutie du
   [03](03-normaliser-des-tableurs.md) §7 : localisé, illustré, dédupliqué,
   plafonné, avec un **témoin de comparaison**.

### Puis seulement

6. La surface (MCP), le modèle local, la parole.

## 3. Ce qui est en attente d'une décision humaine

- ~~**`cargo publish` de lucivy 3.0.0**~~ — **fait le 29 août 2026**, et c'est
  la **3.0.8**. Vérifié le 5 septembre : `lucivy-core = "3.0.8"`,
  `ld-lucivy = "3.0.8"`, `sparse-vector = "3.0.8"`, toutes par version et
  résolues depuis le registre. Restent deux dépendances par chemin, non-lucivy :
  `rag3db` (le cœur, dans ce dépôt) et `codeparsers` (le sous-module).
- **Le rapport de bugs à `tracel-ai`** — 16 entrées prêtes, 8 avec cas minimal.
  C'est ce qui débloquerait le Zipformer, et ce n'est pas nous qui le
  corrigerons.
- **La licence du lexique de prononciation** française — arbitrage entre un
  binaire 100 % permissif et une qualité nettement supérieure sous partage à
  l'identique (doc 47 `../23-aout-2026-20h33/`).

## 4. Dettes nommées

**Bloquantes pour une brique entière :**

- ~~**L'UPDATE de l'index HNSW segfaute au-delà de ~512 lignes**~~ —
  trouvé et corrigé le 25 août au soir (`docs/25-aout-2026-20h30/01` à la
  racine) : deux défauts dans l'extension, un hors-bornes dans le cœur.
  Sonde `e2e_hnsw_scale` conservée, canaris à 1 024 dans la passe.

- **Le Zipformer produit des valeurs fausses sur wgpu** (corr 0,47 contre
  1,000000 sur ndarray). Seul candidat STT à streaming natif. Bug amont,
  bissecté au premier bloc d'encodeur, non résoluble sans traçabilité ONNX →
  code généré.
- ~~**`kuzu` ignore la projection dans `QUERY_VECTOR_INDEX`**~~ — **corrigé le
  27 août 2026**, une ligne dans `hnsw_index.cpp` du fork :
  `searchFromUnCheckpointed` consulte enfin le masque. Le filtre est redevenu un
  **vrai pré-filtre**, et les deux post-filtres qui compensaient ont disparu.
  Les canaris ont été **retournés en affirmation inverse** plutôt que supprimés
  — `e2e_scope::the_projected_graph_honours_the_vector_filter` et
  `e2e_code::where_the_vector_pre_filter_stands_today` — pour prévenir si ça
  rechange.
- **Le G2P français** n'a aucun lexique permissif avec catégories grammaticales,
  et sans catégories, pas de liaisons.

**Silencieuses, donc dangereuses :**

- ~~**`parse_mermaid_template` rend toujours une chaîne**~~ — **corrigé le
  25 août** (commit suivant ce document). Le bug était plus large que noté :
  il touchait aussi `gpu_batch_size=$gpu_batch_size` dans `ingestion.mmd` et
  `kb_pipeline.mmd` — six gabarits sur sept, `limit` **et** la taille de lot
  GPU jamais respectés. Règle désormais : un `$var` **nu** est typé par
  inférence comme un littéral, un `'$var'` **quoté** reste une chaîne.
- **`params_object_schema` ne passe pas le mode strict.** Sans effet sur les
  appels d'outils, bloque la réutilisation d'un `ToolDef` comme schéma de sortie
  structurée. **Le « 15 sur 28 » n'est plus vérifié par personne** (5 septembre)
  : le test ne mesure plus qu'un encadrement — au moins un refus, pas tous — et
  le chiffre survit dans un commentaire figé au 25 août. Le compte de nœuds a
  d'ailleurs changé : **34**, ou 44 avec la feature `code`. Un autre commentaire
  parle encore des « 28 nœuds bruts ».
- **`special_ops`, `title_boost`, `content_boost` et le `boost` par champ**
  sont désérialisés et **jamais lus** — vérifié le 25 août
  ([05](05-ce-qui-a-tenu-depuis-fevrier.md) §8). lucivy n'a **aucune
  pondération par champ** ; leur remplacement est désormais **une topologie**
  (une branche BM25 par champ, pesée à la fusion — doc 52). Ils sont annotés
  « accepté mais non appliqué » dans `config.rs` et partiront avec le
  monolithe. `special_ops` est l'emplacement prévu pour `grep` / `read` (§2.2).
- **`fusion.rs` est mort et public** (`boost_fuse`, `weighted_fuse`,
  `rrf_fuse` : zéro appelant hors tests ; la vraie fusion est
  `search::fuse_signals`). **Revérifié le 5 septembre : toujours zéro appelant**,
  et toujours exporté. À retirer avec le monolithe.
- **La troncature d'embedding** : un chunker qui ignore la fenêtre du modèle
  tronque en silence — le MiniLM multilingue hérite d'une troncature à 128.
  **Toujours vraie au 5 septembre**, et la cause est nommée dans le code : le
  trait `Embedder` n'expose ni fenêtre ni compte de jetons, le budget de lot est
  en **caractères** parce que le tokenizer vit ailleurs. La troncature se fait
  donc en aval, chez les tokenizers, sans un mot. C'est la dernière du lot qui
  soit encore un pur silence, alors qu'on a passé la semaine à en supprimer :
  elle mérite un avertissement avant de mériter une correction.

**Nommées et assumées :**

- Build WASM non revalidé depuis mai ; les liaisons emscripten `setScope` /
  `getScope` sont écrites, pas compilées.
- La quantification q4 manque **des deux côtés** — burn ne la raccorde pas à la
  sortie de burn-onnx, et `MatMulNBits` est absent de l'IR. C'est elle qui
  déplacerait le plancher des 0,5 Md.
- `BurnLlm` à 4,7 j/s, **trois fois plus lent** que le graphe nu mesuré en
  août, inexpliqué.
- L'export **fp16 d'onnx-community est numériquement cassé** — canaris en place
  pour qu'on ne s'y refasse pas prendre.
- ~~Le reranker **remplace** le score de fusion (un mélange serait possible).~~
  Clos le 25 août : `RerankNode` en `boost` dans un `FuseResultsNode` module
  au lieu de remplacer — par topologie, sans mode.
- RBAC : pas maintenant. Charnière = `set_scope` + une future vue restreinte.
- L'éval : on ne mesure pas la pertinence. Un **cas travaillé complet** dort
  dans les documents de février ([05](05-ce-qui-a-tenu-depuis-fevrier.md) §4.5).

## 5. Fragilités d'environnement, à connaître

**Trois ajouts du 3 au 5 septembre 2026**, tous de la même espèce que le reste
de cette section — des choses qui ne cassent pas, elles trompent :

- **`run_e2e.sh` ne lance que les tests `#[ignore]`** (`-- --ignored`). Un test
  sans l'attribut est « filtered out » à chaque passe : il existe, il compile, il
  ne tourne nulle part. Vingt-deux étaient dans ce cas, dont la moitié d'une
  suite et les dix-sept de PostgreSQL. Corollaire : un test qui se relance en
  processus enfant doit passer `--ignored` à l'enfant, sinon celui-ci ne joue
  rien et le parent lit son silence comme un échec.
- **Le répertoire de build reliait un artefact du 24 août** — dix jours avant le
  correctif qui lève l'exclusion lecteur/écrivain. Toutes les suites tournaient
  contre un cœur périmé, et deux tests observaient *correctement* l'ancien
  comportement. `RAG3DB_BUILD` permet de revenir dessus, et plusieurs tests
  disent maintenant vrai des deux côtés, exprès.
- **Rien ne disait sur quelle carte charger les modèles.** Une passe complète
  chargeait BGE-M3, MiniLM, deux rerankers et l'OCR sur la carte du système,
  c'est-à-dire celle dont Lucie se sert. `RAG3WEAVER_REGIME=confort` est
  désormais le défaut du script.


- L'extension `vector` doit être **reliée** après un rebuild de `librag3db.so` ;
  `run_e2e.sh --build-only` croit la cible à jour. Sans ça, **toutes** les
  suites E2E échouent sur `undefined symbol: rag3db::catalog::IndexAuxInfo`.
- Les E2E de modèles chargent **2 Go sur le GPU chacune** et peuvent échouer en
  contention si on les enchaîne — vertes en isolation.
- Pour compter *nos* diagnostics dans le bruit de lucivy :
  `grep -cE '^\s+--> (src|tests|examples)/'`. Un `grep "rag3weaver/src/"` ne
  trouve **jamais rien** (cargo affiche le paquet local en relatif), et un
  `cargo` lancé depuis la racine du dépôt ne compile rien et rend « 0 » par
  absence de sortie. **Trois vérifications aveugles ont été payées le 25 août.**

## 6. Le principe qui ordonne tout

> Ce qu'un agent **n'arrive pas à faire** devient la feuille de route du
> dataflow. Ses échecs sont un signal, pas une gêne.

C'est le corollaire de la décision « aucun Cypher pour les agents »
([01](01-la-vision.md) §7.3) : si une capacité manque, on l'ajoute à
l'abstraction. Le prix — chaque capacité se paie en travail au lieu d'être
empruntée — est le prix de l'agnosticité, et il vaut mieux le voir venir que le
redécouvrir.
