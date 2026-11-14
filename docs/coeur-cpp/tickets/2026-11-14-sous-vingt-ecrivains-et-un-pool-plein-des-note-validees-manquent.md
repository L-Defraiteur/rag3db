# Sous vingt écrivains et un pool plein, des Note validées manquent

- **État** : ouvert (14 novembre 2026, cœur c++)
- **Famille** : moteur
- **Gravité** : perte de données validées (une écriture dite validée est absente, avant le point
  de reprise comme après la réouverture)
- **Atteignable en service** : à établir. Il faut un pool de tampons saturé et des écrivains
  concurrents sur une même table.
- **Touche rag3weaver** : par le moteur
- **Signalé par** : cœur c++ — `TwentyWriters.DdlEveryMinute` ×10
- **Pour** : cœur c++

## Ce qu'on voit

`TwentyWriters.DdlEveryMinute` (`test/transaction/concurrence/twenty_writers_scenario_test.cpp`)
a été joué seul 10 fois à luciepc le 14 novembre, sur `0066dce28`. Son moteur est celui de master
`0c41f9fe6` ; le commit n'ajoute qu'un test. Bâti Release dans
`~/git_workspaces/rag3db-colonnes/build/moteur`, avec `rag3db_vector_extension`. Journaux :
`~/.cache/rag3db-moteur-notes/pk-sous-reprise/ddl10b-<tour>.log`.

| Tour | Verdict | « BEGIN refusé : Buffer manager exception » |
|---|---|---|
| 1, 2, 4, 9 | rouge, `index-creation-refuses-no-reader` seul | 1 290 au tour 1 |
| 3, 10 | vert | 0 |
| 6 | rouge, **`counts-match-commits`** : Note 65 725 pour 65 730 | 124 |
| 7 | rouge, **`counts-match-commits`** : Note 109 505 pour 109 526 ; `catalog-as-expected` : doc_vec true (attendu false) ; `index-creation-refuses-no-reader` | 622 |
| 8 | rouge, **`counts-match-commits`** : Note 47 900 pour 47 920 ; `index-creation-holds-writers-at-most-its-duration` ; `index-creation-refuses-no-reader` | 467 |

- Aucun SIGSEGV (voir le ticket du lecteur qui meurt dans l'index de clé primaire).
- Doc et Link sont toujours justes. Seule Note manque, de 5 à 21 rangées.
- Les deux comptes (« avant le point de reprise », « après la réouverture ») donnent le même
  écart. Les rangées ne sont pas perdues à la réouverture : elles manquent déjà en mémoire.
- **Les trois tours qui perdent des Note sont sous pool saturé.** Les verts n'ont aucun refus.

Les écrivains de Note (2 fils) font `CREATE (:Note {id: <fil>·10⁹ + <itération>, …})`, une fois
sur dix dans un bloc `BEGIN … COMMIT`. Le journal ne compte une écriture comme validée que si
`transact` n'a rendu aucune erreur. Le compte est `MATCH (n:Note) RETURN count(n)`, par balayage :
il ne passe pas par l'index de clé primaire.

## Neuf ou masqué : présent en service (départage du 14 nov.)

`DdlEveryMinute` ×10 par base, à luciepc, en série, même arbre, même bâti (avec
`rag3db_vector_extension`). Journaux : `~/.cache/rag3db-moteur-notes/pk-sous-reprise/dep-<base>-<tour>.log`.

| Base | Verts | `counts-match-commits` (Note perdues) | SIGSEGV |
|---|---|---|---|
| `0066dce28` (moteur de master `0c41f9fe6`, avec le verrou de la clé primaire) | 2 | 3 (5, 21, 20) | 0 |
| `f41c0ba5b` (`0c41f9fe6^`, avant le verrou) | 2 | 3 (2, 5, 30) | 1 |
| `835d5298f` (la lib en service) | 3 | 1 (61) | 0 |

- La perte est présente sur les trois bases : **défaut en service, pas une régression** du
  verrou de la clé primaire.
- Toutes les pertes sont sous pool saturé : 132 à 902 « BEGIN refusé : Buffer manager
  exception » par tour. Aucun tour sans refus n'en perd.
- Le scénario n'a pas changé depuis le 18 octobre (`a4fd353e4`). Le juge
  `concurrence_test.known_red` ne joue pas `DdlEveryMinute` : c'est un cas long (`long.txt`), joué
  seulement avec `CONCURRENCE_LONG=1`. D'où une perte que personne n'avait lue.

- **L'écart des taux** : 1 tour sur 10 sur `835d5298f`, 3 sur 10 sur `f41c0ba5b` comme avec le
  verrou. On ne conclut pas : les refus du pool varient de 132 à 902 par tour, et dix tours sont
  trop peu pour dire que le correctif du COMMIT (`2ac8080ca`) aggrave (c7, 14 nov.).

## La fenêtre, lue dans le code et prise au témoin

**Le chemin.**

1. `TransactionManager::commit` tient `mtxForSerializingPublicFunctionCalls` pendant
   `transaction->commit`. Celui-ci verse d'abord les lignes locales dans les groupes de nœuds
   (`localStorage->commit()`, `transaction.cpp` l. 182), puis écrit et synchronise le journal.
2. Si une étape lève (journal non synchronisé, tampon plein), l'exception sort, et le verrou est
   rendu. La transaction reste active, ses lignes versées.
3. `ClientContext::TransactionHelper::runFuncInTransaction` (`client_context.cpp`, le `catch`)
   l'annule alors : `context.rollback()` → `TransactionManager::rollback`, qui **reprend** le
   verrou.
4. Entre 2 et 3, un autre écrivain qui attendait le verrou pour son COMMIT le prend. Il verse
   ses lignes **à la suite** de celles de A, dans le même groupe, et valide.
5. L'annulation de A :
   - `NodeGroup::rollbackInsert` (`node_group.cpp` l. 595) ramène `numRows` et
     `nextRowToAppend` à la première ligne de A ;
   - `ChunkedNodeGroup::rollbackInsert` tronque le bloc à cette ligne ;
   - les lignes validées de B, situées après, partent avec.

   L'index de clé primaire perd aussi la clé de B : la recherche par clé de la ligne de B rend 0.

**Le témoin** : `RefusedCommitRollback.AWriterCommittingWhileARefusedCommitUnwindsKeepsItsRow`
(`test/transaction/refused_commit_rollback_test.cpp`, branche `annulation-garde-les-autres`,
`8a4bfe9eb`).

- Chaque tour : A ouvre un bloc de 60 000 lignes, et B un bloc d'une ligne. Le journal refuse
  une synchronisation (`CutFileSystem::failSyncOf`). Le COMMIT de A part dans un fil, celui de B
  5 ms après, et B attend le verrou.
- 20 tours. Rouge 3 fois sur 3 à luciepc (Release, master `0c41f9fe6` + le témoin) : A refusé 20
  fois, B validé 20 fois, **18 à 20 lignes de B perdues**. La ligne du tour n'est jamais
  retrouvée par sa clé, et le balayage en perd la plupart.
- Les deux cas séquentiels du même fichier (B valide après l'annulation de A) sont verts. Ils
  gardent l'annulation entière d'un COMMIT refusé.

## Ce qu'il faut pour le fermer

- Le correctif. Deux formes sont possibles, à choisir :
  - l'annulation d'un COMMIT refusé se fait **sous le même verrou** que lui, avant de le rendre ;
  - ou l'annulation ne retire que **ses** lignes : marquées annulées sans troncature quand
    d'autres ont été versées après.
- Le témoin concurrent vert.
- `transaction_test` et `concurrence_test` entiers.
- `DdlEveryMinute` ×10 sans `counts-match-commits`.
- La lib d'après.

## Correctif retenu (c7, 14 nov., 4 h 30 ; Lucie tranche autrement si elle veut)

- **(a)** Le COMMIT refusé s'annule **sous le même verrou**, avant de le rendre. La fenêtre
  disparaît, et le témoin concurrent en est la preuve. Le coût est le temps d'une annulation sous
  le verrou : il est borné, et se mesure comme l'exclusif de la clé primaire.
- **Une garde** dans `NodeGroup::rollbackInsert` : elle refuse de tronquer au-delà de la dernière
  ligne versée par la transaction qui annule (`KU_ASSERT` en debug, erreur en Release). Si une
  autre voie rouvre la fenêtre, elle rougit au lieu de perdre des lignes.
- **(b)**, ne retirer que ses lignes sans tronquer, reste la forme d'après, si (a) coûte trop
  sous charge.

Après la pose du matin :

1. `RefusedCommitRollback` rouge puis vert 20 sur 20 ;
2. `transaction_test` et `concurrence_test` entiers ;
3. `DdlEveryMinute` ×10, `counts-match-commits` vert partout ;
4. la lib d'après, à ce hash.
