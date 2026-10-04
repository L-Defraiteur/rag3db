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
- pages d'un point de reprise raté ou d'un arrêt brutal jamais récupérées : `src/storage/checkpointer.cpp` (Vela `dfee83bf3`, Ladybug `9ca5e8913`) ;
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
- Témoins (`single_writer_crash_test.cpp`) : `RecoveryDeath.DeathBetweenTheTwoRemovalsAtRecovery` (rouge dans l'ordre ancien : « Cannot open file … db.kz.shadow »), `RecoveryDeath.ShadowFileAlreadyReplayedAndRemoved` (tolérance), `Points/CheckpointDeath…/AfterCheckpointLogged`.

**Synchronisation après le rejeu des pages fantômes** : corrigé, sans témoin.

- `ShadowFile::replayShadowPageRecords` synchronise le fichier de données avant que la reprise supprime quoi que ce soit. Le chemin normal du point de reprise le faisait déjà (`applyShadowPages`) ; seule la reprise l'omettait.
- Ce qui manque pour le prouver : un crochet de faute dans `src/` qui simule une coupure (écritures non synchronisées perdues). Une mort de processus ne suffit pas, le cache de pages du noyau survit.
- Coût mesuré à une réouverture avec rejeu (200 000 lignes, base sur btrfs) : avec la synchronisation 76, 89 et 115 ms, sans 69, 71 et 72 ms : environ 20 ms à la médiane, 45 ms au pire. Payé une fois, seulement quand la reprise rejoue des pages fantômes (mort pendant un point de reprise).

## Pour le fermer

un crochet de test dans `src/` (faute d'E/S injectée, mort à un point nommé), puis un témoin par défaut ; ou un correctif lu sur l'amont pour chacun, sans témoin, avec l'accord de la session cœur C++.
