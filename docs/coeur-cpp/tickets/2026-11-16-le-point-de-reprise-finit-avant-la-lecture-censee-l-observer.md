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

openReaderAcrossACheckpoint ouvre un lecteur, écrit ailleurs, lance CHECKPOINT, attend30ms puis lit et relève done. Un checkpoint fini avant ce relevé rend ce plan rouge. Le relevé est après read : il ne prouve pas une fin avant le début de lecture, la fin peut survenir pendant celle-ci. Le log montre les mêmes valeurs avant/pendant/après (300000|45000150000), distinctes des autres (300001|45008149994), sans assertion finale atteinte. Cette observation **ne prouve pas** qu'un lecteur attend ou perd son instantané ; les assertions de garanties suivantes ne sont pas atteintes après ASSERT_FALSE. Ne pas appeler cela une régression moteur sur ce seul résultat.

## Suite et fin

Le lecteur contrôlé de la paire50e2 est NON JOUE après l'arrêt du premier cas chaînes ; [bilan conservé](../16-11-2026/15-paire-diagnostique-arretee-sur-sigsegv-et-sonde-non-activee.md). Aucune nouvelle preuve de chevauchement ou de snapshot à ajouter au présent ticket.

Relire synchronisation et garantie réellement visée ; préférer les primitives de test existantes à un sleep accru. Toute correction de témoin doit conserver la preuve qu'il lit pendant la fenêtre visée, avec rouge préalable et qualification sous admission neuve. Ne pas reclasser probabiliste ni affaiblir known_red automatiquement. Aucun runtime nouveau autorisé par ce ticket.
