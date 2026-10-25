# Une relation LOCKS trouvée d'un seul côté fait échouer le drain de la montre

- **État** : ouvert, le 25 octobre 2026. Signalé par G, à la demande de l'orchestration ; confié
  à la seconde session cœur C++ (hotfixs). Cause non trouvée, aucun correctif écrit.
- **Gravité** : écriture refusée, et des fichiers restent en erreur. La garde fait son travail et
  rien n'est écrit à moitié. Mais soit la base porte une arête à moitié écrite, soit la garde voit
  mal une arête valide.
- **Atteignable en service** : oui. C'est l'hôte de Lucie, sur la base du dépôt de code, depuis
  le réveil de 17 h 43 sur la lib 04063ee06. Jamais avant.
- **Touche rag3weaver** : oui, par le drain de la montre (gabarit de code, table de liens LOCKS).
- **Pour** : hotfixs, avec le cœur C++. Les lots du stockage de ce jour sont les premiers
  suspects.

## Ce qu'on voit

Le drain de la montre échoue parfois, avec ce message :

```
drain FAILED: … The transaction was rolled back because a statement in it failed … l'instruction
qui l'a défaite : Query execution failed: Runtime exception: Relationship 851 of table LOCKS was
not found in the forward direction, among the relationships of node 16249. Its update is
refused: it would have reached one direction only.
```

- Il y a trois occurrences dans `~/.local/state/rag3weaver/daemon.log` (lignes 1861, 2074 et 2464).
  La liste, avec le journal depuis 21 h 12, est copiée dans
  `~/.cache/rag3weaver-build/temoins-rust-348fa758b/locks-trois-occurrences.txt`.
- La base est `~/.local/share/rag3weaver/code-rag3db-depot.rag3db`, passée au format 42 à
  18 h 32.
- Dans le même drain, juste après la troisième occurrence, des fichiers restent en erreur :
  `extension/rag3weaver/scripts/rag3db_libs.py`, `scripts/test_rag3db_libs.py`, `src/agent.rs`, …
- Il existe deux instantanés pour comparer : `bascule-04063ee06` (avant la réouverture de 17 h 43)
  et `bascule-66994dd43`.

## La garde

`RelTableData::update` (`src/storage/table/rel_table_data.cpp`, autour de la ligne 145). Une
relation se met à jour sens par sens. Si `findMatchingRow` ne la retrouve pas dans un sens, la mise
à jour est refusée (`RelTable::REL_NOT_FOUND_IN_ONE_DIRECTION`). C'est la garde du 4 octobre :
avant elle, un seul sens était modifié, en silence.

## Pistes, non vérifiées

1. **Les numéros de colonnes qui ne changent plus** (V2-1a, 4ae060070, dans la lib 04063ee06 du
   réveil). Si la table LOCKS a perdu ou gagné une propriété (une migration du gabarit),
   l'identifiant de relation lu dans un sens peut ne pas venir de la bonne colonne, selon que les
   deux sens comptent les trous de la même façon.
2. **Le passage au format 42 d'une base de format 41** à son premier point de reprise (18 h 32) :
   la réécriture d'un groupe CSR d'un seul sens.
3. **Le segment de colonne rebâti au point de reprise** (V2-1b, 6b63e4922) et le bloc neuf bâti
   hors du verrou (3724accba), dans la lib actuelle de l'hôte. Mais le défaut précède cette lib :
   il est vu dès 04063ee06.

## Pour avancer

1. Sur une **copie** de `bascule-04063ee06` (jamais la base vivante, l'hôte tourne), lire la
   relation 851 de LOCKS depuis le nœud 16249 dans les deux sens, avec la lib d'avant (format 41)
   puis avec 04063ee06. Elle existe des deux côtés avant la bascule, ou déjà d'un seul ?
2. Faire de même sur une copie de la base vivante, prise par l'instantané de l'arbre principal.
3. Selon la réponse : un témoin rouge (une table de liens migrée, puis une mise à jour de relation
   après un point de reprise au format 42) avant tout correctif.
