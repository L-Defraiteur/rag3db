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

1. ~~**Mesurer**~~ **Mesuré le 26 à midi**, et le rapport dépasse
   l'espérance :

   | | durée |
   |---|---|
   | Ingestion, index incrémental (actuel) | **16 663 ms** |
   | Chargement, index détruit | 5 366 ms |
   | **Construction en masse de l'index** | **550 ms** |
   | Total en masse | **5 916 ms** |

   **2,8 fois plus rapide au total, et 24 fois sur la partie index** :
   ~13 s d'insertions ligne à ligne contre 0,55 s de construction en une
   fois, pour les mêmes 4 027 vecteurs. La recherche vectorielle fonctionne
   après (`SEMANTIC` rend ses résultats), et le test garde la propriété.

   Aucun contrat touché : rien n'est différé, l'index est complet quand
   `ingest_code` rend la main. Reste à décider **où** placer la bascule —
   une première ingestion volumineuse la veut, un `edit` d'un fichier
   certainement pas (détruire l'index pour trois vecteurs serait absurde).
   Un seuil, donc, et `reindex` comme filet.
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

---

## 8. Fait, le 26 août au soir : la bascule explicite

La première piste du §6 est écrite. Elle ne prend pas la forme d'un seuil.

```rust
cat.bulk_vector_index(&[FILE, SCOPE, LIBRARY], |c| c.ingest_code(&analysis))?
```

Trois décisions, chacune contre une tentation :

1. **Explicite, pas deviné.** Pas de seuil magique sur la taille du lot :
   l'appelant sait si son lot est gros. Une première ingestion la demande,
   un `edit` qui réingère trois vecteurs non — il paierait une
   reconstruction complète pour économiser trois insertions. Le nœud
   l'expose comme paramètre de configuration (`bulk_index`, faux par
   défaut), donc une fiche peut le câbler comme n'importe quel autre.
2. **Générique, pas propre au code.** La méthode prend des *entités* et
   ignore celles qui n'ont pas de signal vectoriel. Rien dans le mécanisme
   ne connaît `Scope`.
3. **Réparable.** Entre la destruction et la reconstruction, un drapeau
   `vector_index_dropped:{table}` est posé dans `_catalog_meta`. Si le
   processus meurt là, **l'ouverture suivante rebâtit** — parce que
   l'alternative, c'est une recherche vectorielle qui rend moins de
   résultats *en silence*, exactement le défaut qu'on passe nos journées à
   débusquer ailleurs.

Mesuré sur les quatre premiers fichiers de `src/dataflow`, même graphe des
deux côtés (4 fichiers, 288 scopes, 1 313 relations, 964 symboles, mêmes
résultats vectoriels), et sur le module entier :

| chemin | total |
|---|---|
| incrémental | 17 114 ms |
| en masse | 5 400 ms de chargement + 528 ms de construction = **5 928 ms** |

L'index passe de ~90 % du coût à 9 %. Ce qui reste — les 5 400 ms — est
l'affaire de la piste suivante : le court-circuit de l'inchangé.

Deux tests le tiennent, dans `e2e_code` :
`the_bulk_switch_yields_the_same_graph_and_a_working_vector_index` compare
les deux chemins, et
`an_interrupted_bulk_load_is_repaired_when_the_catalog_reopens` tue le
chargement par une panique, sur une base **sur disque**, puis rouvre **sans
redéclarer le schéma** — de sorte que rien d'autre que la réparation ne
peut recréer l'index.
