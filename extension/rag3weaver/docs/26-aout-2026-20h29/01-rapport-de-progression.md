# 01 — Rapport de progression, 26 août 2026

Trente-six commits entre minuit et la mi-journée, sur trois fronts qui se
sont révélés être le même : **ce que l'agent n'arrive pas à faire est la
feuille de route**. Chaque défaut trouvé ici l'a été en regardant un modèle
travailler ou en lisant une mesure, jamais en imaginant.

## 1. Ce qui a été livré

### Les fiches d'outils deviennent contraignantes

| Commit | Ce que ça change |
|---|---|
| `ee7f95311` | `%% choices:` dans une fiche : liste close, ou `@targets` / `@relations` résolus **contre le catalogue vivant**. Une valeur hors liste est un `bad_choice` **avant** d'instancier le graphe, avec la liste dans l'erreur. |
| `88ac04cf7` | Généralisé : `Choices` et `json_schema` descendent dans `ConfigParam`, et une fiche **hérite par câblage** de ses `$var` — à travers les niveaux d'imbrication. Un paramètre déclaré sans type prend celui du nœud. |
| `0c6ea9f53` | Les listes closes deviennent **exactes** : `result_mode`, `mode`, `strategy`, `format`, `direction`. Les alias meurent, le repli silencieux `_ => Outgoing` devient une erreur. |

Mesuré : Gemini n'invente plus `HAS_SIGNALS` ; l'erreur nomme les neuf
relations réelles.

### Le rendu : quatre fois moins de jetons, et deux défauts révélés

| Commit | Ce que ça change |
|---|---|
| `09c4ef782` | `RenderResultsNode` : markdown compact, **passe-plat** sur `results` — c'est ce port qui reste libre, et c'est par lui que `search_expand` compose. |
| `368ce3a51` | Mesuré : **1 027 caractères contre 4 721** en JSON, à trajectoire identique. |
| `13159b2ac` | Liens `port.rs:120-140`, hiérarchie `PortValue::take`, regroupement par classe. **Et le défaut que ça révèle** : `ResultMode::Aggregated` rendait une entrée **par chunk** au lieu d'une par parent, alors que la limite BM25 borne les parents. |
| `ae72bc498` | Un préfixe qui ne rend rien **dit pourquoi** — « les chemins sont relatifs à la racine de cette source, pas au dépôt ». |

*Rendre lisible, c'est rendre vérifiable* : le doublon de scopes et le
`list(prefix)` muet ont tous deux été trouvés en lisant la sortie compacte.

### Les événements, les runs, les boucles

| Commit | Ce que ça change |
|---|---|
| `aee27c5b8` | **Un bus, plusieurs sujets** créés à la demande, curseurs nommés. L'agent et le runtime publient ; un graphe de trace consomme — sans écho, par construction. |
| `6319b4f3a` | **L'identité d'un run est son adresse** : `run_id` sur chaque événement, sujet `run.<id>`, lien parent quand un outil lance un graphe. On obtient un arbre. |
| `6355333fc` | Les runs **se parlent** : `SendMessageNode`, boîte lue **entre deux tours**, entités `Run` et `Message` liées — donc conversations cherchables. |
| `b67fbc181`, `252e12bfe` | **Le réacteur** : `%% on:` / `%% policy:` (each / batch / debounce), un fil qui **attend** (tokio `select`) au lieu de sonder. Deux agents conversent par leurs boîtes. |
| `71b0df276` | **La cellule est l'espace de noms du bus** : `org/project/<sujet>`. Un joker ne peut pas traverser une organisation — inexprimable, pas seulement interdit. |

### Deux dépendances tranchées

- `f40b827cd` — **wasm abandonné** pour rag3weaver : 1 782 lignes de FFI et
  deux features en moins. Ce n'était pas du code mort, c'était une
  contrainte d'architecture (ni fils, ni async) payée pour un usage que
  personne n'avait.
- `f0f29d52e`, `53fead88f`, `70a4bb3c5` — **luciole retiré**. Mesuré avant
  de décider : `PortValue` était le sien, `execute_via_luciole` n'avait que
  **deux** appels, le puits SSE lui empruntait deux lignes. Notre runtime
  exécute maintenant chaque niveau en parallèle. `e2e_search` : 13,8 s
  avant, 14,0 s après.

### Le code : lire, résoudre, et savoir ce qu'on ne sait pas

| Commit | Ce que ça change |
|---|---|
| `f31c5f223` | **Lire hors index** : `RootPolicy` (`closed` / `anywhere` / `under`), et les scopes analysés **à la volée** pour un fichier non indexé. L'index est un service rendu, pas une porte. |
| `9a01aec7f` | **La couche `Symbol`** : une référence non résolue est une donnée. Deux passes par lot, et **l'ordre d'ingestion ne change plus le graphe** — y compris « usage d'abord », que le résolveur intra-lot ne peut pas voir. |
| `2676b802b` | **Rattraper un appel d'outil resté dans le texte** — trois formes connues, borné, jamais silencieux. |

### La performance, mesurée puis corrigée

| Commit | Ce que ça change |
|---|---|
| `ae4495ad2` | Temps **par phase** dans le rapport d'ingestion. |
| `cb5e8f15f` | Profil **par nœud** (`RAG3WEAVER_INGEST_PROFILE=1`). Et le court-circuit de l'inchangé : 44 s → 2 s, **puis retiré** — la garde était fausse deux fois, la passe complète l'a attrapé. |
| `528bb092a` | `chunked: Some(false)` : `Symbol` en BM25 seul. **12,1 s → 1,7 s.** Au passage : il héritait de `HYBRID`, donc on embarquait 3 275 noms nus. |
| `e0d690482` | **Construction de l'index en masse : 24×.** 13 s d'insertions HNSW ligne à ligne contre **0,55 s** en une fois. |

## 2. Ce qui est mesuré

| | |
|---|---|
| Unitaires | 838 (`code,openai-llm`), 740 (défaut) |
| E2E | 33 suites, 257 tests — dernière passe complète verte |
| Gemini, Q3 | 369 666 jetons en 197 s → **35 432 en 30 s** |
| Qwen3-Coder-30B **local** | les cinq mêmes épreuves, **deux missions d'édition réussies**, 82 s |
| Rendu d'une recherche | 4 721 → 1 027 → **611** caractères |
| Ingestion `src/dataflow` | 28 s → 17,5 s → **5,9 s** avec l'index en masse |

## 3. Ce qu'on a trouvé sans le chercher

1. **`ResultMode::Aggregated` violait son propre contrat** — une entrée par
   chunk au lieu d'une par parent. Tout appelant payait ses résultats en
   double depuis toujours.
2. **`Symbol` héritait de `HYBRID`** : 3 275 embeddings vectoriels pour des
   noms nus, jamais décidé par personne.
3. **Notre identité de scope inclut le hash de signature**, et
   `reingest_file` supprime les scopes disparus — donc un `edit` qui change
   une signature **détruit les relations entrantes** venues d'autres
   fichiers. Le même piège que RAGForge, trouvé en relisant son code.
4. **`Consistency` est déclaré et non tenu** : aucune variante ne calcule
   d'embedding. Pire, `Eventual` — le défaut — écrit les lignes **sans les
   indexer** et vide la file, si bien que rien ne peut les rattraper.
5. **`InsertRecordNode::undo` laisse des documents fantômes** dans l'index
   plein texte.
6. **`DROP_VECTOR_INDEX` existe dans le fork et n'est référencé nulle
   part**, alors que `CREATE_VECTOR_INDEX` sur table pleine est son mode
   nominal — et que l'index est créé sur une table **vide**.
7. **`search_vector_bruteforce` a zéro appelant** : aucun repli si l'index
   est en retard.
8. **`fulltext_on_chunks` n'est jamais lu.** Drapeau mort.
9. **Ce qui faisait ramer la machine, c'est la compilation**, pas
   l'inférence : charge 11,6 avec `-j 8` pour **un** binaire de test, contre
   1,7 cœur pour quatre tests BGE-M3 sur GPU.

## 4. Les pistes à poursuivre

### Immédiat — ~~chiffré, sans risque~~ **fait le 26 au soir**

1. ~~**La bascule d'index en masse**~~ — `Catalog::bulk_vector_index`,
   explicite (pas de seuil deviné), générique (des entités, pas du code) et
   **réparable** : un drapeau dans `_catalog_meta` fait rebâtir à
   l'ouverture si le processus meurt entre la destruction et la
   reconstruction. Ingestion de `src/dataflow` **21,3 s → 6,2 s**, l'index
   passant de ~90 % du coût à 9 % (`ef0d656ab`,
   [doc 18 §8](../25-aout-2026-18h58/18-index-vectoriel-differe.md)).
2. ~~**Le court-circuit de l'inchangé**~~ — `Catalog::split_unchanged`, dans
   `ingest_entities`, avec la garde en deux conditions : tous les champs
   identiques **et** les artefacts dérivés présents. Ré-ingestion à contenu
   identique **19,5 s → 2,1 s**, entités 16 892 → 101 ms (`8ebf4ab35`,
   [doc 17 §8](../25-aout-2026-18h58/17-relations-a-travers-les-lots.md)).
3. ~~**Les relations entrantes après un `edit`**~~ — le test manquant
   échouait, et pas pour la raison prévue : `CONSUMES` n'était pas une vue
   matérialisée mais une arête sans trace, parce qu'une référence **résolue
   dans le lot** ne laissait pas de `MENTIONS`. Désormais toute référence en
   laisse un, pour un demi-seconde et pas une arête de plus au final
   (`f4498880a`, [doc 17 §9](../25-aout-2026-18h58/17-relations-a-travers-les-lots.md)).

Au passage, un défaut de la passe elle-même : `e2e_highlight_long_text` ne
compilait plus depuis l'ajout du champ `chunked`, et la passe complète
rendait « 0 passed, 33 suites non lancées » — vert de loin. Corrigé ; la
passe tourne à **263 tests sur 33 suites**.

### Court terme — les chantiers ouverts avec leur dessin écrit

4. **La session comme graphe** ([13](../25-aout-2026-18h58/13-la-session-comme-graphe.md)) —
   `assemble`, `decide`, `act`, **`absorb`**, `record`, `stop`. C'est
   `absorb` qui compte : garder le markdown entier d'un `read` au tour 8,
   c'est le payer huit fois. Chiffre de vérité : une conversation de dix
   tours, jetons avec et sans.
5. **Tout est écoutable** ([14](../25-aout-2026-18h58/14-tout-est-ecoutable.md)) —
   sélecteurs (`run`, `tag`, `kind`, `node`, `port`, `dir`), prédicats sur
   les champs, et un **registre d'intérêt** pour ne pas fabriquer ce que
   personne n'écoute. La portée est faite ; les événements de ports et le
   sélecteur restent.
6. **Le monde ouvert** ([16](../25-aout-2026-18h58/16-le-monde-est-ouvert.md)) —
   chemins auto-descriptifs (`/abs`, `proj:id/…`), `Root { expires_at }`
   avec son graphe d'entretien, et la **fiche de promotion** : un réacteur
   qui compte les visites par racine git et propose l'ingestion. C'est là
   que naîtront les gabarits de politique d'un produit **lucyCode**.
7. **Les handles** ([13](../25-aout-2026-18h58/13-la-session-comme-graphe.md) §5) —
   `#execute-2` lisible et stable, **avec** l'outil qui les résout, ou pas
   du tout.

### Fond — les questions ouvertes

8. **L'identité d'un fichier** ([15](../25-aout-2026-18h58/15-identite-d-un-fichier.md)) :
   URI de source plutôt que chemin relatif. Indexer un dossier puis son
   sous-dossier crée aujourd'hui **trois identités** pour le même fichier.
9. **Honorer `Consistency`** — et corriger `flush_insertions`, qui écrit
   sans indexer.
10. **Le repli par balayage** rebranché, sans quoi un index en retard rend
    moins de résultats **en silence**.
11. **Le cahier des charges luciole**
    ([12](../25-aout-2026-18h58/12-cahier-des-charges-luciole-parite-tokio.md)) —
    caduc pour nous depuis qu'on s'en est passés ; reste une note pour
    lucivy si ces primitives ont du sens pour elle-même.

## 5. La vision, inchangée mais mieux outillée

Le moteur ne cherche pas à être un RAG de plus : il veut que **tout soit un
graphe** — la recherche, les outils, la trace, la session, les politiques.
Ce qui a avancé aujourd'hui va toujours dans ce sens : un rendu est un
nœud, une politique est une fiche, une conversation est un run qui a une
adresse, et une boucle réagit à sa frontière.

Deux principes se sont vérifiés assez souvent pour être écrits :

> **Ce que l'agent n'arrive pas à faire est la feuille de route.** Les
> `enum`, le rattrapage d'appels, le « vouliez-vous dire », le rendu
> compact : quatre corrections nées de quatre échecs observés.

> **Rendre lisible, c'est rendre vérifiable.** Le rendu compact a révélé
> deux défauts que personne ne cherchait.
