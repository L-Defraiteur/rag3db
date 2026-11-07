# Un lecteur voit les lignes d'un COPY de relations annulé par un autre écrivain

- **État** : corrigé `b7447937d` (7 novembre 2026, lot bloc-annule du cœur c++, sur master en `42b85dc62`) : un bloc
  entièrement annulé garde son information de version, lignes marquées annulées (`NO_INSERTED`), plus
  de `versionInfo.reset()` ; témoin déterministe au grain du bloc
  `RollbackUnderReaders.AFullyRolledBackBlockShowsNoRowToAnyReader`, le probabiliste 60 rejeux sur
  60 verts sur le lot (4 sur 60 rouges sur master). Ouvert le même jour.
- **Gravité** : réponse fausse (lecture sale : des lignes jamais validées comptées par un lecteur).
- **Atteignable en service** : oui — un hôte qui lit pendant qu'un autre client fait un COPY de
  relations puis l'annule (ou le voit refusé).
- **Touche rag3weaver** : par le moteur seulement (`src/storage`).

## Ce que c'est

Pendant qu'un écrivain fait un `COPY R FROM …` (5 000 relations) puis `ROLLBACK`, un lecteur qui
compte en boucle les relations d'un nœud (`MATCH (a:A {id: 0})-[r:R]->() RETURN count(r)`, 10
relations validées) rend parfois **924** ou **2058** : 10 + 914, 10 + 2 048 (un bloc entier) lignes
du COPY, jamais validées, comptées comme visibles.

## Le témoin

`test/transaction/rollback_under_readers_test.cpp`,
`RollbackUnderReaders.AReaderSurvivesTheRollbackOfAnotherWriter` (probabiliste : un fils borné par
`alarm`, 60 tours de COPY + ROLLBACK sous un lecteur en boucle).

| moteur | rejeux | rouges |
|---|---|---|
| 04063ee06 (avant les lots V2) | 40 | 0 |
| a91f066ea (master du 6 novembre, V2-1b, V2-1c, V2-2a+2b, (A)) | 60 | 4 |
| da8104fd5 (a91f066ea + retrait des générations, v22-retrait-2) | 40 | 3 |

Première vue la nuit du 26 octobre (passe du retrait : 327 verts, ce rouge), lue le 7 novembre.
Ce rouge est **celui de master**, pas celui d'un lot qui se pose dessus.

## La cause

La cause est entre 04063ee06 et f8c0c8df0 : V2-1b pièce 1 (6b63e4922), **V2-1b pièce 2
(f92b21d56, « un balayage tient ses blocs »)**, V2-1c (98cb797ff, 3724accba), V2-2a+2b
(ae4432960), ou (A) (f42d90d13). Bissection à luciepc (chaque commit bâti et joué 40 fois) : 6b63e4922 (pièce 1) 0/40,
**f92b21d56 (pièce 2) 2/40**, 3724accba 1/40. La cause : `ChunkedNodeGroup::rollbackInsert(startRow = 0)` fait `versionInfo.reset()` et
`mayHaveVersionInfo = false` — un bloc entièrement annulé n'a plus d'information de version, donc
toutes ses lignes passent pour validées ; un lecteur qui tient encore ce bloc (le balayage tient
sa liste depuis V2-1b pièce 2) et les index de lignes de l'index CSR pris avant l'annulation les
compte. 2 048 = un bloc plein, 914 = le bloc en cours de remplissage au moment du ROLLBACK.

## Le correctif

`ChunkedNodeGroup::rollbackInsert` : toute annulation, complète ou partielle, marque ses lignes
annulées (`VersionInfo::rollbackInsert`, état `NO_INSERTED`) puis tronque ; l'information de
version n'est plus jamais effacée. Au grain du bloc, donc nœuds et relations dans les deux sens.
Le témoin déterministe rejoue la course telle qu'elle est : le compte de lignes lu avant
l'annulation complète d'un bloc de 914 lignes, le balayage (`ChunkedNodeGroup::scan`) après :
rien n'est sélectionné, ni pour le lecteur ni pour l'écrivain annulé (`isInserted` est gardé par
le compte de lignes, `scan` ne l'est pas : c'est exactement la fenêtre).
