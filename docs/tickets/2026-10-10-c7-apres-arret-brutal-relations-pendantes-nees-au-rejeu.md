# Après un arrêt brutal du mélange C7, des relations à une extrémité manquante n'existent qu'après le rejeu

- **État** : ouvert (A4′ faite, 10 octobre 2026, nuit ; le chemin n'est pas trouvé).
- **Gravité** : réponse fausse (des relations pendantes après la reprise, que la base vivante
  n'avait pas) ; le vérificateur d'intégrité les voit (`rel-endpoints-exist`,
  `rel-directions-agree`, `stored-endpoints-visible`, `same-answers`).
- **Atteignable en service** : non (mode multi-écrivains, éteint hors du banc).
- **Touche rag3weaver** : non tant que le mode reste éteint.

## Ce que c'est

`Launchers/ConcurrencyBench.C7_RandomMix/Thread_Crash` (quatre écrivains, douze clés, créations,
`DETACH DELETE`, relations, mises à jour, une transaction sur huit annulée ; cinq manches ; le
processus est tué base ouverte, puis la base rejoue son journal). Sous A4′, ses variantes à chaud
et après réouverture sont passées au vert (elles étaient probabilistes, rouges 18 et 19 fois sur
20 le 3 octobre) : la base vivante n'a plus de relation pendante. Après l'arrêt brutal, le
vérificateur trouve 3 à 6 relations qui n'existent qu'après le rejeu, chacune avec une extrémité
manquante (`[rel Link0|6-><missing>|6|0]`), et des entrées stockées dont les deux extrémités sont
invisibles. Le rejeu produit donc un état que la base vivante n'avait pas — même étiquettes
qu'avant A4′, pas une régression.

## Ce qui est exclu

- Le chemin « détachement sur le dernier état validé puis mort » : témoin vert
  (`InsertLockTest.DetachDeleteOnTheLatestCommitIsReplayedTheSameWayAfterADeath`) — le rejeu
  détache la même relation que la transaction vivante.
- L'ordre du journal contre l'ordre des validations : `Transaction::commit` rend les versions
  visibles (`undoBuffer->commit`) avant d'écrire le journal, mais sous le mutex du gestionnaire,
  qui sérialise les validations ; une transaction qui a vu une autre validée écrit ses propres
  enregistrements après elle.

## Pistes

- Une transaction dont la validation échoue **après** que ses versions sont visibles (une erreur
  de l'étape 3 de `NodeTable::commit`, ticket du 10 octobre) : ses lignes ont pu être vues et
  détachées par un autre, et ne sont pas au journal.
- Un `DETACH DELETE` sous la vue du dernier état validé qui voit une transaction **au milieu**
  de sa validation (une partie de ses versions déjà à `commitTS`, pas l'autre).
- Les relations dont une extrémité est un nœud **local** de la transaction qui les crée, puis
  supprimé dans la même transaction, au rejeu.

## Pour le fermer

Un témoin minimal (le banc l'a proposé) tiré du journal d'une passe rouge : les enregistrements
de la relation pendante et du détachement de son extrémité, dans l'ordre du fichier, contre
l'ordre des événements du banc. Puis la correction, et la ligne de `known_red.txt` qui sort.
