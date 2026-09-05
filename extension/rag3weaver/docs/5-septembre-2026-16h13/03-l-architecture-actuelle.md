# L'architecture actuelle

**5 septembre 2026.** Les **contrats**, pas la liste des fichiers — c'est ce
qu'une session compressée doit retrouver.

## 1. Ce qu'un backend doit fournir : cinq organes

| organe | trait | ce qu'il fait |
|---|---|---|
| la connexion | `DbConnection` | `execute`, `execute_with_params` — **synchrone** |
| le dialecte | `SchemaDialect` | ~60 méthodes : l'**intention** rendue dans la langue du backend |
| la recherche | `SearchBackend` | vecteur, résolution de décalages, enrichissement, et **facultativement** le plein texte |
| les blobs | `BlobStore` | où vivent les index lucivy et sparse |
| les checkpoints | `CheckpointStore` | la reprise après incident |

**Deux backends** les fournissent tous les cinq : rag3db natif et
PostgreSQL/pgvector.

`SchemaDialect::speaks_cypher()` existe parce que deux organes portaient du
Cypher en dur. Le dialecte dit aussi **ce qu'il sait offrir** —
`nouveau_magasin_de_checkpoints` rend `None` s'il n'en a pas, et `initialize()`
le **dit** au lieu de démarrer amputé. Choisir sur le *nom* d'un dialecte serait
la fragilité qu'on a payée toute la semaine.

## 2. Le plein texte : deux moteurs, une option

`MoteurTexte::{Auto, Lucivy, Natif}`. `Auto` demande au backend
(`sert_le_plein_texte`), les deux autres forcent. **Ce n'est pas un
remplacement** : lucivy redeviendra le défaut quand elle sera plus légère en
disque, et le chemin du retour est éprouvé — lucivy tourne sur PostgreSQL.

Deux étages : **la base fait le rappel** (index GIN trigramme, qui vit avec les
données), **nous faisons l'ordre** (Jaro-Winkler, sur quelques dizaines de
candidats).

- `<%` / `word_similarity` et non `%` : l'opérateur simple compare deux chaînes
  *entières*, la longueur écrase le score.
- Le plancher de rappel est posé à **0,3** et non 0,6 : sinon la base tranche à
  la place de l'étage qui sait trancher.
- L'index porte sur la **table de chunks** : sans spans de surlignage, l'unité
  indexée doit être celle qu'on veut rendre.
- Les accents sont normalisés **des deux côtés** par un enrobage IMMUTABLE de
  `unaccent` — la fonction nue est STABLE, donc interdite dans un index.

## 3. Deux scores, et pourquoi ce n'est pas le même nombre

| | à quoi il sert | propriété |
|---|---|---|
| score de **classement** | ordonner les résultats d'**une** requête | relatif, et **change de nature selon le backend** — du BM25 non borné sur lucivy |
| score de **confiance** | dire si quelque chose est probant | **absolu**, borné, comparable partout |

Le second est du **Jaro-Winkler pur**, recalculé sur le texte rendu, seuil
`0,88` mesuré. Les deux chemins de texte passent par la même
`marquer_la_confiance`, donc le même nombre veut dire la même chose partout.

On **marque**, on ne filtre pas — le recouvrement est réel. Et la phrase **nomme
son signal** : elle porte sur le plein texte, sinon elle parlerait au nom du
vecteur, qui est fait pour rapprocher ce qui ne partage aucun mot.

## 4. Le contrat de décalage

Toute la résolution est bâtie sur des **décalages de ligne en u64** : kuzu a
`OFFSET(id(n))`, PostgreSQL a `_row_id BIGSERIAL`.

La bonne formulation n'est **pas** « un backend rend des décalages » mais :

> **un décalage est nécessaire là où un index Rust vit à côté des données.**

C'est ce qui le rend obligatoire pour lucivy et le sparse, et facultatif pour un
backend qui sert lui-même texte et vecteur.

**Piège associé, payé cette semaine** : l'identifiant rendu par une insertion est
une *chaîne* sur rag3db (`ID(n)`) et un *entier* sur SQL (`_row_id`). Ne lire que
la chaîne laissait le cache d'identifiants **et** l'indexation lucivy sautés en
silence.

## 5. Le cloisonnement, et le domaine de travail

`Scope { org, project }`, matérialisé par `_org` / `_project` sur toute table de
données, avec leur index.

**La cellule est un paramètre explicite et obligatoire** de `text_search`, jamais
dérivée d'un filtre construit ailleurs : une frontière de locataire ne doit pas
dépendre de la présence d'un `WHERE`.

Le **filtre utilisateur** est distinct de la cellule
(`compile_filter_utilisateur` la rend sans elle), et il descend jusqu'au SQL par
une jointure rendue par le dialecte — `MATCH (n)-[:X_CHUNKED_FROM]->(p:X)` d'un
côté, `JOIN X AS p ON p._uuid = n._parent_uuid` de l'autre.

## 6. La cohérence, et sa frontière

`Consistency::{Immediate, Eventual, Strict}`. La file vit dans `Catalog::pending`,
**en mémoire** : `Strict` appelle `self.drain()`.

Donc `Strict` ne franchissait pas la frontière du processus. Il la franchit
maintenant par une **marque d'eau** : l'écrivain pose
`_ingestion/pending/{son id}` à la transition file vide → non vide et l'efface
après chaque drain réussi ; un lecteur `Strict` attend que plus aucun écrivain ne
soit marqué.

Trois façons de ne pas y arriver, **toutes dites** : le délai expire, une marque
est périmée au-delà d'une minute — un processus mort ne doit pas geler les
autres —, ou les marques sont illisibles.

**Ce que ça ne fait pas** : un lecteur sait *qu'il* y a du travail non publié,
pas *lequel*. La marque est **binaire**, alors qu'elle devrait être par
disponibilité et par ressource — voir
[01](01-les-objectifs-et-leur-ordre.md).

## 7. Par où un lecteur atteint la base

`Acces::{Direct, Demon, Auto}`, même forme que `MoteurTexte`. `Direct` échoue
avec une **erreur nommée** plutôt que de se rabattre ; `Demon` sans serveur est
une erreur ; `Auto` prend le direct et se rabat **en le disant**. Le `Lecteur`
rendu porte `par: Chemin` en clair.

`Rag3dbConnection::read_only` retente dans un budget court sans **filtrer sur le
message** — il vient du cœur C++ et changerait sans qu'on le sache. L'erreur dit
son compte de tentatives.

## 8. Le motif des services

Les nœuds de dataflow reçoivent ce dont ils ont besoin par un `ServiceRegistry`,
**jamais en verrouillant le catalogue** : `Catalog::search()` tient déjà son
verrou quand le graphe s'exécute, donc un `lock()` depuis un nœud rend `None` —
c'est-à-dire un repli silencieux.

`Catalog::register_search_services` porte la **liste unique** : connexion,
dialecte, cellule, index FTS et sparse, embarqueurs. Elle était reconstruite à la
main dans dix montages, et aucun n'avait le dialecte.

Corollaire de conception : un service **absent** doit produire un échec bruyant
en aval, pas un zéro résultat.

## 9. Ce qui reste faux ou incomplet

- **`Catalog::search` est un monolithe de 409 lignes** dans un `catalog.rs` de
  6 373. Le chemin composable existe en parallèle et **n'est pas emprunté** :
  deux chemins à maintenir, et chaque correction de cette semaine a dû être
  portée aux deux.
- **`create` / `update` / `delete` mentent** : ils poussent dans une file en
  mémoire et rendent. Seul `ingest_entities` est honnête. C'est le sujet du
  [01](01-les-objectifs-et-leur-ordre.md).
- **La troncature d'embedding est silencieuse** — le trait `Embedder` n'expose ni
  fenêtre ni compte de jetons, le budget est en caractères.
- **`crate::acces` n'a aucun appelant en production**, et son appelant naturel
  manque : rien ne permet d'ouvrir un catalogue **en lecture**.
- **`fusion.rs` est mort et public** ; `special_ops`, `title_boost` et
  `content_boost` sont acceptés et jamais appliqués.
- **Le miroir Rust de `search_base`** — 70 lignes de test qui recopient le
  gabarit. Le **générer serait pire** : il prouverait que l'analyseur est
  d'accord avec l'analyseur.
