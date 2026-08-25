# 06 — La feuille de route

Décidé le 25 août 2026, contre l'idée de livrer une surface : **on ne livre pas,
il manque de quoi avaler le réel.**

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

1. **`codeparsers` intégré.** 24 555 lignes, 78 fichiers, 12 langages, un crate
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
2. **`grep` et `read`** — voir [05](05-ce-qui-a-tenu-depuis-fevrier.md) §4.2. Les
   deux outils qu'un agent de code utilise le plus, les briques existent dans
   lucivy, et **le champ `special_ops` attend toujours dans notre config**.
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

Reste, dans l'ordre : **`Catalog::search` devient un gabarit** (le monolithe de
450 lignes, deux chemins à maintenir tant que ce n'est pas fait) ; le titre
d'une entité simple indexé en BM25 ; `column=` sur le vecteur (seconde colonne
d'embedding à l'ingestion) ; un E2E de fusion inter-KB.

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

- **`cargo publish` de lucivy 3.0.0** — irréversible. À la publication,
  remplacer nos dépendances par chemin par `lucivy-core = "3"`,
  `sparse-vector = "0.3"`, `luciole = "0.2"`, et **vérifier qu'il n'y a qu'une
  entrée `luciole` dans `Cargo.lock`**.
- **Le rapport de bugs à `tracel-ai`** — 16 entrées prêtes, 8 avec cas minimal.
  C'est ce qui débloquerait le Zipformer, et ce n'est pas nous qui le
  corrigerons.
- **La licence du lexique de prononciation** française — arbitrage entre un
  binaire 100 % permissif et une qualité nettement supérieure sous partage à
  l'identique (doc 47 `../23-aout-2026-20h33/`).

## 4. Dettes nommées

**Bloquantes pour une brique entière :**

- **Le Zipformer produit des valeurs fausses sur wgpu** (corr 0,47 contre
  1,000000 sur ndarray). Seul candidat STT à streaming natif. Bug amont,
  bissecté au premier bloc d'encodeur, non résoluble sans traçabilité ONNX →
  code généré.
- **`kuzu` ignore la projection dans `QUERY_VECTOR_INDEX`** — tous les filtres
  vectoriels sont des post-filtres. Canari en place.
- **Le G2P français** n'a aucun lexique permissif avec catégories grammaticales,
  et sans catégories, pas de liaisons.

**Silencieuses, donc dangereuses :**

- ~~**`parse_mermaid_template` rend toujours une chaîne**~~ — **corrigé le
  25 août** (commit suivant ce document). Le bug était plus large que noté :
  il touchait aussi `gpu_batch_size=$gpu_batch_size` dans `ingestion.mmd` et
  `kb_pipeline.mmd` — six gabarits sur sept, `limit` **et** la taille de lot
  GPU jamais respectés. Règle désormais : un `$var` **nu** est typé par
  inférence comme un littéral, un `'$var'` **quoté** reste une chaîne.
- **`params_object_schema` ne passe pas le mode strict** — 15 de nos 28 nœuds
  échouent. Sans effet sur les appels d'outils, bloque la réutilisation d'un
  `ToolDef` comme schéma de sortie structurée.
- **`special_ops`, `title_boost`, `content_boost` et le `boost` par champ**
  sont désérialisés et **jamais lus** — vérifié le 25 août
  ([05](05-ce-qui-a-tenu-depuis-fevrier.md) §8). lucivy n'a **aucune
  pondération par champ** ; leur remplacement est désormais **une topologie**
  (une branche BM25 par champ, pesée à la fusion — doc 52). Ils sont annotés
  « accepté mais non appliqué » dans `config.rs` et partiront avec le
  monolithe. `special_ops` est l'emplacement prévu pour `grep` / `read` (§2.2).
- **`fusion.rs` est mort et public** (`boost_fuse`, `weighted_fuse`,
  `rrf_fuse` : zéro appelant hors tests ; la vraie fusion est
  `search::fuse_signals`). À retirer avec le monolithe.
- **La troncature d'embedding** : un chunker qui ignore la fenêtre du modèle
  tronque en silence — le MiniLM multilingue hérite d'une troncature à 128.

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
