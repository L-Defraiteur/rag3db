# (Requalifié) L'annulation d'un COPY dans une table indexée rend des lignes validées introuvables par leur vecteur

- **État** : **requalifié le 5 octobre 2026 — ce n'est pas l'annulation.** La ligne manque
  déjà avant le premier `COPY` annulé, une passe sur deux, et l'annulation rend exactement
  l'état d'avant (huit passes dans un seul processus, session cœur C++). C'est une ligne
  injoignable dès la construction par des `COPY` successifs : suivi dans
  `2026-10-04-ligne-lointaine-injoignable-index-bati-d-un-coup.md`, section « La même
  famille par des COPY successifs ». Ce qui suit est le constat d'origine, gardé pour sa
  recette ; son hypothèse (des arêtes de retour abîmées par l'annulation) est écartée. Ce
  n'est pas non plus la cause du plantage de la reprise de l'arbre principal, corrigé à
  part (`35d09c466`).
- **Gravité** : réponse fausse (une recherche ne rend pas une ligne validée, cherchée par son
  vecteur exact).
- **Atteignable en service** : oui. C'est le chemin d'échec d'un paquet de rag3weaver :
  index posé d'avance, morceaux par COPY, ROLLBACK des COPY du paquet.
- **Touche rag3weaver** : oui (la recherche sémantique sur Scope_Chunk, File_Chunk et
  Library_Chunk).

## Ce que c'est

Sur une table à index vectoriel, après un ROLLBACK qui annule plusieurs COPY, des lignes
validées AVANT ces COPY ne sortent plus en tête quand on cherche leur vecteur exact. La
recherche rend une autre ligne : « row 15 gives 87 », de 1 à 9 lignes sur 128. Le défaut
est variable d'une passe à l'autre. Il demeure après `e1049934e`, qui ramène le compte de
l'index à l'annulation. Aucun plantage observé.

## Recette

`single_writer_crash_test.cpp`, `ProductReloadRecovery`. Une session :

```
CREATE NODE TABLE Chunk(id INT64 PRIMARY KEY, emb FLOAT[64]);
CALL CREATE_VECTOR_INDEX('Chunk', 'chunk_index', 'emb', mu := 30, ml := 60, pu := 0.05,
     metric := 'cosine', alpha := 1.1, efc := 200);
```

La session suivante, avec `auto_checkpoint=false` et `force_checkpoint_on_copy=false` :

- quatre fois `BEGIN; COPY Chunk FROM <32 lignes>; COMMIT;` ;
- puis `BEGIN;`, trois COPY de 32 lignes, `ROLLBACK;` ;
- puis l'une des deux fins : un point de reprise, un paquet de plus validé et la mort ; ou
  une fermeture propre.

On rouvre, puis on cherche chaque ligne par son vecteur exact :
`QUERY_VECTOR_INDEX(..., 3, efs := 200)`, et la ligne doit être à la plus petite distance.

Fréquence sur dix passes, le 5 octobre :

| Fin | Rouge |
|---|---|
| Sans annulation (mort après les quatre paquets) | 0 sur 10 |
| Annulation, point de reprise, mort | 6 sur 10 |
| Annulation, fermeture propre | 5 sur 10 |

La fermeture propre n'a aucun journal à rejouer : c'est l'annulation, pas le rejeu.

## Témoin

- `Ends/ProductReloadRecovery.VectorsSetAfterRecoveryAreFound/RolledBackCheckpointThenDeath`
  et `/RolledBackThenClose` : probabilistes (`probabilistic.txt`), jamais comparés.
- `/Death` : garde-fou vert.

Ces cas sont joués avec des vecteurs de 64 dimensions proches les uns des autres : une
composante forte par ligne, sur un fond presque commun, comme des plongements réels.

## Cause

Pas établie. Les tables du graphe (`_<table>_chunk_index_LOWER` et `_UPPER`) ne sont pas
lisibles par Cypher : le banc n'a pas pu compter les arêtes.

Hypothèse **[déduit]** : la fin de chaque COPY relie ses lignes au graphe. Elle réécrit
alors aussi les listes de voisins de lignes déjà validées (`shrinkForNode` retire de vieux
voisins au profit des nouveaux). L'annulation jetterait les arêtes neuves sans rétablir les
anciennes. Les vieilles lignes resteraient avec des voisins perdus, ou pointant vers des
décalages annulés : ceux que le paquet suivant réoccupe, ou qui n'existent plus.

Une arête vers une ligne qui n'existe plus est aussi une piste pour le SIGSEGV dans
`shrinkForNode` sous un SET à la reprise. Non vérifié.

## Pour le fermer

Il faut que l'annulation d'une transaction rende aux lignes validées leurs listes de voisins
d'avant. Ou bien, à défaut, que l'index se déclare à rebâtir. Puis il faut un témoin qui
compte les arêtes par les internes : aucune arête d'une ligne validée vers un décalage annulé,
et chaque ligne joignable. Les deux cas probabilistes passent alors en garde-fous verts, sur
des essais répétés.
