# Sous vingt écrivains, un pool plein laisse une annulation inachevée

- **État** : ouvert (9 novembre 2026, cœur c++)
- **Famille** : moteur
- **Gravité** : indisponibilité — la base refuse toute transaction jusqu'à sa réouverture
- **Atteignable en service** : à établir. Le banc le reproduit seul, sur master avant et après
  29a7bc52a ; un hôte sous un pool de tampons serré et des DDL concurrents y ressemble.
- **Touche rag3weaver** : par le moteur (l'hôte devrait fermer et rouvrir la base)
- **Signalé par** : cœur c++ — banc et reprise décrits dans ce ticket
- **Pour** : cœur c++

## Ce que c'est

`TwentyWriters.DdlEveryMinute` (`test/transaction/concurrence/twenty_writers_scenario_test.cpp`)
rougit seul sous poste lourd, sur 813d7eda8 (master sans le correctif des deux sens) comme sur
52d73a301 (avec). La lib du banc est bâtie de 9bae60786 et suivants, en Release, à luciepc.

Ce que dit son journal (`~/.cache/rag3db-moteur-notes/csr/dep-base.log` à luciepc) :

| Message | Nombre |
|---|---|
| `BEGIN refusé : Buffer manager exception: Unable to allocate memory! The buffer pool is full and no memory could be freed!` | 9 |
| `BEGIN refusé : A rollback of this database could not complete, so it must be closed and reopened before it is used again` | 318 434 |

Le contrôle qui échoue est `index-creation-refuses-no-reader` (8 refus de lecteurs), puis une
exception non rattrapée (`unknown file: Failure`). Les fenêtres « 5 CHECKPOINT », « 6 ADD b
DEFAULT » et « 6 migration annulée » ne voient plus que des refus.

**Lecture** : un pool plein fait échouer une annulation, et la base se met d'elle-même en état
« à rouvrir » (la garde posée pour une annulation inachevée). Tout ce qui suit est refusé. La
question est de savoir si l'annulation peut échouer faute de mémoire, ou si elle devrait réserver
ce qu'il lui faut avant de commencer.

`TwentyWriters.DdlEveryFifteenSeconds` montrait les mêmes « buffer pool is full » dans le banc
entier, mais passe seul des deux côtés.

## Les seuils de temps notés ici

Deux cas de coût rougissent seuls, sur les deux mêmes versions. Ce sont des seuils à revoir avant
d'en faire un défaut :
- `Builds/VectorIndexDeleteCost.DoesNotDependOnIndexSize/SetInBatchesOf64` ;
- `Builds/VectorIndexDeleteCost.DoesNotDependOnIndexSize/SetInBatchesOf64WithRelation` : 7,95 ms
  par ligne à 15 000 lignes pour 5,86 permises, contre 1,93 ms à 1 500.

`TwentyWriters.KilledDuringTheCheckpoint` (enfant mort de SIGSEGV avant le point de reprise) n'a
été vu que dans le banc entier ; il passe seul des deux côtés. À rejouer sous charge, sans ticket.

## Témoin

`TwentyWriters.DdlEveryMinute` lui-même, rouge seul. Un témoin réduit reste à écrire : une
annulation qui échoue faute de pool, puis un BEGIN.

## Pour le fermer

La cause de l'annulation inachevée, le correctif, le témoin vert.

## Reprise du 11 novembre — témoin réduit rouge

Le témoin initial `6e7f11680` ne reproduit pas : `FullBufferRollback.*`, 3/3 verts
à luciepc (3,325 s), journal `~/.cache/rag3db-moteur-notes/annulation/rouge.log`
(le nom du fichier ne dit pas son verdict). La variante CREATE à clé STRING courte
passe aussi (4/4 au total, `cle-string.log`) : ses lignes sont encore locales.

Le COPY atteint au contraire `INSERT_INFO` puis `rollbackPKIndexInsert`. Le cas
existant INT64 et sa variante STRING courte passent (2/2, `copy-string.log`).
Avec 20 000 clés STRING préfixées de 128 caractères `k`, le défaut est reproduit :
`CopyRollbackUnderAFullBuffer.StringKeysRemainUsableAfterRollback`, témoin sauvegardé
sur `annulation-sous-pool-plein`, commit `65486c02d`.

Recette : pool des tests de 64 Mio, point de reprise automatique coupé, COPY dans
Doc sous transaction A ; B insère et valide dans Other, par lots depuis 5 000 avec
division par deux après refus. B valide 155 077 lignes et subit 7 refus.
Le premier ROLLBACK de A échoue explicitement avec « buffer pool is full », le second
et le BEGIN suivant exigent de rouvrir la base, COUNT est refusé. Le fils ferme sans
signal. Le voisin INT64 reste vert. Preuve : luciepc
`~/.cache/rag3db-moteur-notes/annulation/copy-long-string.log` (1 vert, 1 rouge).

Aucun correctif à ce stade. Prochain pas : tracer l’allocation durant le retrait des
clés longues, puis comparer les stratégies des moteurs avant proposition à l’orchestration.

### Cause tracée et suite

La pile de `annulation/trace.log` confirme : `rollbackPKIndexInsert` →
`scanIndexColumns` → `StringVector::reserveString` →
`InMemOverflowBuffer::allocateNewBlock` → `MemoryManager::allocateBuffer` →
`BufferManager::pin`, qui refuse. La copie des clés longues vers le vecteur de
lecture demande le nouveau bloc ; ce n’est pas une preuve d’allocation dans le
retrait de la clé elle-même. La trace reproduit le rouge, voisin INT64 vert.

Proposition transmise à l’orchestration avant tout correctif : lire les clés depuis
leur stockage existant pour l’annulation, sans recopier les chaînes dans un nouveau
tampon ; vérifier les verrous et la durée de vie de la vue empruntée.

Lectures de référence : PostgreSQL réserve 8 Kio dans
[ErrorContext](https://github.com/postgres/postgres/blob/master/src/backend/utils/mmgr/mcxt.c)
pour traiter les erreurs ; cela ne prouve pas une réserve pour tout rollback.
[SQLite](https://sqlite.org/lang_transaction.html) peut annuler toute la transaction
après SQLITE_NOMEM. [DuckDB](https://duckdb.org/docs/2.0/guides/troubleshooting/crashes)
invalide la base après une erreur interne. Ces politiques ne démontrent pas que
notre annulation exige une réserve forfaitaire.

Branche de reprise désormais `cx-coeur-annulation`, publiée, worktree
`~/git_workspaces/cx-coeur-annulation`, même commit `65486c02d`.
L’ancien worktree `rag3db-csr` reste sur disque, détaché, sans suppression.

### Correctif en essai, non validé

Commit `0511dece7` sur `cx-coeur-annulation` : les slots de l’index local détiennent
déjà les clés et leurs décalages. `discardLocalRange(first,count)` retire celles
qui appartiennent à la plage réservée par l’undo, sous verrou exclusif local,
sans lecture des colonnes ni modification du persistant ou des tombstones.
Les clés conservées sont déplacées dans les slots existants, en maintenant le
préfixe compact de chaque chaîne. La comparaison par soustraction évite de faire
déborder `first+count`. Le code impose un déplacement `noexcept`. Cela concerne
ce retrait de clés, pas une preuve que toute annulation du moteur n’alloue rien.

Compilation et tests ciblés en file à luciepc (`annulation/vert.log`) : aucun
verdict sur ce correctif à ce point. Les suites entières et la mesure de coût
restent à jouer ; le ticket reste ouvert.

## Vérifications entières du 11 novembre et travail restant

Porteur : cœur C++. Lot `0511dece7`, branche `cx-coeur-annulation`. Les sources et
tests distants ont été comparés à ce commit : `git diff 0511dece7 -- src test` vide.
Aucun rebuild supplémentaire avant ces suites ; mêmes binaires que les ciblés.

Preuves à luciepc : `~/.cache/rag3db-moteur-notes/annulation/` :
`vert.log`, `transaction.log`, `transaction.xml`, `concurrence.log`,
`concurrence.xml`, `suites.log`, `suites-codes.txt`. Les lanceurs 3809571 puis
1423326 sont terminés.

- Compilation entière réussie ; stockage 8/8 verts ; ciblés FullBuffer/Copy 6/6
  trois fois, avec réouverture et compte des lignes validées inchangé.
- `transaction_test` : code 0, 359 cas joués, 357 verts, 2 outils sautés attendus
  (`VectorIndexCrashReopenProbe.OpenInAFreshProcess`, `WalReader.PrintAFile`),
  1 cas désactivé ; 1 336,857 s.
- `concurrence_test` : code 1, XML 424 cas, 25 rouges ; 3 206,038 s.
  Comparaison aux listes du lot : 16 dans known_red, 5 dans probabilistic,
  4 hors listes. Aucun verdict global vert à ce point.

Les quatre hors listes sont précisément ceux déjà décrits dans ce ticket :
`TwentyWriters.DdlEveryFifteenSeconds`, `TwentyWriters.DdlEveryMinute`,
`Builds/VectorIndexDeleteCost.DoesNotDependOnIndexSize/SetInBatchesOf64` et
`Builds/VectorIndexDeleteCost.DoesNotDependOnIndexSize/SetInBatchesOf64WithRelation`.
Leur présence historique n’est pas une preuve de leur indépendance du correctif
actuel. Travail restant lié à ce même ticket, sans doublon :

1. Rejouer ces quatre cas isolément sur `0511dece7` ; comparer aux symptômes du banc.
2. Si un rouge persiste, rejouer sur la base sans correctif, avec mêmes conditions.
3. Mesurer séparément le coût de plusieurs enregistrements undo, en conservant les
   conditions et les preuves.
4. Départager, puis seulement demander un tour de push et coordonner la lib.

Le ticket reste ouvert. La fenêtre de push de l’architecture est respectée :
aucun tour C++ accordé, aucun code du lot sur master.

### Rejeux isolés terminés — trois rouges à comparer à la base

`annulation/departage.log` et `isole-*.{log,xml}`, à luciepc, même binaire :
- `DdlEveryFifteenSeconds` : vert, code 0, 243,298 s.
- `DdlEveryMinute` : rouge, 376,830 s, 14 lecteurs refusés pendant la création
  d’index ; puis exception de liaison « Cannot find property b for d ».
- `SetInBatchesOf64` : rouge, 8,330 ms/ligne à 15 000 contre 2,137 à 1 500
  (borne 6,274).
- `SetInBatchesOf64WithRelation` : rouge, 8,509 contre 2,421 (borne 6,842).

Le lanceur 2956994 est terminé. La disparition du défaut d’annulation dans le
témoin réduit ne suffit pas à expliquer ces rouges. Suite déjà portée par ce
ticket : comparer les trois cas persistants au parent sans correctif `65486c02d`
(moteur de `6e7f11680`), sous les mêmes conditions, avec sauvegarde des binaires
du correctif avant rebâti. Mesurer les cas de plusieurs COPY annulés des deux
côtés à partir des temps gtest, en distinguant mesure et garantie fonctionnelle.
Aucun nouveau défaut attribué à ce stade, pas de changement des listes de rouges.

## Reprise du 12 novembre — comparaison terminée, verdict encore partiel

Porteur : cœur C++. Le lanceur `3805918` est terminé ; aucun processus à attendre
et aucun rebuild ni rejeu lancé lors de cette reprise. Preuves à luciepc sous
`~/.cache/rag3db-moteur-notes/annulation/` : `base.log`,
`base-TwentyWriters.DdlEveryMinute.{log,xml}`, `base-SetInBatchesOf64.{log,xml}`,
`base-SetInBatchesOf64WithRelation.{log,xml}`, `base-plusieurs-undo.{log,xml}`.

Source contrôlée : HEAD distant `6e7f116806034c7ac6eb3bf2f6309d1c8aaf1b93`,
fichiers du parent sans correctif `65486c02d` ; diff vide pour `src`, `test`,
`extension/vector`, `extension/geo`. Le bâti entier de la base finit en 141 étapes.
Ne pas attribuer les binaires courants au correctif : ce sont désormais ceux de
la base. Les binaires sauvegardés du correctif sont dans `binaires-0511dece7/`
avec SHA256 ; une extension chargée dynamiquement exige aussi son bâti assorti.

### Deux rouges de coût présents des deux côtés

| Cas | Correctif : 15 000 / 1 500 lignes (ms/ligne) | Base (ms/ligne) | Verdict |
|---|---:|---:|---|
| SetInBatchesOf64 | 8,330 / 2,137 | 10,469 / 2,378 | même contrôle rouge des deux côtés |
| SetInBatchesOf64WithRelation | 8,509 / 2,421 | 11,185 / 3,059 | même contrôle rouge des deux côtés |

Ces observations établissent que le correctif n’est pas nécessaire pour reproduire
ces deux rouges. Elles ne mesurent pas une accélération : exécutions uniques et
charge non appariée, pas de borne assouplie ni de liste modifiée.

### DdlEveryMinute : verdict différentiel inconnu

La base est rouge (1 009,647 s), sur l’attente de deux écrivains pendant
`3 CREATE REL TABLE` (785/769 ms, bornes d’environ 177/168 ms). Elle n’a ni refus
« buffer pool is full », ni exception « Cannot find property b », ni annulation
inachevée dans ce journal.

Le correctif est rouge seul (376,830 s) : le CREATE_VECTOR_INDEX et le ADD b sont
refusés faute de pool ; le test continue malgré ce dernier refus et son SET b lève
l’exception de liaison. Le journal contient 1 137 occurrences textuelles de
« buffer pool is full » (ce n’est pas un compte de transactions distinctes),
et aucune de la garde d’annulation inachevée.

Ces symptômes différents interdisent de conclure « rouge identique donc indépendant ».
La pression dépend du scénario chronométré et des écritures effectivement réalisées.
Reste à comparer ces conditions et isoler le déclencheur sur une charge bornée avant
tout rejeu supplémentaire. Cette analyse reste portée par ce ticket ; aucun
nouveau lot ou correctif de DDL autorisé par ce seul constat.

### Plusieurs enregistrements undo

Huit cas `RolledBackCopiesTest.*` et `RolledBackBigCopyTest.*` verts sur la base
(1,438 s) comme dans la suite entière du correctif. Trois COPY annulés puis point
de reprise : base 0,409 s, correctif 0,470 s ; un COPY de 100 000 lignes : base
0,305 s, correctif 0,298 s. Temps gtest incluant création/reprise de base et toute
la recette, pas le coût isolé du retrait de clés ; charge et ordre non contrôlés.
Les garanties fonctionnelles sont couvertes, la mesure du coût isolé reste ouverte.

### Prochain pas et limite du verdict

- Examiner les volumes, durées et états du pool déjà journalisés aux fenêtres DDL
  des deux exécutions ; poser une comparaison bornée seulement après ce diagnostic.
- Définir la mesure isolée du retrait par plage si le risque de coût reste matériel.
- Conserver le témoin long STRING rouge sans correctif et vert avec, les ciblés
  et la suite de transactions verts, mais aucun verdict global ni tour de push
  tant que DdlEveryMinute n’est pas départagé. Ticket ouvert, pas de lib publiée.

### Lecture des fenêtres existantes (12 novembre)

Les journaux établissent une divergence de pression avant le CREATE_VECTOR_INDEX,
pas uniquement une différence de résultat final : pool configuré 2 048 Mio des
deux côtés ; pic base 1 341 Mio, pic correctif 2 052 Mio (les réservations en cours
peuvent dépasser momentanément la limite). Avant `3 CREATE REL TABLE`, le pic de
la fenêtre précédente est 1 176 Mio sur la base contre 2 051 sur le correctif.
Avant `4 CREATE_VECTOR_INDEX` : 1 177 contre 2 052 Mio.

Le CREATE_VECTOR_INDEX de la base réussit en 752 409 ms ; celui du correctif est
refusé après 172 ms. La base atteint les comptes après réouverture Doc 59 927,
Link 447 312, Note 59 683, tous égaux aux comptes attendus. Le correctif s’arrête
sur le SET b avant ce contrôle : aucun compte final comparable à inventer.
Les fenêtres hors DDL comptent 295 723 transactions doc sur la base contre
245 379 sur le correctif, avec 46 contre 57 319 erreurs : ces exécutions ne
portent pas la même charge réalisée. Leurs totaux ne constituent pas une
comparaison appariée du coût du retrait par plage.

Le nouveau retrait parcourt toutes les partitions locales pour chaque
enregistrement d’insertion annulé ; l’ancien relisait les groupes sélectionnés
et cherchait les clés. Le risque de coût sous grand index local mérite donc une
mesure bornée nombre d’entrées × nombre d’enregistrements, distincte des temps
gtest de COPY complets. C’est une hypothèse de coût à tester, pas la cause prouvée
de la divergence de pression. Aucune nouvelle compilation ni exécution de rouge
pour cette lecture ; la comparaison contrôlée reste à préparer dans ce ticket.

### Protocole borné préparé, aucune mesure déclenchée

Voir [le protocole index × plages](../12-11-2026/03-protocole-borne-index-et-annulations.md).
Porteur cœur C++, même lot. Première cellule prévue N=65 536/U=64, R=4 096
constant ; matrice plafonnée à 12 cellules, 72 processus, 15 minutes, un seul
`poste mesure`, pool 512 Mio/RSS 1 Gio. Les modes comparent des entrées identiques,
le retrait par clé étant explicitement une référence basse sans lecture de colonnes.

Les binaires existants ne paramètrent pas N × U : aucune nouvelle mesure présentée
comme réemploi fictif. Prochaine action avant créneau : préparer et relire le
pilote externe minimal, confirmer ses dépendances identiques et la possibilité
de lier l’archive de base sans rebuild moteur. Le protocole distingue plages et
INSERT_INFO ; une mesure d’annulation complète exigerait un second pilote et
un moteur du correctif assorti, à proposer seulement après la première preuve.
Sources/logs terminés conservés ; aucune lib distribuée modifiée.

### Pilote et commandes à relire préparés, sans charge lancée

Voir les trois fichiers `../12-11-2026/outils/cx-mesure-retrait.cpp`,
`cx-commandes-lien-revue.json`, `cx-premiere-cellule-revue.json`.
Première cellule **seulement** : N=65 536, U=64, R=4 096, trois paires alternées.
Les 15 minutes sont un plafond **global**, pas par cellule ; la matrice complète
reste conditionnée à l’utilité du pilote. Le pilote compare des mécanismes sur
index existant, pas les moteurs baseline/fix exacts ni leurs rollbacks.

Relecture de provenance : seule archive disponible = base, SHA256
`40dca10ed39bca8c45769f086e3bfd95fe0f6502876fc12482b33c44ddd662cc`.
La disposition identique d’une classe ne démontre pas la conformité ODR avec son
en-tête modifié. La surcouche sur archive de base proposée précédemment est
retirée : **ne pas compiler/lier** le pilote ainsi. Archive et en-têtes assortis
à `0511dece7` requis, provenance non disponible à ce point.

Les commandes préparées ne compilent que le pilote ; elles ne reconstruisent
aucun moteur copié, et restent `ready_to_compile=false`. Aucun compilateur ou
mesure exécuté. Reste : revue source et provenance, disponibilité d’une archive
assortie à traiter explicitement avec l’orchestration, puis lanceur qui impose
budget global/RSS/groupe et créneau ; pas de mesure autorisée à ce stade.
