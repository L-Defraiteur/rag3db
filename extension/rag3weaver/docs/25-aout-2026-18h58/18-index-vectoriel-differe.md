# 18 — L'index vectoriel : ce qui coûte, et ce qu'on a le droit de différer

26 août 2026, midi. Le profil d'ingestion a désigné le coupable, et il n'est
pas celui qu'on soupçonnait. Enquête faite ensuite sur ce qu'on casserait
en différant.

## 1. La mesure

`RAG3WEAVER_INGEST_PROFILE=1`, sur `src/dataflow` (27 fichiers, 1 387
scopes) :

| Nœud | Durée | Volume |
|---|---|---|
| **`embed`** | **13 087 ms** | 4 027 vecteurs |
| `chunk_insert` | 540 ms | 4 027 lignes |
| `flush_fts` | 314 ms | — |
| `insert` | 303 ms | 1 387 lignes |
| `chunk_link` | 233 ms | 4 027 liens |
| `chunk` | 12 ms | — |
| phase `Symbol` entière | 255 ms | 3 275 entités |

**Quatre-vingt-dix pour cent de l'ingestion est dans `embed`** — et le test
tourne sur `HashEmbedder`, donc ce n'est pas du calcul de modèle. C'est
l'insertion dans l'index HNSW : écrire la colonne `embedding` déclenche une
mise à jour d'index synchrone, ~3,2 ms par vecteur.

Mes deux soupçons de départ — les offsets ligne par ligne, l'indexation
plein texte document par document — étaient **faux** : quelques centaines
de millisecondes chacun.

Corollaire immédiat, qui corrige ce qu'on croyait du chantier `Symbol` :
sur ses 12,1 s, environ **10,6 s étaient le vecteur** et **0,6 s les
chunks**. Ce n'était pas d'avoir des chunks qui coûtait, c'était d'avoir un
vecteur par chunk.

## 2. Le contrat de cohérence est déclaré, pas tenu

`Consistency::{Immediate, Eventual, Strict}` promet trois comportements.
Il n'est consulté **qu'à un seul endroit** (`catalog.rs:3305-3316`), et
aucune variante ne calcule d'embedding ni ne touche l'index vectoriel.
Le commentaire « search immediately, even if embeddings are pending »
décrit un mécanisme qui n'existe pas.

Pire, et c'est un **défaut préexistant à signaler** : `Eventual` — le
défaut — appelle `flush_insertions`, qui construit un graphe à un seul
nœud (`InsertRecordNode`), sans embedder, sans handles, sans découpage…
et **vide `pending.entities`** au passage. Les lignes sont écrites, jamais
indexées, et un `drain()` ultérieur ne peut plus les rattraper.

Ce qui est réellement tenu aujourd'hui n'est donc pas un contrat, c'est une
**propriété émergente** : `SET n.embedding` traverse la transaction, le
fork met l'index à jour dans la foulée. Un seul test en dépend, et il le
fait en `Consistency::Immediate` — c'est-à-dire en court-circuitant le
contrat.

## 3. Le différé existe déjà — pour le plein texte

`FlushNode` commite les handles lucivy en fin de graphe. Ce n'est pas une
optimisation : sans `commit()`, le lecteur ne voit rien. **Le plein texte
est donc déjà un index différé**, avec un vidage explicite… et incomplet :
un `drain()` qui ne contient que des `create()` d'entités simples n'ajoute
aucun `FlushNode`. Les documents restent invisibles jusqu'au drain suivant
ou à l'arrêt.

Deux autres précédents : `BufferedBlobStore`, avec ses deux invariants
écrits — **lecture de ses propres écritures**, **les suppressions ne sont
pas tamponnées** — et son jeu complet de frontières de vidage ; et
`SparseCommitNode`, nœud écrit, enregistré, **jamais câblé**.

## 4. Ce qu'on casserait vraiment

**Grave**

1. **La reprise après interruption.** Le runtime saute les nœuds
   `Completed` et réinjecte leurs sorties. Un vidage vectoriel porté par un
   tampon en mémoire ne serait **jamais rejoué**, et `_embed_hash` déjà
   posé empêcherait tout rattrapage — `EmbedNode` saute les lignes dont le
   hash correspond. **Corruption silencieuse et permanente.** C'est le seul
   point de cette gravité.
2. **Il n'y a aucun repli par balayage.** `search_vector_bruteforce`
   existe, fonctionne, et n'a **zéro appelant**. Un index en retard rend
   donc moins de résultats sans le dire — `partial` compte des opérations
   CRUD, pas des vecteurs manquants.
3. **Ne jamais vider par offset interne.** Les offsets sont réattribués
   après suppression ; un vidage tardif ressusciterait un vecteur sous
   l'offset d'une autre ligne. C'est le mode d'échec déjà documenté côté
   plein texte. Le vidage doit **rejouer `embed_set`** : le `MATCH` ne
   trouve rien sur une ligne supprimée, et l'annulation devient
   gratuitement correcte.

**Moyen** : `Strict` doit déclencher le vidage (sinon toute la suite
`e2e_idempotent_registration` tombe) ; `ingest_entities` est une frontière
de durabilité déclarée, le vidage doit s'y greffer ; `reindex` reconstruit
le plein texte mais laisserait le vecteur en retard tout en effaçant son
drapeau.

**Faible** : `EmbedNode::undo` annule `_embed_hash` sans effacer
`embedding` — déjà incohérent aujourd'hui.

**Et une inversion instructive** : sur le chemin d'annulation, le vecteur
est aujourd'hui **plus correct que le plein texte**.
`InsertRecordNode::undo` supprime les lignes sans rien dire à lucivy, qui
garde donc des documents fantômes. La désindexation symétrique manque déjà,
côté FTS.

## 5. La brique la moins chère est là, inutilisée

`CREATE_VECTOR_INDEX` **sur une table déjà remplie est son mode nominal** :
il lit la cardinalité et construit par balayage complet. `DROP_VECTOR_INDEX`
existe dans le fork, testé — et **n'est référencé nulle part** dans
rag3weaver. Or l'index est créé à `register_entity`, sur une table **vide**,
c'est-à-dire au pire moment possible du point de vue du coût.

Donc la séquence **détruire → charger → construire** est réalisable
aujourd'hui, sans rien différer, sans toucher au contrat, sans risque de
reprise. Elle ne demande qu'une mesure : personne n'a jamais comparé
construction en masse et construction incrémentale ici.

Et une variante encore plus simple existe : **écrire l'embedding dans le
`CREATE`** plutôt qu'en `SET` après coup. On emprunte alors le chemin
d'insertion — mesuré plus robuste que le chemin `UPDATE`, celui du segfault
du 25 août — et on ne diffère rien du tout.

## 6. L'ordre proposé

1. **Mesurer** `DROP` → ingestion → `CREATE` contre l'actuel, sur les mêmes
   27 fichiers. Aucun contrat touché ; si le rapport est bon, ça devient le
   mode d'une première ingestion, avec `reindex` comme filet.
2. **Écrire l'embedding au `CREATE`** si la mesure ne suffit pas : même
   gain potentiel, chemin plus sûr.
3. **Le vidage différé**, seulement si les deux premiers ne suffisent pas,
   et alors avec les cinq pièces que l'enquête a nommées : un
   `VectorCommitNode` (un **nœud**, donc checkpointé et rejoué), un vidage
   par rejeu de `embed_set`, un registre persistant qui distingue « vecteur
   calculé » de « vecteur indexé », `Consistency` enfin honoré, et le repli
   par balayage rebranché.

**Un cron n'est pas la bonne forme.** Ce que les trois niveaux de
`Consistency` demandent, ce n'est pas une horloge, c'est un **point de
synchronisation nommé** — quelque chose qu'on puisse attendre. Une fois ce
point existant, un cron n'est plus qu'une politique d'appel.

## 7. Deux défauts préexistants, indépendants de tout ça

- **`flush_insertions` (le chemin de cohérence par défaut) écrit sans
  indexer et vide la file** : les lignes concernées ne sont jamais
  découpées, embarquées ni indexées, et rien ne peut les rattraper.
- **`InsertRecordNode::undo` laisse des documents fantômes** dans l'index
  plein texte.

Aucun des deux ne vient de l'idée du différé ; tous deux méritent leur
correction.
