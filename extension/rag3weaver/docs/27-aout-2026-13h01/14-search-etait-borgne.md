# `search` était borgne, et ce n'était pas l'embedder

*27 août 2026, fin de soirée. Fait suite au doc 11 §5 et au doc 13 §7.*

## 1. La question

> « donc les embeddings fonctionnent ou pas ? ou c'est un mock encore ? »

La réponse honnête est : **les deux réponses évidentes étaient fausses**.

L'embedder était vrai. Dans `fil-vrai-moteur.md`, écrit à 22 h 18, le
catalogue portait bien un BGE-M3 chargé sur burn, 1 024 dimensions, poids sur
le disque. Et pourtant les quatorze résultats de `search` sortaient tous
étiquetés `bm25`, et « node failure » ne rendait rien.

Ce n'était pas un embedder factice. C'était **un outil qui n'appelait jamais
l'embedder**.

## 2. Ce que `search` était

`templates/tools/search.mmd`, jusqu'à ce soir :

```
source["SearchSourceNode(...)"]
bm25["BM25SearchNode(limit=$limit)"]
resolve["ResolveParentNode"]
render["RenderResultsNode"]
```

Quatre nœuds, pas un seul vectoriel. Le graphe `simple_hybrid_search.mmd`
existait à côté, complet, avec sa fusion RRF — et personne ne le branchait
sur l'outil que les agents tiennent dans la main.

C'est la sixième trouvaille de la même famille cette semaine : un mécanisme
construit, testé, documenté, **et jamais atteint depuis le chemin réel**
(`BurnDevice::for_role`, `Postures::describe_for`, la borne du lot dense,
`pause_dialogue`, `embed_batch_size`). La leçon se répète : ce qui n'a pas de
test *de bout en bout par le chemin de l'utilisateur* n'existe pas, même
quand il a des tests unitaires verts.

Et le symptôme, lui, avait l'air d'un défaut du modèle : « les agents
n'utilisent pas `search`, ils préfèrent `grep` ». Trente-neuf tours, zéro
appel. On a failli en conclure quelque chose sur les agents.

## 3. Ce que `search` est

```
source → bm25   ↘
source → vector → fuse → resolve → render
source ─────────────────────────────↗ (query)
```

Trois changements l'accompagnent, chacun réparant une chose qui ne se voyait
pas :

**Une cible sans vecteurs ne casse plus le graphe.** `VectorSearchNode` et
`SparseSearchNode` regardent ce que la cible déclare (`SearchTarget::
default_signals`, que `SearchOptions` peut surcharger) et rendent une liste
vide **en le disant** plutôt que d'échouer. Sans ça, `Symbol` — déclaré BM25
seul le 26 août pour ne pas payer 3 275 embeddings de noms — rendrait l'outil
inutilisable, et le seul recours aurait été de garder `search` borgne pour
tout le monde.

**La provenance survit à la fusion.** `FuseResultsNode` écrasait `signal` par
son propre nom : après fusion, plus moyen de savoir si un résultat venait du
plein texte ou du vecteur. Il rend maintenant `bm25`, `vector`, ou
`vector+bm25` quand les deux l'ont trouvé. C'est ce qui a permis de vérifier
la réparation en une ligne de trace :

```
### 1. Rust Book ★ 0.02 · vector+bm25
### 2. French Chef Knife ★ 0.01 · vector
```

Le couteau n'est trouvé que par le vecteur, sur la requête « programming
language ». C'est un mauvais résultat — et c'est exactement la preuve que la
branche dense fonctionne.

**Les sessions d'agents ont de vrais embeddings.** `e2e_burn_code_agent` et
`e2e_cloud_code_agent` montaient leur catalogue avec `HashEmbedder::new(64)`.
Les deux prennent maintenant `BGE_M3`, comme `e2e_conversation_a_plusieurs`.
Une expérience sur le comportement d'agents dont l'outil est truqué ne mesure
pas le comportement d'agents.

## 4. Le rendu est un gabarit

Le second morceau de la soirée. La forme d'une fiche de résultat était un
`format!` de cent lignes dans `render_nodes.rs` : pour la changer il fallait
recompiler le moteur.

La séparation est maintenant nette :

- **Rust décide *quoi* montrer** — `build_view()` produit un `ResultsView` :
  la lentille de chemin appliquée, les groupes formés, les champs internes et
  nuls disparus, les extraits bornés, les voisins regroupés par relation, le
  décompte par type.
- **Le gabarit décide *comment*** — `templates/render/*.md.jinja`, minijinja.

La forme par défaut reprend `BRAIN_SEARCH_OUTPUT_PROPOSAL.md` et
`brain_search_example_output.md` de `LR_CodeRag` (la maquette d'origine, cf.
doc 11 §6) :

```
# Search: "drain" — `Scope`

**Results:** 3

---

## Results

### 1. Catalog::drain (function) ★ 14.19 · vector+bm25
📍 `src/catalog.rs:3172-3233`
🔹 `pub fn drain(&mut self) -> Result<DrainStats, CatalogError>`
📝 Vide les insertions en attente.

---

## Dependency Graph

```
Catalog::drain (function) ★ 14.19 @ src/catalog.rs
└── [DEFINED_IN]
    └── catalog.rs (File)
```

---

## Node Types Summary

| Type | Count |
|------|-------|
| function | 2 |
| struct | 1 |
```

Ce que ça répare, point par point, par rapport à la fiche d'avant :

| Avant | Maintenant |
|---|---|
| tout sur une ligne, le lieu noyé au milieu | `📍` sur sa ligne, copiable tel quel dans un `read` |
| `docstring=…` perdu parmi les colonnes | `📝`, promu ; `signature` en `🔹` |
| `scope_type=function` dans la liste brute | `(function)` dans le titre |
| les voisins en `↳` mêlés aux résultats | un graphe à part, groupé par relation |
| rien | un décompte par type, pour voir la forme du résultat d'un coup d'œil |

`compact` reste fourni : une ligne par résultat, trois fois moins cher en
jetons. Un flux qui paie ses résultats au jeton le demande explicitement.

**`template=` prend un nom, pas un chemin.** Un graphe peut être écrit par un
modèle : accepter un chemin arbitraire ferait de ce champ une lecture de
fichier quelconque, rendue au modèle, qui contournerait le domaine de travail
par lequel passent `read` et `grep`. Un nom se résout dans
`templates/render/<nom>.md.jinja` (`$RAG3WEAVER_RENDER_TEMPLATES` pour
déplacer le répertoire) ; du Jinja écrit en toutes lettres passe aussi ; un
`/` ou un `..` est refusé, au **montage du graphe** et pas au milieu d'un tour.

## 4 bis. Le cross-encoder, allumé par celui qui pose la question

> « peut-être brancher cross encoder aussi dès que vector utilisé, ou bien en
> option les agents mettent mode "sentence" »

**Pas « dès que vector ».** Depuis le §3 le vecteur est *toujours* branché :
« dès que vector » voudrait donc dire « à chaque appel de `search` ». Or la
moitié des requêtes d'un agent sur du code sont un nom de fonction, et un
cross-encoder n'a rien à dire d'un identifiant — il compare deux phrases. Ce
serait un modèle de plus sur le GPU, à chaque tour, pour rien.

C'est donc la seconde version, avec un paramètre qui **oblige à la seule chose
qui le rend utile** : poser une vraie question.

```
search(target="Scope", query="comment un nœud signale son échec", rerank=20)
```

Sa fiche, telle que le modèle la lit :

> `rerank` — Faire relire les N premiers résultats par un cross-encoder qui
> compare votre requête à chacun, phrase contre phrase, et les remet dans
> l'ordre. Ne le demandez que si `query` est une vraie question en langue
> naturelle — sur un identifiant il ne sert à rien et coûte cher. 20 est une
> bonne valeur ; 0 le désactive.

**Un nombre plutôt qu'un mot.** `mode="sentence"` aurait demandé au graphe une
conditionnelle qu'il n'a pas : un graphe-outil est figé, seuls les `$var` sont
substitués. Un entier fait les deux à la fois — il dit *si*, et il dit
*combien*. Ce qui l'a rendu possible tient en deux lignes de `RerankNode` :

- **`candidates = 0` est un passe-plat exact.** Il était refusé
  (« must be at least 1 ») ; il vaut maintenant « passe » : pas de service
  consulté, pas d'étiquette touchée, pas de ligne de journal. Une absence
  voulue n'est pas un incident.
- **`keep_signal`** garde la provenance. Le rerank est ici la dernière étape
  avant le rendu : `bm25+vector` vaut mieux que le nom du dernier nœud
  traversé. Le défaut ré-étiquette toujours — c'est ce qui permet à une fusion
  en aval de reconnaître le rerank par son nom et de l'utiliser en `boost`
  (`generic_rerank_as_boost_signal_inside_fusion`).

Sans service `reranker` enregistré, le nœud conserve l'ordre d'entrée et le
dit. Un agent qui demande `rerank=20` sur un montage sans cross-encoder obtient
donc des résultats, pas une erreur.

## 5. Ce que la machine paie

Troisième morceau, arrivé en cours de route : « ma machine recommence à
galérer ». Un traceur, `charge.py`, échantillonne toutes les cinq secondes —
charge par cœur, CPU, RAM, **swap**, occupation et VRAM par carte, les trois
plus gros processus par mémoire résidente. Il tourne pendant `run_e2e.sh`,
build C++ compris, et laisse son TSV dans `target/charge-last.tsv` ; le résumé
des pics s'affiche à la fin de la passe.

Ce qu'il a mesuré dès la première passe :

- **Ce n'est ni le CPU ni le GPU.** Au pire moment : charge 84 sur 24 cœurs
  (3,5 par cœur), CPU à **11 %**, GPU à 0 %.
- **C'est l'édition de liens.** Vingt-deux `rust-lld` en parallèle, ~2,8 Go
  de mémoire résidente chacun — parce qu'un binaire de test fait **1,4 Go**,
  presque entièrement du DWARF. Trente-quatre binaires, ~48 Go sur le disque.
- **Le swap est de la zram.** 98 Go nominaux, `vm.swappiness` à 80. « Swapper »
  ici veut dire compresser en RAM ; y revenir veut dire décompresser, au CPU.
  D'où le motif : le poste rame *et* le CPU est à 10 %.
- **`jobs = -2` ne borne pas ce qu'il faut.** Il borne le parallélisme *CPU*,
  qui est le bon réglage pour compiler et le mauvais pour lier : lier est
  gourmand en mémoire, pas en cycles.

La piste, à mesurer à la prochaine passe : `split-debuginfo = "unpacked"` sur
le profil de test, qui laisse le DWARF dans les `.o` au lieu de le faire
recopier par lld dans chaque binaire.

## 6. Le rôle qu'on avait oublié de placer

Mesuré pendant cette même passe, lancée avec
`RAG3WEAVER_BURN_DEVICE_{EMBEDDER,RERANKER,OCR}=gpu:1`. Deux instants, à
trente secondes d'écart :

```
23:32:32   card0  vram 30544 Mo      card2  busy 40%  vram  6791 Mo
23:33:02   card0  vram 32182 Mo      card2  busy  4%  vram  2120 Mo
```

Le premier, c'est `e2e_burn_code_agent` ; le second, `e2e_burn_embedder`. Le
mapping du doc 13 §4 est **juste** : `gpu:1` tombe bien sur card0, et
l'embedder y va comme demandé.

Ce qui n'y va pas, c'est **le LLM**. `RAG3WEAVER_BURN_DEVICE_LLM` ne figurait
pas dans l'incantation — trois rôles sur quatre. Qwen part donc sur la carte
par défaut, qui est card2, celle des écrans : d'où les 40 % d'occupation et
les 6,8 Go pendant la suite d'agent. On épargnait consciencieusement les
écrans du seul modèle qui ne les gênait pas.

L'incantation complète :

```sh
RAG3WEAVER_BURN_DEVICE_EMBEDDER=gpu:1 \
RAG3WEAVER_BURN_DEVICE_RERANKER=gpu:1 \
RAG3WEAVER_BURN_DEVICE_OCR=gpu:1 \
RAG3WEAVER_BURN_DEVICE_LLM=gpu:1 \
./run_e2e.sh --summary
```

**Sauf que card0 ne peut plus les prendre.** llama-server y tient 30,5 des
32,6 Go en permanence ; BGE-M3 seul la porte à **32,2 Go, soit 98,7 %**. Le
reranker et l'OCR par-dessus ne tiennent pas. Le vrai levier n'est pas dans
les variables d'environnement : c'est de libérer card0, ou d'assumer que le
travail passe par la carte des écrans.

Et la leçon de méthode reste : le placement se **vérifie**, il ne se lit pas
dans un document. `charge.py` le montre en deux lignes, à chaque passe.

## 7. Ce qui reste

- Mesurer `split-debuginfo = "unpacked"` (§5).
- Refaire tourner `e2e_conversation_a_plusieurs` avec le `search` réparé et
  comparer à `fil-vrai-moteur.md` : les sondes du témoin contiennent déjà
  `("Scope", "node failure")`, la requête qui rendait zéro.
- `Meter::describe()` n'a toujours aucun appelant.
- `Trace` n'a toujours pas de `hashsafe`.
