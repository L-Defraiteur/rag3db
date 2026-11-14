# La pose du 14 novembre : lib 0c41f9fe6 et binaire 012be4a58

Session G (optimiseur). Le oui de Lucie est arrivé par c7 à 13 h 25. Toutes les sessions ont dit « va »
avant l'arrêt de luciepc : 96, archi, codeparsers, hotfixs, arbre principal, le cœur, banc, tty,
everything declarative et recherche. Toutes leurs passes épinglent `RAG3DB_LIB` explicitement. Heures
lues avec `date +%T`.

## Les libs (page des libs)

| lib | md5 librag3db / libvector | ce qu'elle porte | état |
|---|---|---|---|
| **0c41f9fe6** | 9bd53729 / 0fc4f8f0 | le verrou par index de clé primaire pendant le point de reprise, et le correctif de `LocalStorage::commit` (COMMIT sous index vectoriel) | **`courant` aux deux postes depuis le 14 nov. à 13 h 37 min 59 s** ; posée |
| e93710f36 | 66486486 / d87152e4 | le correctif de `LocalStorage::commit` seul (tête 2ac8080ca) | gardée, repli |
| 835d5298f | 4caf4df6 / e173993d | l'annulation sous pool plein (b5c44313e) | gardée (`courant` avant, du 13 nov. à 20 h 06) |
| 207701bb2 | df1b7270 / df672830 | le clang-format 18 | gardée |
| 29a7bc52a | b65cc5be / 7ca6d4ec | le correctif des deux sens | gardée |

`rag3db-build-0c41f9fe6731a203b330ce975c4e26e9089bf807-Release-p12-v11-g17-s18-rcOFF-stOFF-asanOFF-tsanOFF-ubsanOFF` :
- **Bâti :** tête 0c41f9fe6, qui est aussi le dernier commit moteur, en incrémental sur e93710f36, du
  13 nov. à 23 h 47 min 37 s à 23 h 48 min 13 s. La lib et l'extension vector portent le même
  identifiant de bâti (lu par `strings`).
- **ABI contre 835d5298f :** 1 symbole exporté ajouté (`rag3db::storage::StorageManager::getPrimaryKeyIndexes()`),
  0 retiré, aucune vtable exportée changée. L'ajout est compatible vers le haut.
- **Passe e2e** sous elle, sources d64250416, `POSTE_MEM_MAX=32G` : e2e_code 36/36, e2e_lectures 27/27, VERT.
- **Départage du cœur avant la pose :** la perte de Note sous pool de tampons saturé (DdlEveryMinute ×10)
  est aussi dans 835d5298f (1/10), et avant comme après le verrou (3/10). Ce n'est pas une régression.

### `lib-des-passes` (luciepc seulement)

`~/.cache/rag3weaver-build/lib-des-passes` à luciepc est un fichier d'une ligne, l'id d'une lib du
magasin. Plusieurs scripts de passes de luciepc le lisent quand ils ne reçoivent pas de lib explicite :
`tour.sh`, `etat-e2e.sh`, `essai.sh`, `banc5.sh`, `bin-blanc.sh`, et les témoins `archfil-`, `archnom-`,
`m8def-`, `recul-`, `refus-temoin.sh` (tous dans `~/.cache/rag3weaver-build/`). Il est passé de
835d5298f à **0c41f9fe6** à la pose, et l'ancien est gardé dans `lib-des-passes.avant-0c41f9fe6`.
**Il n'y en a pas au portable** (décision de c7) : rien ne l'y lit, et les passes du portable épinglent
`RAG3DB_LIB` explicitement, la règle depuis le 25 oct. Un fichier que rien ne lit serait une fausse
autorité.

## Le binaire

Tête de master 012be4a58, bâtie juste avant la pose contre 0c41f9fe6 : md5 547a8f56, check-lib 3bc77ad5,
lucivy 4.3.5. Essai 7991 VERT aux deux postes, check-manifests 2/0 à luciepc et 6/0 au portable.
Les anciens binaires sont gardés en `.avant-0c41f9fe`.

## Les poses

| poste | arrêt | réveil | MainPID | instantané |
|---|---|---|---|---|
| luciepc | 13 h 37 min 49 s (hôte inactif) | 13 h 37 min 55 s | 3472276 | `avant-0c41f9fe6` (code-principal, code-lourd) |
| portable | 13 h 39 min 38 s | 13 h 40 min 06 s | 2943666 | `avant-0c41f9fe6` (memoire-longue, code-rag3db-depot) |

- Services : luciepc 7878 et 7879 à 4-5 ms ; portable 7979 à 82 ms et 7980 à 20 ms.
- Backends rouverts : 2 à luciepc et 5 au portable, 0 erreur au journal.
- **Témoin par le chemin de l'hôte posé** (portable, 7970) : `/sante` dit le binaire et la lib
  0c41f9fe6 ; `search_code « getPrimaryKeyIndexes »` sur code-rag3db-depot répond ok (9 occurrences).
- Atelier de Lucie : 21 outils. /ide/ : « Dépôts », 431 éléments, aucune erreur de console.
- md5 relus après la pose : binaire 547a8f56, `courant/librag3db.so` 9bd53729,
  `courant/libvector.rag3db_extension` 0fc4f8f0, aux deux postes.
- Disque du portable : 224 Gio libres (262 la veille au soir).
