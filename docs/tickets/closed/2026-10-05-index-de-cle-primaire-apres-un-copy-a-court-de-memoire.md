# Après un COPY annulé, refusé ou à court de mémoire, le point de reprise de l'index de clé primaire ne finit plus

- **État** : corrigé le 5 octobre 2026 (« fix(index de clé): le compte de l'index en mémoire recule au retrait d'une clé, et sa réservation ne passe plus sous zéro — plus de point de reprise sans fin après un COPY annulé, refusé ou à court de mémoire »). Aucun filet n'est resté dans le moteur
- **Gravité** : blocage, et danger pour l'hôte (une réservation de mémoire sans borne) ; avec un COPY à court de mémoire, aussi une réponse fausse (une ligne créée ensuite introuvable par sa clé)
- **Atteignable en service** : oui — un gros `COPY` annulé ou refusé, la même session qui continue, puis un point de reprise ; ou un point de reprise de fermeture après le `COPY` annulé, puis le `COPY` rejoué après la réouverture
- **Touche rag3weaver** : oui — le chemin d'échec d'un paquet (un `COPY` annulé puis rejoué), et le `COPY` refusé suivi d'un repli
- **Ouvert le** : 5 octobre 2026, session cœur C++ (le contrôle du chargement journalisé l'a attrapé par un `COPY` à court de mémoire) ; la porte la plus large — sans aucune panne de mémoire — a été trouvée par le banc (son ticket : `2026-10-05-point-de-reprise-sans-fin-apres-un-copy-annule.md`)
- **Pour** : cœur C++ (index de hachage, annulation)

## Ce que c'est

Après un gros `COPY` annulé (`ROLLBACK`), refusé pour une clé en double, ou interrompu par « The buffer pool is full », le `COPY` suivant passe, les lignes sont là, chaque clé se retrouve — et le point de reprise d'après ne finit plus : il prend de la mémoire sans s'arrêter (4 à 8 Gio en une demi-seconde quand rien ne le borne).

## Recette minimale (le banc, sur `master` d'avant le correctif, sans panne de mémoire)

```cypher
CALL auto_checkpoint=false;
CREATE NODE TABLE Doc(id INT64 PRIMARY KEY, name STRING);
BEGIN TRANSACTION; COPY Doc FROM 'cent-mille-lignes.csv'; ROLLBACK;
COPY Doc FROM 'cent-mille-lignes.csv';
CREATE (:Doc {id: 500000, name: 'a'}); CREATE (:Doc {id: 500001, name: 'b'});
CHECKPOINT;        -- ne finit pas
```

## La cause

Deux défauts d'origine de l'index de clé primaire, qui se combinent.

1. **L'index en mémoire ne reculait jamais son compte d'entrées.** `InMemHashIndex::deleteKey` (`src/include/storage/index/in_mem_hash_index.h`) retirait la clé de sa case sans toucher à `indexHeader.numEntries` — le fichier n'avait que deux « ++ ». Or l'annulation d'un `COPY` retire ses clés une à une par ce chemin (`RollbackPKDeleter` → `discardLocal`). Après un `COPY` de 100 000 lignes annulé, l'index en mémoire était vide et se croyait à 100 000.
2. **La réservation de l'index sur disque soustrayait sans regarder.** `HashIndex::reserve` (`src/storage/index/hash_index.cpp`) appelait `splitSlots` avec « cases requises − cases existantes », en non signé. Le point de reprise qui suit le `COPY` validé réservait pour le compte gonflé (200 000) et n'y rangeait que 100 000 clés ; au point de reprise suivant, les cases requises étaient moins nombreuses que les cases existantes, la soustraction rebouclait, et `splitSlots` partait pour diviser de l'ordre de 2^64 cases en en ajoutant une à chaque tour.

Sur la même piste, deux défauts de l'annulation côté table, vus avec un `COPY` à court de mémoire (il réserve et compte ses lignes avant de les écrire) : le compte de lignes de la table ne reculait que des lignes réellement écrites (« the table holds 243918 rows in 1 groups; group 0: 81306 rows » après deux échecs), et le curseur de réservation du groupe (`NodeGroup::nextRowToAppend`) n'était reculé par aucune annulation. Une ligne créée ensuite n'était pas retrouvée par sa clé.

## La taille

Mesuré sur le moteur d'avant (table vide, clé `INT64`, un `COPY` annulé puis le même validé, deux `CREATE`, `CHECKPOINT` ; chaque essai borné à 20 s et 2 Gio) : **16 000 lignes passent, 18 000 ne finissent plus**. L'index est en 256 parts qui commencent chacune à une page de cases ; il faut que la réservation gonflée en ajoute.

Par lecture, non mesuré : sur une table déjà grande, un `COPY` annulé de quelques milliers de lignes suffit à ajouter une case par part. Un paquet de rag3weaver défait sur un index de plusieurs dizaines de milliers de lignes est donc dans la zone.

## Le correctif

- Le compte de l'index en mémoire recule au retrait d'une clé.
- La réservation de l'index sur disque ne divise que s'il manque des cases.
- Le compte de lignes de la table recule de tout ce que l'enregistrement d'annulation y avait ajouté (`NodeTableVersionRecordHandler::rollbackInsert`) ; le curseur de réservation recule avec les lignes (`NodeGroup::rollbackInsert`).

**Une base déjà abîmée** — un index sur disque plus grand que son compte, écrit par un point de reprise de l'ancien moteur : fabriquée avec un moteur d'avant (`COPY` annulé, `COPY` validé et son point de reprise, arrêt), puis ouverte sur une copie par le moteur corrigé — 100 000 lignes, deux `CREATE`, un `CHECKPOINT` qui finit, 50 000 `CREATE` de plus, un second `CHECKPOINT`, la réouverture : comptes et clés justes. C'est la seconde correction qui la couvre. Essai fait une fois, à la main (programme `essai-index.cpp` aux annexes de la session) ; pas de témoin au dépôt, il faudrait une base en fichier.

Un filet avait été écrit en attendant la cause (après un `COPY` à court de mémoire, la base exigeait d'être rouverte) ; il n'a pas été poussé : il coûtait des pages perdues et changeait trois tests d'origine.

## Témoin

`test/transaction/rolled_back_copies_test.cpp`, `RolledBackBigCopyTest`, quatre cas à 100 000 lignes : un `COPY` annulé puis le même validé ; trois annulés ; un `COPY` refusé pour une clé en double au milieu du lot ; la forme de la reprise de rag3weaver (annulé, point de reprise, réouverture, `COPY`, point de reprise). Chacun : comptes, clés aux deux bouts, plus grand décalage, avant et après le point de reprise et la réouverture.

`test/copy/copy_test.cpp`, `CopyTest.RowCountIsRightAfterCopiesThatRanOutOfMemory` : des `COPY` à court de mémoire puis un réussi, et la même session qui continue — une ligne créée, retrouvée par sa clé à son décalage, un point de reprise, la réouverture.

Rouges avant : joués sous plafond de mémoire (clé introuvable, puis point de reprise sans fin). Au banc : la recette par un processus fils borné.

## Ce qui reste, hors de ce ticket

- Les pages qu'un `COPY` annulé ou refusé avait écrites, quand la base est fermée sans point de reprise ensuite (ticket de durabilité, « les pages d'un COPY tué »).
- `STATS_INFO` compte encore les lignes annulées : c'est une estimation, et un autre ticket (`2026-10-04-copy-refuse-gonfle-la-cardinalite.md`).
- À deux écrivains d'un même bloc (mode éteint hors du banc), reculer le curseur de réservation recule aussi celle de l'autre écrivain : rangé sous A3′ avec le défaut voisin de l'annulation.
