# L'architecture au 3 septembre 2026

Ce que le moteur est aujourd'hui, avec ce qui a bougé cette semaine. Écrit pour
qu'une session compressée retrouve **les contrats**, pas la liste des fichiers.

## 1. Ce qu'un backend doit fournir — quatre organes, pas trois

Le `Catalog` est une couche au-dessus d'une base, et depuis aujourd'hui
au-dessus de **plusieurs**. Ce qu'il réclame d'un backend :

| organe | trait | ce qu'il fait |
|---|---|---|
| la connexion | `DbConnection` | `execute`, `execute_with_params` — **synchrone** |
| le dialecte | `SchemaDialect` | ~60 méthodes de DDL/DML : l'**intention** rendue dans le langage du backend |
| la recherche | `SearchBackend` | vecteur, résolution de décalages, enrichissement — et **facultativement** le plein texte |
| le magasin de blobs | `BlobStore` | où vivent les index lucivy et sparse |

Un cinquième est facultatif : le **magasin de checkpoints**. `CypherCheckpointStore`
porte son dialecte dans son nom, donc `initialize()` ne le monte que si
`dialect.speaks_cypher()`, et **le dit par un `Warning`** sinon plutôt que de
démarrer amputé en silence.

**Le principe qui tient tout** : `speaks_cypher()` existe parce que deux organes
sont encore écrits en Cypher en dur. Un dialecte qui ne se prononce pas ne
change rien (défaut `true`) ; PostgreSQL dit `false` et fournit les siens.

## 2. Le plein texte : deux moteurs, une option

```rust
MoteurTexte::{ Auto, Lucivy, Natif }   // Auto par défaut : on demande au backend
```

Ce n'est **pas** un remplacement de lucivy. `Auto` interroge
`SearchBackend::sert_le_plein_texte()` ; les deux autres valeurs forcent. lucivy
reste choisissable sur n'importe quel backend et redeviendra le défaut quand
elle sera plus légère.

**Deux étages, et c'est le point.**

```
   la base fait le RAPPEL          nous faisons l'ORDRE
   index GIN trigramme             Jaro-Winkler (src/jaro.rs)
   large, indexé, rapide           sur quelques dizaines de candidats
```

Sur PostgreSQL : `<%` / `word_similarity` et non `%` — l'opérateur simple compare
deux chaînes *entières*, donc la longueur écrase le score. Et l'index porte sur
la **table de chunks**, pas sur l'entité : sans spans de surlignage, l'unité
indexée doit être celle qu'on veut rendre. L'extrait devient gratuit, et les deux
signaux classent les mêmes objets.

Les accents sont normalisés **des deux côtés** par `rag3weaver.sans_accents`, un
enrobage IMMUTABLE de `unaccent` — la fonction nue est STABLE, donc interdite
dans une expression d'index.

## 3. Le contrat de décalage, et où il est nécessaire

Toute la résolution de recherche est bâtie sur des **décalages de ligne en u64**.
kuzu a `OFFSET(id(n))`, PostgreSQL a reçu `_row_id BIGSERIAL`.

La bonne formulation du contrat n'est **pas** « un backend rend des décalages »
mais :

> **un décalage est nécessaire là où un index Rust vit à côté des données.**

C'est ce qui le rend obligatoire pour lucivy et le sparse, et facultatif pour un
backend qui sert lui-même le texte et le vecteur. Neo4j sera le premier à le
montrer — texte Lucene et vecteur natifs, mais **le sparse restera à nous**.

## 4. Le cloisonnement multi-locataire

`Scope { org, project }`, matérialisé par les colonnes `_org` / `_project` sur
toute table de données, et par un index `(_org, _project)`.

**La cellule est un paramètre explicite et obligatoire de `text_search`**, pas
une dérivation d'un filtre construit ailleurs : une frontière de locataire ne
doit pas dépendre de la présence d'un `WHERE`. C'est la leçon de la fuite
trouvée aujourd'hui, sur les deux chemins de recherche à la fois.

## 5. Le motif des services — et pourquoi ce n'est pas cosmétique

Les nœuds de dataflow reçoivent ce dont ils ont besoin par un `ServiceRegistry`,
jamais en verrouillant le catalogue :

```rust
services.register("fts_handles", …);
services.register("texte_natif", …);   // seulement si c'est le chemin choisi
services.register("cellule", …);        // seulement si multi_cell
```

**`Catalog::search()` tient déjà son verrou quand le graphe s'exécute.** Un
`lock()` depuis un nœud rend donc `None` — c'est-à-dire un repli silencieux. Ce
défaut a été trouvé deux fois aujourd'hui, sous deux formes : un plein texte qui
retombait sur lucivy, et un cloisonnement qui disparaissait.

Corollaire de conception : un service **absent** doit produire un échec bruyant
en aval. Si `texte_natif` manque, on n'a pas ouvert d'index lucivy non plus, donc
la recherche échoue avec « aucun index FTS ouvert » — le défaut de câblage se
voit au lieu de se déguiser en zéro résultat.

## 6. La cohérence, et sa frontière

```rust
Consistency::Immediate  // ne pas attendre
Consistency::Eventual   // attendre les insertions en attente  ← défaut
Consistency::Strict     // vider toute la file avant de chercher
```

**Elle est intra-processus.** La file vit dans `Catalog::pending`, en mémoire :
`Strict` appelle `self.drain()`, et `has_pending()` ne teste que
`!self.pending.is_empty()`. Un lecteur d'un autre processus a son propre
catalogue, dont la file est vide — il demande `Strict` et obtient `Immediate`.

Ça n'a jamais été garanti par le verrou de fichier, qui rendait l'accès
concurrent *impossible* et non *ordonné*. Ça devient visible maintenant que la
frontière est franchissable. Voir le point 1 du
[rapport de session](01-rapport-de-session.md).

## 7. Ce qui a bougé cette semaine, en une liste

- `src/jaro.rs` — **nouveau**. Jaro, Jaro-Winkler, comparaison mot à mot, repli
  des accents.
- `SearchBackend` gagne `TextHit`, `text_search`, `sert_le_plein_texte`.
- `SchemaDialect` gagne `speaks_cypher`, `upsert_scope_node`,
  `secondary_indexes`, `relation_indexes`, `blob_store_indexes`,
  `text_search_indexes`.
- `Catalog` gagne `set_blob_store`, `set_moteur_texte`, `plein_texte_natif`,
  `poser_index`.
- `search.rs` gagne `search_texte_natif`, qui **ne passe pas** par
  `finish_bm25_chunked` : celle-ci résout les décalages en Cypher écrit en dur.
- Le cœur C++ perd deux en-têtes morts (64 lignes) et gagne le report de Vela
  (3 lignes) — voir [`docs/3-septembre-2026-14h42/`](../../../docs/3-septembre-2026-14h42/).

## 8. Ce qui reste faux ou incomplet dans l'architecture

- **`finish_bm25_chunked` résout les décalages en Cypher en dur.** Elle est donc
  inutilisable hors rag3db, d'où le chemin séparé. À unifier un jour par le
  backend.
- **Deux magasins parlent Cypher en dur** : checkpoints et blobs. Ils devraient
  être dialectés ou fondus.
- **`PostgresBlobStore` existait sans être atteignable** — il demandait un
  `Arc<dyn SyncDbConnection>` là où le catalogue ne sait rendre qu'un
  `Arc<dyn DbConnection>`. Corrigé, mais c'est le symptôme d'un code écrit sans
  jamais être appelé.
- **Le filtre utilisateur n'est pas câblé sur le chemin texte natif** : seule la
  cellule l'est.
