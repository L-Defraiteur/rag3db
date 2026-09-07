# Un HNSW sur codes quantifiés (TurboQuant) dans l'extension vecteur

**7 septembre 2026.** Note d'idée, à la demande de Lucie, après lecture de
[turbovec](https://github.com/ryancodrai/turbovec) (Rust, MIT, 16 700 étoiles
en cinq mois, 1.0.0 le 18 août 2026). Ce n'est pas un chantier ouvert : c'est
ce qu'on ferait, pourquoi, et ce qui dirait qu'il est temps.

## Ce que turbovec est, et n'est pas

Une implémentation de **TurboQuant** (Google Research) : quantification des
vecteurs à 2 ou 4 bits par coordonnée, *data-oblivious* — pas de codebook à
apprendre, une rotation puis une grille fixe, avec une calibration par
coordonnée optionnelle (TQ+). La recherche est un **balayage exhaustif** sur
les codes, par table de correspondance, comme FAISS `IndexPQFastScan`.

Ses chiffres : 3,4–3,5× plus rapide que FAISS PQ FastScan à 4 bits, sur
100 000 vecteurs, 8 vCPU, k=64 ; 10 M de documents en 4 Go au lieu de 31 ;
`add()` en 6–20 µs, suppression en O(1). Le rappel est mesuré contre FAISS PQ,
**pas contre l'exact**. Bancs scriptés et résultats JSON dans le dépôt.

Pour nous, aujourd'hui, ça ne change rien à la latence de lecture : notre
HNSW répond en moins d'une milliseconde sur des corpus qui tiennent en
mémoire (20 000 chunks × 384 flottants = 30 Mo), et ce qui coûte à la
requête est ailleurs (embarquement de la requête, plein texte, fusion,
allers-retours en base).

## L'idée à garder : le HNSW qui traverse sur des codes

C'est la façon standard des index de production (FAISS `IndexHNSWPQ`, Qdrant,
Milvus, DiskANN) :

1. **le graphe traverse sur les codes compressés** — distance approchée par
   table de correspondance, huit fois moins d'octets lus par nœud visité, une
   traversée qui tient dans le cache ;
2. **les derniers candidats sont reclassés sur les vecteurs pleins** — le
   rappel revient à celui du HNSW exact, ou presque ; la colonne `embedding`
   existe déjà pour ça ;
3. **TurboQuant apporte l'absence d'entraînement** : une insertion ne dépend
   pas de ce qui est déjà là, le graphe et les codes se mettent à jour
   ensemble. C'est ce qui le distingue d'un PQ classique pour un index qui
   bouge en continu (un dépôt de code sous les doigts d'un agent).

Chez nous, c'est **dans l'extension vecteur du cœur C++**, pas dans
rag3weaver : le HNSW de rag3db range des flottants 32 bits dans ses nœuds et
calcule ses distances dessus. Il faudrait y ranger des codes, écrire le noyau
de distance par table (une LUT par requête, 2 et 4 bits), et brancher le
reclassement sur la colonne pleine. Une petite semaine pour quelqu'un qui
connaît l'extension, plus le banc de rappel.

## La variante plus simple : un index plat quantifié à côté

Sans toucher au HNSW : un second index, plat, sur codes, alimenté à
l'écriture sans rien construire (ajouter = écrire les codes, supprimer =
échanger). Interrogé pendant que le HNSW n'est pas à jour, ou seul sur les
petits dépôts (quelques centaines de milliers de chunks, quelques
millisecondes par requête). C'est l'angle **écriture** : le HNSW se construit
(on le reconstruit en masse après une première ingestion), n'aime pas les
suppressions, et chaque insertion incrémentale paie des distances pour se
placer.

## Ce qui dirait qu'il est temps

- un index qui dépasse la mémoire cache puis la mémoire vive : quelques
  millions de vecteurs, ou un index qui porte **plusieurs embarquements** de
  gros modèles (idée de Lucie du 6 septembre : une colonne par modèle, la
  liste des modèles disponibles dans la méta) ;
- ou une mesure qui montre que l'écriture est le problème : le coût de la
  reconstruction du HNSW après une première ingestion et celui d'une
  insertion incrémentale, deux chiffres à ajouter au banc
  `e2e_mesure_ingestion_code` (une demi-heure), et qu'on n'a pas encore.

Le banc à faire ce jour-là : nos vecteurs granite-107m (384) et granite-278m
(768) sur nos chunks, rappel à 4 bits **contre l'exact**, sur les 45
questions du banc de qualité de la session moteur.

## Les deux chiffres, mesurés le soir même

Banc de l'optimiseur (`tests/e2e_banc_hnsw.rs`, `docs/optimiseur/7-septembre-2026-21h56/01`),
cœur C++, 21 778 chunks, HashEmbedder :

| | 384 dims | 768 dims |
|---|---|---|
| reconstruction du HNSW après ingestion en masse | 4,9 s (0,23 ms/chunk) | 9,4 s (0,43 ms/chunk) |
| insertion ligne à ligne dans l'index plein, surcoût HNSW par chunk | 3,2 ms | 4,6 ms |
| point mort au-delà duquel drop + reconstruire gagne | ~1 500 chunks | ~2 000 chunks |

Lecture : **TurboQuant n'a rien à gagner chez nous, ni en lecture ni en
écriture** — 5 à 9 s une fois, 3 à 5 ms par chunk qu'un agent ne voit pas.
Ce que ça donne à coût nul : un seuil dans le rattrapage d'embarquement qui
tombe et reconstruit l'index au-delà de ~1 500 chunks (confié à rag3db-40,
dans le chantier de l'index à plusieurs embarquements). L'idée reste notée
pour le jour où un index dépasse la mémoire.

## En passant : la taille du plein texte

Lucie note que lucivy pèse 3,6× le texte brut sur disque et 5× en mémoire
décompressée. C'est l'ordre de grandeur d'un index inversé avec positions
(Lucene est dans les mêmes eaux) ; ce n'est pas le même objet qu'un index
vectoriel, et la quantification n'y répond pas. Ce qui y répondrait, c'est
le choix de ce qu'on indexe avec positions et de ce qu'on n'indexe qu'en
présence, et c'est un autre sujet.
