# Durabilité sur faute d'entrée-sortie, coupure ou mort au mauvais instant

- **État** : ouvert
- **Gravité** : perte (pour la plupart)
- **Atteignable en service** : non au banc sans crochet dans src/
- **Touche rag3weaver** : oui en cas d'incident (disque plein, coupure)
- **Ouvert le** : 4 octobre 2026, revue des amonts
- **Pour** : cœur C++ (journal, reprise)

## Ce que c'est

Dix défauts corrigés chez Vela ou Ladybug qui ne se voient qu'avec un disque plein, une coupure de courant ou une mort placée entre deux appels. Le tableau de la revue les détaille (§3).

## Recette minimale

```cypher
-- pas de recette Cypher. Exemples :
-- a) disque plein pendant une validation, espace libéré, nouvelles validations, réouverture :
--    les validations suivantes sont derrière un enregistrement déchiré.
-- b) mort entre la suppression du fichier fantôme et celle du journal, à la reprise :
--    la base ne s'ouvre plus.
```

## Témoin

aucun (non atteignable sans crochet)

## Cause

- validation publiée en mémoire avant l'écriture du journal : `src/transaction/transaction.cpp:63-71` (Vela `a96f7a7d9`, Ladybug `b824045f0`) ;
- répertoire jamais synchronisé après création ou suppression du journal et du fichier fantôme : `src/storage/wal/wal.cpp`, `src/storage/shadow_file.cpp` (Vela `645d68973`) ;
- rejeu des pages fantômes non synchronisé avant la suppression du journal : `src/storage/shadow_file.cpp:134-141` (Vela `e5e700e73`, Ladybug `e2ada3f90`) ;
- suppression du fichier fantôme avant le journal à la reprise : `src/storage/wal/wal_replayer.cpp:752-755` (mêmes commits) ;
- pages d'un point de reprise raté ou d'un arrêt brutal jamais récupérées : `src/storage/checkpointer.cpp` (Vela `dfee83bf3`, Ladybug `9ca5e8913`). **Témoin depuis le 4 octobre au soir** : `JournaledCopyDeath.DeathInTheMiddleOfACopyLeavesNothingOfIt` (`data-file-bounded`) — trois morts au milieu d'un COPY d'un million de lignes font passer le fichier de données de 1,8 à 5,5–7,8 Mo, sur le chemin journalisé comme sur l'ancien ; rien n'est perdu ni lu faux ;
- compteur de pages libres sur une plage libérée deux fois : `src/storage/free_space_manager.cpp:34-39` (Ladybug `9ca5e8913` ; aucune double libération trouvée chez nous) ;
- point de reprise échoué après un DROP qui laisse les colonnes compactées : `src/storage/table/node_table.cpp:808-815` (Ladybug `22ee54a71`) ;
- verrou d'état de page et file d'éviction : `page_state.h:42-52`, `buffer_manager.cpp` (Ladybug `7e4248202`, `eff87c1e1`, `d6adbd2b7`) ;
- frontière d'algorithme de graphe dimensionnée hors instantané : `src/function/gds/gds_frontier.cpp:90-93` (Ladybug `1a233d280`) ;
- madvise sur des pages de 16 ou 64 Ko (aarch64) : `src/storage/buffer_manager/vm_region.cpp:69` (Ladybug `bb52f183c`).

## Correctif de l'amont (à lire, ne pas copier)

voir chaque ligne ci-dessus.

## Avancement (4 octobre 2026, banc)

Deux lignes de la cause sont traitées ; le ticket reste ouvert pour les autres.

**Ordre des suppressions à la reprise** : corrigé, avec témoin.

- `WALReplayer::removeWALAndShadowFiles` supprime le journal d'abord, le fichier fantôme ensuite. Une mort entre les deux laisse un fichier fantôme sans journal, que la reprise suivante supprime déjà (chemin « pas de journal »).
- Tolérance de l'ordre ancien : si le journal finit par CHECKPOINT et que le fichier fantôme manque (laissé ainsi par une mort entre les deux suppressions dans l'ordre ancien), la reprise saute le rejeu des pages au lieu d'échouer.
- Crochet de test dans `src/` : `WALReplayer::setRecoveryHookForTesting`, appelé à des points nommés, sans effet hors des tests (aucun crochet posé) et jamais en lecture seule. Points posés (`RecoveryPoint`) :
  - `SHADOW_PAGES_REPLAYED` : pages fantômes recopiées et synchronisées, rien supprimé ;
  - `JOURNAL_REMOVED` : journal supprimé, fichier fantôme pas encore.
- Côté banc, un point de mort de plus pendant le point de reprise : `DeathPoint::AfterCheckpointLogged` (journal clos par CHECKPOINT, pages fantômes pas encore appliquées), qui produit l'état qu'une reprise doit rejouer.
- Témoins (`single_writer_crash_test.cpp`) : `RecoveryDeath.DeathBetweenTheTwoRemovalsAtRecovery` (rouge dans l'ordre ancien : « Cannot open file … db.kz.shadow »), `RecoveryDeath.ShadowFileAlreadyReplayedAndRemoved` (tolérance), `Points/CheckpointDeath…/AfterCheckpointLogged`, `Points/CheckpointDeath…/AfterJournalCleared` (journal vidé par le point de reprise ordinaire, fichier fantôme encore plein : la reprise le supprime sans le rejouer, les pages étant déjà recopiées et synchronisées).
- Limite de la tolérance : elle couvre une mort de processus dans l'ordre ancien, pas une coupure de courant. L'ancien code recopiait les pages sans les synchroniser ; après une coupure à cet instant, des pages peuvent manquer et la tolérance les croirait appliquées. Hors du périmètre de la stèle (arrêts au mauvais instant, pas coupures).

**Synchronisation après le rejeu des pages fantômes** : corrigé, sans témoin.

- `ShadowFile::replayShadowPageRecords` synchronise le fichier de données avant que la reprise supprime quoi que ce soit. Le chemin normal du point de reprise le faisait déjà (`applyShadowPages`) ; seule la reprise l'omettait.
- Ce qui manque pour le prouver : un crochet de faute dans `src/` qui simule une coupure (écritures non synchronisées perdues). Une mort de processus ne suffit pas, le cache de pages du noyau survit.
- Coût mesuré à une réouverture avec rejeu (200 000 lignes, base sur btrfs) : avec la synchronisation 76, 89 et 115 ms, sans 69, 71 et 72 ms : environ 20 ms à la médiane, 45 ms au pire. Payé une fois, seulement quand la reprise rejoue des pages fantômes (mort pendant un point de reprise).

**Les pages d'un COPY tué : la fuite n'est pas bornée** (5 octobre 2026, banc, mesuré, non corrigé).

- Sonde non commitée (`~/.cache/rag3db-banc-notes/sonde-fuite-copy.patch`, sur la fixture `JournaledCopyDeath`). Elle tue 12 fois un COPY d'un million de lignes, rouvre, fait un point de reprise, puis écrit 200 000 lignes ordinaires et fait un point de reprise. Même chose sur l'ancien chemin et sur une base sans morts.
- Chaque mort laisse entre 0,3 et 3,4 Mo, selon l'avancement du COPY. Après 12 morts, le fichier de données passe de 28 Ko à 14,6–25,9 Mo sur le chemin journalisé, et à 14,9–16 Mo sur l'ancien (deux passes). La croissance est linéaire, sans palier : bornée par mort (la taille du COPY), pas par le nombre de morts.
- Rien ne la rend : ni la réouverture, ni le point de reprise, ni les écritures qui suivent. Les 200 000 lignes ordinaires font grossir le fichier de 13,2 Mo après les morts, contre 14,3 Mo sans morts : au mieux 1,1 Mo réutilisé, sur 15 à 26 Mo perdus. Les pages écrites par un COPY non validé ne sont connues d'aucun gestionnaire de pages libres après la reprise.
- Aucune donnée perdue ni lue faux : la base reste intègre (niveaux 1 et 2) après chaque mort.
- **Classement : confort pour la stèle** (orchestration, 5 octobre). Personne n'y perd une donnée ni ne lit un résultat faux : c'est de l'espace. Ordre de grandeur : mille morts en plein chargement font de 1 à 3 Go.
- La page du chargement journalisé (`extension/rag3weaver/docs/3-octobre-2026-23h31/coeur-cpp/04-le-chargement-en-masse-journalise.md`, §3, « Un point à tenir ») déduisait que ces pages seraient « reprises naturellement », à éprouver par `data-file-bounded`. La sonde réfute cette déduction, sur les deux chemins.
- Piste de remède, non codée : à la réouverture, les pages au-delà de ce que le dernier point de reprise connaît du fichier sont libres par définition, puisque le journal porte des lignes et non des pages. Il faut les rendre au gestionnaire de pages libres avant que le rejeu n'en prenne, car le rejeu écrit lui-même des pages neuves. Pour la session cœur C++, à l'étape 4 du chargement journalisé : c'est le même terrain.

## Pour le fermer

un crochet de test dans `src/` (faute d'E/S injectée, mort à un point nommé), puis un témoin par défaut ; ou un correctif lu sur l'amont pour chacun, sans témoin, avec l'accord de la session cœur C++.
