# La première ingestion en masse

**6 septembre 2026, 22h21.** Chantier « tout ce qui n'est ni le modèle ni le
plein texte » sur le chemin d'indexation, au feu vert de Lucie :

> oki on y va pour optimiser donc tout ce qui est mentionné en dehors
> embedding et fts du coup

Référence : `e2e_mesure_ingestion_code` sur le cœur C++ de rag3db
(`RAG3WEAVER_MESURE_RACINE=…/rag3db/src`, granite-107m par le démon,
1 642 fichiers, 18 140 scopes, 20 132 chunks, 16 396 symboles, 380 000
arêtes). Point de départ : **51,5 s**, dont ~19 s de modèle (carte à 99 %) et
~28 s hors modèle ([doc 19h57 §8](../6-septembre-2026-19h57/01-la-decoupe-des-scopes-et-la-vue-par-parent.md)).

## 1. Ce que la cartographie a établi

- **Un seul écrivain de nœuds**, `InsertRecordNode` : un `UNWIND … MERGE`
  par groupe `(table, colonnes)`, sans tranche, qui rend `ID(n)` pour le
  cache d'identifiants et l'indexation lucivy ligne à ligne.
- **Les vecteurs sont posés après la ligne** : `EmbedNode` relit les
  marqueurs (`embed_check_hashes`), embarque, puis `SET n.embedding` par lot
  de modèle. 7,7 s d'écritures sur les 20 132 chunks, en partie cachées par
  le pipeline.
- **Deux `batch_select` avant chaque ingestion** (`split_unchanged`), même
  sur une base vide.
- **`MarquerDecoupeNode`** : un `SET _chunked_hash` par parent après les
  liens — 825 ms sur 18 140 scopes, 645 ms sur 16 396 symboles qui n'ont
  pas de chunks.
- **Le moteur sait charger une table de nœuds d'un bloc** : `COPY table
  (colonnes) FROM 'fichier.csv'`, sur une table vide ou non, avec les
  vecteurs `FLOAT[n]` en cellule `"[a,b,c]"` — c'est le chargement qu'utilise
  l'extension vecteur elle-même. Une clé en double refuse tout le fichier.
- **Le runtime sérialise chaque lot en JSON deux fois** — en sortie du nœud
  (`NodeCompleted.outputs`) puis en entrée du suivant (`NodeStarted.inputs`)
  — pour des instantanés dont seul le résumé d'une *requête* lit le JSON.
  Sur un drain de 225 000 liens, 3,1 s hors de tout nœud.

## 2. Ce qui est fait

### La première ingestion (`Catalog::premiere_ingestion_possible`)

Quand le moteur a un `COPY`, que la table de l'entité et celle de ses chunks
sont vides, que l'entité n'alimente pas une base de connaissances, et que
personne n'a posé `RAG3WEAVER_INGESTION_LIGNE_A_LIGNE` :

```
insert(COPY) → chunk → embed(Enrich) → chunk_insert(COPY, vecteurs compris) → chunk_link(COPY)
insert.done → flush_fts
```

- `split_unchanged` est sauté : il n'y a rien à relire.
- `InsertRecordNode::with_mode(InsertMode::Copy)` : dédoublonnage par uuid
  (la dernière occurrence gagne, comme `MERGE`), un CSV par groupe de
  colonnes, `COPY`, puis **une relecture des identifiants** seulement quand
  lucivy ou le handle sparse en ont besoin (`SchemaDialect::select_node_ids`).
  Refusé par le moteur ou par une valeur qu'on ne sait pas écrire (liste de
  chaînes, carte, NULL), le groupe repasse par le `MERGE` avec un
  avertissement dans `FlushResult.warnings`.
- `EmbedNode::with_mode(EmbedMode::Enrich)` : pas de vérification de
  marqueurs, les vecteurs s'attachent aux enregistrements
  (`EntityRecord::vectors`, un `Vec<f32>` et non une liste de
  `CypherValue` : 4 Ko contre 30), `_embed_hash` et `_sparse_hash` dans
  `data`, et le port `embedded` mène à l'insertion. Le sparse est inséré
  dans son handle par `InsertRecordNode`, au décalage relu.
- `_chunked_hash = _content_hash` dans la ligne du parent ; pas de
  `MarquerDecoupeNode`. Si le graphe meurt entre le parent et ses chunks,
  la table n'est plus vide et l'ingestion suivante voit l'absence de chunks
  (`split_unchanged`) — le contrat tient sans la marque.
- Dès la seconde ingestion (table non vide), le chemin de toujours reprend,
  à l'identique.

### Le contrat CSV, éprouvé (`tests/e2e_chemin_de_masse.rs`)

- lecteur **séquentiel** (`parallel=false`) : le parallèle refuse un saut de
  ligne entre guillemets, et du code en a toujours ;
- `null_strings=['__rag3weaver_null__']` : le moteur lit `""` comme un NULL,
  or `_embed_hash` à la naissance d'un chunk est une chaîne vide, et la lire
  NULL la ferait réécrire. Le NULL n'est jamais écrit sur le chemin de masse
  (son sens dépend du type de colonne, que l'écrivain ne connaît pas) ;
- guillemet doublé, sauts de ligne, virgules, barres obliques : la chaîne
  revient telle quelle ;
- **un fichier par graphe** (`fichier_csv`) : le processus, un compteur et
  l'instant. Deux tests dans le même processus posaient la même table à la
  même milliseconde et se volaient le fichier — un lot de trois lignes en
  laissait une. Le `COPY` des liens d'hier avait le même défaut, masqué par
  son seuil de 2 000.

### Le runtime

`PortSnapshot::from_port` : un `BatchPayload` donne son type et son compte,
pas son JSON. Le résumé d'une requête garde le sien.

### Le profil

`RAG3WEAVER_INGEST_PROFILE=1` ventile désormais le drain lui-même
(`[drain-profile] … drain/extraction du lot | construction du graphe |
exécution | marque et avertissements`), en plus des nœuds. C'est ce qui a
montré les secondes hors nœuds.

## 3. Les mesures

| étape | avant | pas 1 (COPY, vecteurs avec la ligne) | pas 2 (marque, instantanés) |
|---|---|---|---|
| `insert` Scope (18 140) | 2 511 ms | 826 ms | 836 ms |
| `chunk_insert` Scope_Chunk (20 132) | 2 007 ms + 7 745 ms de `SET` | 2 401 ms, vecteurs compris | 2 434 ms |
| `embed` Scope (mur, 2 fils) | 23 194 ms | 19 435 ms (`write_ms` 3) | 19 496 ms |
| `marquer_decoupe` (×3) | 1 479 ms | 1 507 ms | 0 |
| drain des relations (exécution / nœud liens) | — | 3 685 / 1 753 ms | 3 139 / 1 756 ms |
| drain des rendez-vous (exécution / nœud liens) | — | 4 623 / 1 547 ms | 3 696 / 1 521 ms |
| drain des arêtes résolues (exécution / nœud liens) | — | 1 428 / 605 ms | 1 159 / 581 ms |
| `symboles/ingestion` | 2 240 ms | 1 938 ms | 1 172 ms |
| symboles, en tout | 9 461 ms | 9 248 ms | 7 288 ms |
| **total** | **51,5 s** | **47,4 s** | **42,5 s** |

Pas 3, le checkpoint en fichiers binaires (§3 bis) : **36,4 s**. Le
checkpoint coûte encore ~2,1 s (la sérialisation MessagePack des lots, sur le
fil du graphe : 320 ms pour les trois sorties de `chunk`, 364 ms pour les
225 000 liens à l'entrée du drain des rendez-vous ; l'écriture, elle, est
sur le fil de fond). Symboles 9,5 → 5,0 s, relations 4,4 → 3,0 s.

Ce que dit la colonne « pas 1 » : le modèle est maintenant le mur (19,4 s
d'`embed` pour 38,6 s de modèle sur deux fils, la carte à 99 %), et le
`COPY` des chunks avec leurs vecteurs coûte 2,4 s — c'est le lecteur CSV
séquentiel qui parse 85 Mo de flottants. Deux pistes pour lui, non prises :
des flottants plus courts (`{:.5}` : −25 % de fichier, au prix d'un arrondi
à éprouver par l'écart absolu max, pas par un cosinus), ou le Parquet, que
le moteur lit en parallèle mais qui coûte une dépendance.

## 3 bis. Le checkpoint : des fichiers binaires, un fil d'écriture

Le profil du pas 2 a montré le poste suivant, et il n'était dans aucun
nœud : sur rag3db, `initialize()` monte un magasin de checkpoints, donc
chaque graphe passe par `execute_with_checkpoint`, qui **sérialisait en JSON
le lot d'entrée entier et la sortie de chaque nœud, et les écrivait dans une
ligne de la base** (`inputs_json`, `output_ports`, `undo_json`). Mesuré :

| | sérialisation | écriture en base |
|---|---|---|
| départ du drain des rendez-vous (225 000 liens) | — | 1 855 ms |
| départ de l'ingestion des scopes (18 140) | — | 591 ms |
| `chunk` (20 132 chunks + 20 132 liens + 18 140 parents) | 707 ms | 613 ms |
| `insert`, `embed`, `chunk_insert` (18–20 k chacun) | ~300 ms | ~280 ms |
| **en tout** | | **8,5 s sur 42,7** |

Une première idée — un « checkpoint léger » qui ne garde plus les lots
au-delà de 5 000 enregistrements — a été écrite puis **retirée** : Lucie a
raison, un checkpoint qui ne reprend plus n'est pas un checkpoint. Ce qui
est fait à la place, sur sa piste :

- **les lots en octets** (`rmp-serde`, MessagePack à champs nommés :
  auto-décrit comme le JSON, sans l'échappement) — `CheckpointPortValue`
  porte `data_bytes` le temps de traverser le magasin, puis `data_file` ;
  `data_json` reste pour les valeurs directes (une requête) et les
  checkpoints d'avant ;
- **des fichiers horodatés, un par lot**, sous
  `<dossier>/<exécution>/<horodatage>-<nœud>-<port>.{entree,sortie}.bin`
  (`CatalogConfig.checkpoint_dir`, défaut sous le dossier temporaire) ; la
  base ne garde que l'état des nœuds et les chemins ; les gros contextes
  d'undo (> 64 Ko) suivent, en `@file:` ;
- **un fil d'écriture de fond** (`checkpoint_store::Spiller`) : le graphe
  tend les octets, le fil écrit, `mark_completed` et `mark_failed`
  attendent qu'il ait tout posé, et les sorties d'une exécution finie sont
  effacées avec ses `output_ports` ;
- **un fichier absent n'est pas un nœud fait** : à la reprise, un nœud dont
  la sortie ne se relit pas est rejoué (ils sont idempotents) ; une entrée
  initiale qui ne se relit pas refuse la reprise en le disant.

À venir, une fois ce chemin solide, à la demande de Lucie : trois modes —
*complet* (celui-ci), *opérations* (les entrées seulement, la reprise rejoue
tout depuis le début), *aucun* — au niveau du catalogue et, pour les
ingestions, par entité.

## 4. Ce qui reste, et pourquoi

| reste | la raison vraie |
|---|---|
| **Le runtime hors nœuds, encore ~4 s sur trois drains** (1,4 + 2,2 + 0,6 s d'écart entre `drain/exécution` et le nœud de liens) | les instantanés JSON n'étaient qu'une des causes ; la suivante se cherche par phase du runtime (préparation, niveau, rangement), pas encore instrumentée |
| **Le `COPY` des chunks, 2,4 s** | le lecteur CSV séquentiel ; Parquet ou flottants courts, à mesurer |
| **`symboles/mise en file`, 0,8 s** | 224 000 `link_jusqu_a` à 3,5 µs : un `RelationRef` (canal) par lien. Un `link_batch` sans canal ferait mieux, mais c'est une API de plus |
| **L'existence des bouts avant un `COPY` de liens, ~1,1 s** | `select_by_uuids` par tranches sur les deux tables. Un ensemble en mémoire des uuids posés dans la session l'éviterait ; c'est de l'état de plus dans le catalogue |
| **`flush_fts`, 0,6 s** | plein texte, hors périmètre |
| **Le modèle, 19 s** | la carte à 99 % ; c'est le chantier de la session moteur (fp16, fused attention) |

## 5. Les bases de connaissances, en passant

Lucie a demandé si les KB ont un vrai intérêt. Réponse donnée en séance,
consignée ici : **l'idée oui, l'implémentation non, et ça aurait dû être un
gabarit.** Une KB est un document composé de plusieurs entités, indexé
comme un tout — un produit avec ses avis, un ticket avec son fil — et
l'embarquement du texte composé capture ce que la vue par parent (composée
à la requête) ne capture pas. Mais c'est un second pipeline (`KBGather`,
`KBUpdate`, `KBChunk`, `KBEmbed`) qui prend chaque amélioration en retard :
pas de court-circuit de l'inchangé, pas de chemin de masse, une dette
d'agrégat qui n'est pas en base (C5 bis). Ce que ça devrait être : une
entité dérivée décrite dans le schéma, dont le contenu est le rendu d'une
traversée de relations par un gabarit jinja, qui passe par
`insert → chunk → embed` comme les autres ; la seule chose propre aux KB
qui reste est le suivi de dépendance, une dette en base sur le patron de C4
et C5. À replier après ce chantier.

## Comment vérifier

```sh
./run_e2e.sh --test e2e_chemin_de_masse        # le contrat CSV, la première puis la seconde ingestion
RAG3WEAVER_MESURE_RACINE=/home/lucied/git_workspaces/rag3db/src RAG3WEAVER_MESURE_MODELE=granite-107m \
  ./run_e2e.sh --test e2e_mesure_ingestion_code # le cœur C++ ; lire [ingest-profile] et [drain-profile]
RAG3WEAVER_INGESTION_LIGNE_A_LIGNE=1 …           # le même, par le chemin de toujours, pour comparer
```
