# L'architecture actuelle

**6 septembre 2026, 11 h 43.** Les **contrats**, pas les fichiers. Un fichier se
retrouve par `grep` ; un contrat, non.

## Les documents du même genre, avant celui-ci

| | quoi |
|---|---|
| [`5-septembre-2026-16h13/03`](../5-septembre-2026-16h13/03-l-architecture-actuelle.md) | la version du 5 septembre — celle-ci en est la suite directe |
| [`3-septembre-2026-17h50/03`](../3-septembre-2026-17h50/03-architecture.md) | le second backend, les cinq organes |
| [`30-aout-2026-07h00/03`](../30-aout-2026-07h00/03-architecture-ce-qui-a-change.md) | ce qui avait changé fin août |
| [`29-aout-2026-12h24/02`](../29-aout-2026-12h24/02-architecture.md) | le démon et la fin du tout-synchrone |
| [`27-aout-2026-13h01/12`](../27-aout-2026-13h01/12-architecture.md) | version d'août, plus détaillée sur le dataflow |
| [`23-aout-2026-20h33/30`](../23-aout-2026-20h33/30-passation-architecture-et-intention.md) | la passation d'origine : l'**intention**, qui n'a pas bougé |
| [`vision_roadmap_09_2026/15`](../vision_roadmap_09_2026/15-le-moteur-cesse-d-etre-mono-backend.md) | pourquoi le moteur n'est plus mono-backend |

## 1. Les cinq organes d'un backend

Un backend, c'est cinq choses. Deux les fournissent toutes : rag3db natif et
PostgreSQL/pgvector.

| organe | ce qu'il fait |
|---|---|
| `DbConnection` | exécuter, paramétrer, rendre des lignes |
| `SchemaDialect` | dire le DDL et les requêtes dans sa langue |
| `SearchBackend` | servir le plein texte et le vecteur quand il sait |
| `BlobStore` | stocker les index lucivy et sparse |
| `CheckpointStore` | reprendre après incident |

La règle qui a coûté une semaine et qu'il ne faut pas défaire : **c'est le
dialecte qui dit ce qu'il sait offrir**, jamais son nom.
`nouveau_magasin_de_checkpoints(conn) -> Option<Arc<dyn CheckpointStore>>`.

## 2. Les quatre disponibilités — **le contrat central depuis aujourd'hui**

[`src/disponibilite.rs`](../../src/disponibilite.rs) porte la doc de référence.
L'essentiel :

**Un ensemble, pas un niveau.** `data` précède tout — on n'indexe pas ce qui
n'est pas posé — mais `textsearch`, `sparse` et `dense` sont des **frères
parallèles** qui se commitent séparément. On n'attend pas « jusqu'au niveau N »,
on attend **les signaux qu'on nomme**.

**Le même vocabulaire des deux côtés :**

| | qui parle | ce qu'il dit |
|---|---|---|
| écriture | `FlushResult.rendu_pret` | *ce qu'il a rendu prêt* |
| lecture | `SearchOptions.exige` | *ce dont il a besoin d'être prêt* |

**La garantie est conservatrice.** On ne dit jamais « prêt » quand ça ne l'est
pas ; on attend parfois plus que demandé. La table en tête du module dit
exactement où, et il faut la lire avant de raisonner dessus.

**Un seul arbitre**, `SearchOptions::ce_qui_doit_etre_pret`. `Consistency`
survit comme raccourci nommé mais ne porte **plus d'état** — il le traduit
(`en_disponibilites`). Deux façons de dire la même chose ne divergent que si
deux endroits les lisent.

**L'attente des autres processus est une question séparée** (`attendre_les_autres`) :
on ne peut pas vider la file d'un autre, seulement attendre qu'il la vide.

## 3. La coupe, et la dette qui vit dans la base

```
… → chunk_insert  │  embed
└─ data + textsearch ┘  └─ dense/sparse dus
```

Les nœuds d'embarquement sont des **feuilles** du graphe — sauf `rechunk_embed`,
dont le flush FTS attend la fin ; quand on coupe, ce flush prend son déclencheur
en amont, sinon l'index plein texte resterait dans le tampon.

**Ce qui n'est pas embarqué n'est pas gardé en mémoire.** C'est une ligne de
base : un chunk dont `_embed_hash` ou `_sparse_hash` est vide. Donc :

- ça survit à un processus qui meurt ;
- ça s'interroge — `SchemaDialect::count_marqueur_manquant`,
  `select_chunks_sans_marqueur` ;
- `Catalog::embarquer_le_retard(exige, limite)` le solde, borné par table.

`Catalog::peut_devoir_un_embarquement` est un **indice**, pas une vérité : il
évite un balayage de table dans le cas nominal. Si un autre processus crée de la
dette, la recherche le dit quand même.

## 4. Les deux marqueurs d'embarquement — schéma v3

`_embed_hash` (dense) et `_sparse_hash` (sparse), tous deux vides à la naissance
d'un chunk. **Un marqueur unique ne peut pas répondre à deux questions**, et
c'est ce qui produisait le défaut : sur le chemin dual, l'écriture dense
marquait pour les deux, donc un vecteur sparse perdu restait annoncé écrit.

Quatre endroits suivent ces marqueurs, et ils doivent rester d'accord :
l'écriture (chacun pose le sien **après** son vecteur), le filtre de
réembarquement (chaque signal juge par le sien, le dual repasse si l'un des deux
manque), `embed_check_hashes` (trois colonnes, **aucun filtre**), et l'undo (les
deux à zéro).

**L'ordre compte** : le marqueur se pose *après* le vecteur. `embed_get_offset`
lit le décalage sans rien poser ; `embed_set_hash_returning_offset` et
`batch_update_fields` marquent ensuite.

## 5. Le contrat de décalage, et pourquoi il ne bouge pas

Le plein texte et le sparse rendent des **décalages de ligne**, résolus en
chunks. Ce contrat survit à la coupe pour une raison structurelle : **aucun
décalage n'est transporté d'une étape à l'autre**. Les quatre sites qui écrivent
dans un index sparse relisent le décalage par un `MATCH` sur `_uuid` au moment
de s'en servir. Le moment de l'embarquement n'a donc aucune influence sur la
correspondance.

Le piège adjacent, toujours vrai : l'identifiant est une **chaîne** côté rag3db
et un **entier** (`_row_id`) côté PostgreSQL. `InternalNodeId::parse` accepte les
deux.

## 6. Les deux scores, qui ne peuvent pas être le même nombre

- Le score de **classement** est relatif et dépend du moteur — BM25 non borné
  sur lucivy.
- Le score de **confiance** doit être absolu : Jaro-Winkler recalculé sur le
  texte rendu, `SEUIL_CONFIANCE = 0.88`.

Poids de fusion **mesurés**, pas devinés : `POIDS_TRIGRAMME = 0.20`,
`POIDS_JARO = 0.80`. Le trigramme seul a une marge **négative** — bon signal de
rappel, mauvais signal de classement.

`PLANCHER_RAPPEL = 0.3` : le défaut de `pg_trgm` (0,6) faisait rendre zéro ligne
à une requête à deux fautes, et un mur de rappel ressemble à « ça n'existe pas ».

## 7. Ce qui remonte à l'appelant, et par quel canal

C'est le point le plus corrigé de la semaine. Trois canaux, et un seul arrive
jusqu'à un agent :

| canal | va où |
|---|---|
| `ctx.warn` dans un nœud | le rapport d'exécution — **nulle part** pour un agent |
| `SearchMeta.warnings` | recopié par `RenderResultsNode` dans la fiche rendue — **arrive** |
| `FlushResult.warnings` | rendu à l'appelant du drain — **arrive** |

D'où les arêtes `source|bm25|vector -->|meta| render` dans `search_base.mmd`, et
`ramasser_les_avertissements` côté écriture. **Ce qui n'est pas remontable n'est
pas dit** : ajouter un `ctx.warn` sans canal, c'est ajouter un silence.

`merge_port_values` sait fondre deux `SearchMeta` — avertissements concaténés
sans doublon, comptes additionnés, `partial` par `|=`.

## 8. Les deux moteurs de texte, et l'option

`MoteurTexte::{Lucivy, Natif, Auto}`. Sur le chemin **natif**, l'index vit avec
les données : rien à ouvrir, et `data` et `textsearch` arrivent au même instant
par construction. Sur lucivy, l'index est un blob et se commite séparément.

Le service `plein_texte_natif` porte cette information aux nœuds — un nœud ne
peut pas la demander au catalogue, dont le verrou est déjà tenu quand le graphe
s'exécute.

## 9. Le motif « une option, pas un remplacement »

Quatre fois maintenant, et c'est devenu la forme par défaut de ce moteur :

| | l'option | le défaut |
|---|---|---|
| moteur de texte | `MoteurTexte` | `Auto` |
| accès lecteur | `Acces::{Direct, Demon, Auto}` | `Auto`, **jamais de repli silencieux** |
| régime | `Regime::{Confort, Plein}` | `confort` |
| portée d'un lot | `ingest_entities_jusqu_a` | `ingest_entities`, complet |

La règle commune : **le choix se déclare, il ne se devine pas**, et il se **dit**
dans ce que l'appel rend.

## 10. Ce qui reste faux

- **`Catalog::search` fait 409 lignes** et n'emprunte pas le chemin composable.
  Chaque correction doit être portée deux fois.
- **`FlushResult.failed` vaut `0` en dur** au succès : aucun canal d'échec par
  enregistrement.
- **`PendingWork` est une file, `drain` une barrière.** Une recherche sur A
  attend les écritures sur B. `Eventual` flushe toute la file, donc **seul
  `AUCUNE` respecte aujourd'hui l'invariant de non-couplage**.
- **`crate::acces` et `KBSearchNode` n'ont aucun appelant de production.**
- **`fusion.rs` non plus**, et il est supplanté ; son en-tête le dit.
- **Le miroir Rust de `search_base`** est écrit deux fois, avec un test de
  parité — `GraphTool::new_avec_herites` est un colmatage, pas la sortie.
- **La taille de chunk n'est pas dérivée de la limite du modèle.** Le MiniLM
  multilingue tronque à **128 jetons** ; c'est compté et dit
  (`Embedder::troncatures`), mais pas évité.

## 11. Après la passe fondations (soirée du 6) — ce que la §10 ne dit plus juste

- **`Catalog::search` fait toujours 417 lignes**, mais il n'est plus sur le
  chemin du produit : `Catalog::rechercher` lance le gabarit `search_base`
  sur le catalogue, et `KBSearchNode` passe par lui. Ses appelants sont des
  tests ; son retrait est une décision de surface (bilan, §« ce qui reste »).
- **`FlushResult.failed` ne vaut plus `0` en dur** : le canal d'échecs par
  groupe le compte, et `rendu_pret` perd la disponibilité qu'un échec dérivé
  a perdue.
- **`PendingWork` n'est plus une barrière** : un drain emporte la fermeture
  d'une cible et remet le reste en file. L'invariant tient sur les trois
  niveaux, `AUCUNE` compris — et à travers la frontière du processus, par la
  marque par niveau et par table.
- **`crate::acces` a son appelant** (`Catalog::ouvrir_en_lecture`).
  **`fusion.rs` n'existe plus.** `KBSearchNode` est devenu un alias du lanceur.
- **Le miroir Rust de `search_base`** compte neuf nœuds et vingt arêtes ;
  c'est toujours un miroir, et toujours une dette.
- Ce qui reste faux est dans le bilan, avec sa raison.
