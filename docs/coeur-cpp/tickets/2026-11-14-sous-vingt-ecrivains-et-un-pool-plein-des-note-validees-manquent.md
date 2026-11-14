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

## Piste (non prouvée)

Un COMMIT qui échoue en plein ajout, faute de tampon, est annulé après la sortie du verrou
public. Son annulation (`ChunkedNodeGroup::rollbackInsert`, qui marque puis **tronque**) part
de sa première rangée. Elle emporterait les rangées qu'un autre écrivain de Note a validées entre
l'échec et l'annulation.

À vérifier :

- dans le code, l'ordre « échec du commit → sortie du verrou → annulation » ;
- au banc, quelles clés manquent. Le `CommitLog` du scénario note chaque validation
  (`N <itération>`) : comparer aux clés présentes, et dater les manquantes par rapport aux refus
  du pool.

## Ce qu'il faut pour le fermer

Un témoin déterministe : deux écrivains sur une table, le COMMIT de l'un qui échoue en plein
ajout (faute injectée), l'autre qui valide entre-temps. Puis le correctif, le témoin vert, et
`DdlEveryMinute` ×10 sans `counts-match-commits`.
