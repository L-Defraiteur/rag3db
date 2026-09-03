# Rapport de session — 3 septembre 2026

Dix-huit commits, trois sessions en parallèle. Ce document dit ce qui a été
fait, puis **ce qui reste à réparer et dans quel ordre** — c'est sa dernière
section qui compte le plus.

## 1. Ce qui a été livré

### PostgreSQL est prouvé — huit tests, et il n'en avait aucun

`PostgresDialect` faisait 944 lignes, compilait, et n'avait **jamais parlé à une
base**. `tests/e2e_postgres.rs` couvre maintenant : le schéma, l'écriture, le
vecteur, le plein texte, la fusion hybride, les relations, les cellules
multi-locataires, et les accents.

Ce que la vraie base a trouvé — **tout du code qui compilait et n'avait jamais
tourné** :

| défaut | conséquence |
|---|---|
| l'hôte formaté avec le `Debug` d'une énumération | aucune connexion n'était possible ; le dialecte n'avait donc jamais pu être faux |
| `Handle::current()` demandé à l'appel | « no reactor running » depuis les fils de lucivy, verrous empoisonnés |
| cinq organes parlant Cypher en dur | checkpoints, blobs, nœuds de cellule, comptage, et neuf appels prenant les enveloppes `generate_*_ddl` qui se rabattent en silence sur `Rag3dbDialect` |
| treize requêtes en lot en `unnest($items) AS v(colonnes)` | forme que `unnest` n'a pas ; **tout le chemin d'écriture était mort-né** |
| `cypher_to_pg_param` filtrant les `Map` | une `List<Map>` — les lignes d'un lot — perdait chaque map en silence |
| aucun index hors clés primaires | `_row_id`, `to_uuid`, `_parent_uuid`, le préfixe des blobs : le chemin le plus chaud balayait la table entière |
| le plein texte natif sans cloisonnement | **fuite entre cellules** : deux locataires, mêmes descriptions, même score |
| `vector_search_filtered` avec un alias inexistant et ses paramètres non transmis | le chemin vectoriel **filtré** n'avait jamais tourné |

### Le plein texte servi par la base

lucivy coûte trop cher en espace disque pour de la production : sur postgres,
elle demandait un **second corpus** dans le magasin de blobs. D'où
`MoteurTexte::{Auto, Lucivy, Natif}` — une **option**, pas un remplacement.

Deux étages : la base fait le **rappel** (index GIN trigramme, qui vit avec les
données), nous faisons l'**ordre** (Jaro-Winkler, sur quelques dizaines de
candidats). Vérifié que `fuzzystrmatch` n'offre pas jaro sur l'image, donc
l'ordre est en Rust — `src/jaro.rs`, qui passe les valeurs de référence de
Winkler.

Puis **le trigramme indexe les chunks** et non l'entité : l'extrait devient le
texte du chunk, sans appariement de spans.

Et les **accents sont normalisés des deux côtés** — inspiré de ragkit, avec son
piège : `unaccent()` est STABLE, donc inutilisable dans une expression d'index ;
il faut un enrobage IMMUTABLE avec le dictionnaire nommé, employé à l'index **et**
à la requête.

### Les deux forks de Kuzu, repérés et tranchés

Dans [`docs/3-septembre-2026-14h42/`](../../../docs/3-septembre-2026-14h42/) à la
racine, par la session `rag3db-57`.

**LadybugDB : non.** Pas d'histoire git commune, renommages en sens contraire,
et surtout leurs extensions et API sorties en dépôts séparés — 171 de nos
fichiers ne se fusionnent plus fichier par fichier. Leur descente de prédicat
sert l'ART, la nôtre la similarité : orthogonales.

**Vela : oui, par les lectures.** Même histoire git, aucun renommage, tout dans
un arbre. Trois lignes reportées, et **mesuré** : un second processus peut lire
pendant qu'un écrivain travaille, ce qu'il lit est cohérent, et le seul refus
observé (`Couldn't replay shadow pages under read-only mode`) est **transitoire**.

### Le reste

- rag3weaver porte sa **licence LRSL** avant que la racine passe en MIT.
- **codeparsers** est sorti en dépôt séparé (session `rag3db-b5`), `.h` envoyé à
  la grammaire C++ (+14 041 scopes), et les fichiers texte entrent dans l'index.
- L'**identité git** professionnelle est retirée de l'historique de codeparsers.
- La branche a **fusionné dans `master`**, en avance rapide.
- Doc de vision 14 : [le schéma comme artefact](../vision_roadmap_08_2026/14-le-schema-comme-artefact.md).

## 2. Trois défauts trouvés *dans nos propres tests*

Ils méritent leur section parce qu'ils sont de la même famille, et que la
journée l'a fait ressortir trois fois : **c'est le silence qui est le défaut,
pas l'erreur.**

**Quatre tests de `serveur.rs` tombaient au hasard.** `port_libre()` liait le
port 0 puis relâchait. Deux fausses pistes avant la bonne — la course est
**interne** (un test relâche, un voisin prend et tient), et une adresse de
bouclage privée n'y change rien puisqu'un écouteur sur `0.0.0.0` répond partout.
La correction : ces tests n'ont jamais eu besoin d'un port **liable** mais d'un
port **sourd**. `127.0.0.N:1`, où lier demande des privilèges. 0 échec sur 100.

**`scope_bm25_index_per_cell_never_sees_the_other_cell` criait à la fuite** là
où le cloisonnement était intact : ses deux textes « exclusifs » partageaient le
mot `private`, et le test ne tenait que sous l'ancien mode `Contains`. Corrigé
au montage — des mots disjoints — pour qu'il soit indifférent au mode.

**`e2e_postgres` mentait sur elle-même** : sept tests partageant une base, chacun
rasant le schéma, donc six échouaient en parallèle. Le `--test-threads=1` que
j'utilisais corrigeait le symptôme et laissait le piège. Le verrou est
maintenant dans la suite.

## 3. Ce qui reste à réparer, dans l'ordre

L'ordre est celui du **coût de ne pas le faire**, pas celui de la difficulté.

### 1. La marque d'eau d'ingestion — *préalable, pas suite*

`Consistency::{Immediate, Eventual, Strict}` est **intra-processus** : la file
d'attente vit dans le `Catalog` de l'écrivain, en mémoire. Vérifié par la
session voisine — `Strict` appelle `self.drain()`, et `has_pending()` ne teste
que `!self.pending.is_empty()`. Un lecteur d'un autre processus a son propre
catalogue, dont la file est vide : il demande `Strict` et obtient `Immediate`,
**sans que rien ne le dise**.

Le verrou n'a jamais protégé de ça — il rendait l'accès concurrent
*impossible*, pas *ordonné*. On ne perd donc rien qu'on avait : on découvre une
garantie qui n'a jamais franchi la frontière du processus, et qui devient
visible maintenant qu'on peut la franchir.

**Forme proposée** : l'écrivain publie son avancement dans `_catalog_meta`, que
les deux backends savent déjà écrire ; le lecteur qui veut `Strict` attend que
la marque dépasse ce qu'il attend.

**Pourquoi en premier** : c'est le préalable à ce que `rag3daemon` cesse de
relayer les lectures, et tant qu'il relaie, rien ne presse — mais bâtir dessus
sans elle installerait une promesse fausse.

### 2. La reprise sur refus transitoire dans `rag3daemon`

Le refus de pages fantômes est transitoire : cinq à six sur quatre-vingts
cycles, résolus en trois tentatives, aucun perdu. Quelques millisecondes
d'attente suffisent. **Ce n'est pas une lecture qui échoue, c'est une lecture
qui attend.** À faire avec le 1, pas avant.

### 3. Les poids du combo, qui restent une supposition

0,35 trigramme / 0,65 Jaro-Winkler, posés au jugé. Le banc qui les tranchera est
`e2e_phase0b`, celui qui a déjà départagé `Contains`, `ContainsSplit` et
`Parse`. Ragkit mesure 60 % / 85 % / 87 % sur **des noms de médicaments** — ça
valide la *forme* à deux étages, pas les *poids* pour un corpus de code et de
documents.

### 4. Le seuil de confiance, qu'on n'a pas du tout

Notre recherche rend **toujours** quelque chose, même sur une requête absurde.
Ragkit calibre un seuil (0,7) et choisit de **marquer** plutôt que de **filtrer**,
parce que le recouvrement est réel — le pire bruit dépasse la pire vraie requête.
On n'a ni seuil, ni marque, ni notion d'introuvable. C'est le manque le plus
visible pour qui s'en sert.

### 5. Le filtre utilisateur sur le chemin texte natif — **fait**

`search_texte_natif` recevait la **cellule** mais pas la condition de filtre
générale : ni appliquée, ni signalée.

Le test écrit avant la correction — `le_filtre_utilisateur_tient`, trois produits
de même description et de prix différents — en a trouvé **deux** au lieu d'un :

| chemin | ce qu'il faisait |
|---|---|
| texte natif | rendait les trois produits sous `price < 20`, **en silence** |
| vectoriel filtré | `missing FROM-clause entry for table "p"` — **toute** recherche sous filtre utilisateur mourait |

Même cause : la jointure chunk→parent était écrite en **Cypher en dur** dans
`compile_filter_for_vector`, alors que tout le reste du filtre passait déjà par
le dialecte (`filter_join_clause`). Le backend postgres recevait ce `MATCH`
sous le nom `filter_match` et le **jetait**.

Corrigé par `SchemaDialect::chunk_parent_join` — `MATCH (n)-[:X_CHUNKED_FROM]->(p:X)`
d'un côté, `JOIN X AS p ON p._uuid = n._parent_uuid` de l'autre — et par
`compile_filter_utilisateur`, qui rend le filtre **sans la cellule** : celle-ci
reste un paramètre à part, pour qu'une frontière de locataire ne dépende jamais
de la présence d'un `WHERE`.

**Puis le silence lui-même a été fermé.** `SearchBackend::honore_le_filtre()`
rend `false` par défaut — délibérément : un backend neuf est bruyant tant qu'il
n'a pas dit le contraire. Quand il ne garantit rien, les deux chemins poussent
« les résultats ne sont peut-être PAS restreints au domaine demandé ».

Et en vérifiant que l'avertissement **arrive** à l'agent, trois coupures :

| où | ce qui se perdait |
|---|---|
| `VectorSearchNode` | disait « les résultats ne sont PAS restreints » dans son **journal**, que l'appelant ne lit pas |
| `merge_port_values` | ne fusionnait pas deux `SearchMeta` — donc on ne *pouvait* pas brancher deux signaux sur le même port |
| schéma des fabriques | `meta` absent de `BM25SearchNode` et `RenderResultsNode`, alors que les nœuds l'émettaient — la **composition** ne le voyait pas |

La dernière est la plus coûteuse : `search.mmd`, l'outil réellement offert aux
agents, compose par-dessus `search_base` et réécrivait la fiche à partir des
seuls résultats. **Aucun avertissement du moteur n'atteignait un agent depuis
que la composition existe** — pas même celui du 29 août, qui avait pourtant été
réparé un étage plus bas.

Fermé par un passe-plat `render.meta`, deux arêtes dans les gabarits, et un
test qui compare le schéma déclaré à l'implémentation.

**Puis les deux sources ont été unifiées.** Vérifié d'abord qu'aucun nœud
feuille ne calcule ses ports depuis sa configuration — seuls `Graph` et
`GraphNode` le font, et eux n'ont pas de schéma statique. Les ports sont donc
une constante du **type**, pas de l'instance : une seule écriture suffit.

Le schéma de fabrique reste le rédacteur, parce qu'il est le seul disponible
**avant** d'avoir une instance : le sondeur de `GraphNodeFactory::templated`
lit des schémas de type sur une définition où les `$param` peuvent encore
traîner. Les 44 implémentations de nœud délèguent maintenant à
`ports_declares(&XFactory)` — 372 lignes supprimées pour 137.

Deux choses se sont vues en le faisant :

- Le test ne comparait que les **noms** de ports. `KBGatherNode.aggregates`
  était `required` au schéma et facultatif au nœud ; en unifiant, le runtime
  s'est mis à attendre une entrée qui ne vient pas — sept tests d'ingestion.
  Le nœud avait raison (il se nourrit aussi du service `pending_aggregates`),
  le schéma a été corrigé, et le test compare désormais nom, type **et**
  `required`.
- Le test ne couvrait que les 26 types constructibles sans configuration. Il
  couvre les 34, avec une table de configurations minimales, et **refuse de
  rétrécir** : un type neuf sans entrée dans cette table fait échouer le test
  au lieu d'être sauté en silence.

### 6. `test/api/db_locking_test.cpp` affirme un contrat mort

Il énonce l'exclusion lecteur/écrivain que le report de Vela a levée. Il n'est
compilé par personne et ne l'était déjà pas en amont — code de test mort hérité
de Kuzu. À supprimer ou à réécrire, pas à laisser mentir.

### 7. Le fond de la concurrence de Vela, non relu

MVCC, rotation de WAL, points de reprise non bloquants. C'est le plancher de
l'écriture multi-processus, et personne ne l'a examiné. **Préalable à toute
adoption de leur moitié écriture**, pas à la lecture qu'on vient de prendre.

### 8. Le renommage `kuzu` → `rag3db`, à questionner

2 899 fichiers à chaque reprise de l'amont. Ce n'est pas urgent, mais c'est une
dépense récurrente dont personne n'a énoncé le bénéfice.

## 4. Ce qui n'a pas été fait, et qu'il ne faut pas croire fait

- **Rien n'a été compilé de LadybugDB ni de Vela** hors le report de trois
  lignes. Leurs builds restent non tentés.
- **Les 171 fichiers hors du cœur** face à Ladybug ne sont pas chiffrés, et ne
  peuvent pas l'être depuis ce dépôt.
- **Aucun de mes tests postgres n'éprouve `Strict`** : ils passent tous
  `Immediate`, et tiennent parce que l'ingestion est synchrone juste avant.
- **Deux tests de `e2e_simple_entity` échouent, et c'est antérieur** —
  `simple_batch_delete_multiple` et `simple_batch_update_multiple`. Vérifié en
  remisant : ils échouent aussi sans le travail d'aujourd'hui. Le batch
  delete/update ne retire pas l'ancien contenu de l'index plein texte ; le
  symptôme est décrit depuis le 8 mars 2026
  ([rapport batch CRUD](../../../docs/8-mars-2026-01h54/12-rapport-progression-batch-crud.md)).
  Sur 50 suites E2E, c'est le seul échec.
- **Le miroir Rust de `search_base` reste couplé au graphe de production.**
  `search_in_rust`, 70 lignes dans les tests, recopie `search_base.mmd` nœud
  par nœud pour prouver que l'analyseur Mermaid et l'API Rust construisent la
  même chose. L'intention est juste, le sujet ne l'est pas : toute évolution du
  graphe de production oblige à éditer le miroir. **Le générer serait pire** —
  il prouverait que l'analyseur est d'accord avec l'analyseur. Le bon geste est
  un fixture minimal dédié, découplé de `search_base`.
