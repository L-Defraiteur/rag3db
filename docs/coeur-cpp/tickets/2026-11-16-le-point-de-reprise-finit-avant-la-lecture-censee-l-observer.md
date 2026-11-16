# Le point de reprise finit avant la lecture censée l'observer

- **État** : ouvert (16 novembre2026), plan de témoin non atteint
- **Famille** : moteur
- **Gravité** : qualification refusée, garantie de lecture pendant checkpoint non jugée
- **Atteignable en service** : défaut de plan du témoin ; défaut moteur non établi
- **Touche rag3weaver** : indirectement, preuve des lecteurs pendant checkpoint
- **Signalé par** : cœur C++, qualification globale888
- **Pour** : cœur C++, coordination C7, hotfixs

## Constat

[Passe et preuves](../16-11-2026/11-qualification-globale-arretee-sur-deux-rouges.md), source/moteur888c81825e4cd6556261233f0e238af3b95375c9. CheckpointOpenReader.OpenReaderOnRelsIsNotWaitedFor dure2,410s et échoue checkpoint_no_stall_test.cpp358 : checkpointEndedFirst=true attendu false, [check: plan] the checkpoint ended before the read made during it. Nouveau rouge refusé par known_red ; pas de réessai.

openReaderAcrossACheckpoint ouvre un lecteur, écrit ailleurs, lance CHECKPOINT, attend30ms puis lit et relève done. Un checkpoint fini avant ce relevé rend ce plan rouge. Cette observation **ne prouve pas** qu'un lecteur attend ou perd son instantané ; les assertions de garanties suivantes ne sont pas atteintes après ASSERT_FALSE. Ne pas appeler cela une régression moteur sur ce seul résultat.

## Suite et fin

Relire synchronisation et garantie réellement visée ; préférer les primitives de test existantes à un sleep accru. Toute correction de témoin doit conserver la preuve qu'il lit pendant la fenêtre visée, avec rouge préalable et qualification sous admission neuve. Ne pas reclasser probabiliste ni affaiblir known_red automatiquement. Aucun runtime nouveau autorisé par ce ticket.
