# Le HNSW à l'écriture : ce que coûtent la reconstruction et l'insertion

**7 septembre 2026, session « optimiseur », à la demande de la session
architecture.** Deux chiffres qui manquaient au banc d'ingestion, pour
décider si l'idée TurboQuant ([`docs/7-septembre-2026-10h12/01`](../../../../../docs/7-septembre-2026-10h12/01-un-hnsw-sur-codes-quantifies.md)
à la racine) vaut quelque chose pour nous à l'écriture : **le HNSW se
construit** (on le reconstruit en masse après une première ingestion) et
**chaque insertion incrémentale paie des distances pour se placer**. Combien ?

## Le banc

`tests/e2e_banc_hnsw.rs` (`./run_e2e.sh --test e2e_banc_hnsw`, avec
`RAG3WEAVER_MESURE_RACINE` sur `src/` du dépôt). Le cœur C++ de rag3db :
1 642 fichiers, 18 141 scopes, **21 778 chunks**. Vecteurs par `HashEmbedder`
(pour le HNSW le modèle n'importe pas, la dimension si : 384 comme
granite-107m, 768 comme granite-278m) ; base en mémoire, moteur
`build/lecteurs-csv`, extension `vector` seule. Trois mesures :

1. la première ingestion en masse par `Catalog::bulk_vector_index`, avec le
   temps *dans* la fermeture (ingestion, index tombé) et le temps *autour*
   (la reconstruction), puis une reconstruction seule pour confirmer ;
2. un lot de **700 chunks** synthétiques (50 fichiers Rust inédits, 6
   fonctions chacun, un nonce par corps : aucun court-circuit d'idempotence)
   inséré **ligne à ligne dans l'index plein**, le chemin de toujours ;
3. le même lot, index tombé, puis reconstruit.

## Les chiffres

| | dim 384 | dim 768 |
|---|---|---|
| ingestion en masse, index tombé (21 778 chunks) | 10,9 s | 12,3 s |
| **reconstruction du HNSW** après, seule (drop + create) | **4,9 s** (0,23 ms/chunk) | **9,4 s** (0,43 ms/chunk) |
| insertion ligne à ligne, index plein | 6,14 ms/chunk | 7,74 ms/chunk |
| la même insertion, index tombé | 2,92 ms/chunk | 3,11 ms/chunk |
| **ce que chaque chunk paie au HNSW** | **3,2 ms** | **4,6 ms** |
| point mort : reconstruire plutôt qu'insérer à partir de | 1 500 chunks | 2 000 chunks |

Sur les 700 chunks du lot incrémental : 4,3 s ligne à ligne contre 2,0 s
index tombé (dim 384), 5,4 s contre 2,2 s (dim 768) — le HNSW **double** le
coût d'une insertion incrémentale, et c'est sans modèle (vecteurs hachés,
donc sans le temps d'embarquement, qui est ailleurs : 10,7 s sur
l'ingestion complète avec granite-107m et la flash).

## Ce que ça dit

- **À la reconstruction, le HNSW ne pèse rien de grave** : 4,9 s sur 26 s
  d'ingestion du cœur C++ (dim 384), un cinquième ; 9,4 s en 768. C'est le
  prix d'une passe sur 20 000 vecteurs, linéaire en dimension (×1,9 pour ×2),
  et il ne se paie qu'une fois par index. Un index plat quantifié qui
  s'alimente « sans rien construire » gagnerait ces 5 à 9 s sur une
  première ingestion — pas de quoi porter un chantier d'une semaine.
- **À l'insertion incrémentale, le HNSW coûte 14 fois plus par chunk que la
  reconstruction** (3,2 ms contre 0,23 ms) : c'est le placement, les
  distances aux voisins, une à une, par la voie `CREATE`/`SET` du moteur. Pour
  un agent qui touche un fichier (une dizaine de chunks), c'est 30 à 50 ms :
  invisible. Pour un rattrapage de 2 000 chunks et plus, reconstruire est
  déjà moins cher qu'insérer, et c'est ce que `bulk_vector_index` sait faire.
- **Donc, pour nous, TurboQuant à l'écriture ne vaut rien aujourd'hui** :
  ce qu'il gagnerait, c'est les 3 à 5 ms par chunk incrémental (un agent ne
  les voit pas) et les 5 à 9 s d'une reconstruction (une fois). L'angle qui
  reste est celui du doc d'idée : la mémoire quand l'index dépasse le cache,
  ou plusieurs embarquements — pas encore notre cas (30 Mo en 384).
- Ce que le banc suggère à la place, à coût nul : **un seuil dans le chemin
  incrémental** — au-delà de ~1 500 chunks (384) / ~2 000 (768) à insérer
  d'un coup, passer par `bulk_vector_index` plutôt que ligne à ligne. Le
  chemin de masse de l'architecture le fait déjà pour une première
  ingestion ; un rattrapage volumineux (branche changée, dépôt mis à jour)
  gagnerait pareil.

## Les réserves

Base en mémoire (pas de disque dans le HNSW ni dans l'insertion) ; un seul
lot de 700, une seule mesure par dimension (les chiffres de la première
ingestion sont à ±10 % d'une passe à l'autre) ; le coût par chunk ligne à
ligne inclut le reste de l'écriture (entités, relations, symboles :
`entités 3 698 ms` sur 4 297), le HNSW en est la moitié par différence avec
l'index tombé. `MockEmbedder` est exclu exprès : ses vecteurs identiques
font segfauter le HNSW au-delà de quelques centaines de points.
