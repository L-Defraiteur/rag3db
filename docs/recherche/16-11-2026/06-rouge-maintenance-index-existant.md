# Rouge observé : la maintenance ne lit pas l’index existant fermé

Source tests seule `187d58a7b5ebb8e4a0a59b9b57ed75623e9db9a2`, base68590270 ; pilote final `3a528b67921f3c3b698e6d6db593bf8d4f834516`, code3dbcef4d40. GO distinct C7 après arrêt d’entrée v1 conservé, sans retry de ce reçu.

Une seule fonction exécutée : `catalog::tests::existing_fts_live_ids_reads_closed_files_without_writer_or_changes`. Compilation réussie125.018966s, exécution0.064099s ; résultat brut **0 passé / 1 échoué / 0 ignoré**, 1846 filtrés. Cause au point d’appel attendu :

```
existing files must be readable without warmup: "pas d'index plein texte ouvert pour « Document » (ouverts : [])"
```

Le test a construit puis fermé son writer privé avant l’appel. Ce rouge concerne bien l’ouverture manquante, sans dépendre d’un nouveau préfixe d’erreur. Les quatre autres fonctions restent NON JOUÉES ; réserve C7 du négatif Document consignée, à corriger avant implémentation/full. Aucune implémentation automatique après le rouge.

Superviseur PID3621239/start117319365, début1173193.710458657, fin globale observée125.390661s<1200, échéance1174393.710458657. Cgroup réel max34359738368/highmax/swap0/cpu800000/100000. Source propre, profil minimal sans défaut features=[rag3db-native], lib59ac complète exportée une fois, pin source/cache2150fcd847b1df7de04908d357a0c78abc05e50ce599e2793ff08ab48108825d. Ledger et inventories publiés avant invocation ; ELF SHA256 `e4a7b7228bb7cce7047baf7c4eeaeec375224325295132239015596a5de84723`, 186186784octets, toujours présent dans le target de session.

Preuves brutes archivées PC et local dans `~/.cache/rag3weaver-build/cx-archive-recherche-fts-existing-red-v2` : **25fichiers**, manifeste SHA256 `a3519cc5325b675e7d4c4eff3c4d9061aa380175a8971ee1ced34fcbfa2b8110`, réhash local25/25 identiques. Entrée, profil natif/cgroup, commandes, logs JSON compilation, inventories/ledger, log du juge, reçu/recette/verdict/enfants/bus et contrôleurs inclus. Aucun verdict recomposé : verdict commun **ROUGE**, code101, erreur=null, restes=[], nettoye=true, autorité terminée.

Poste Recherche libéré ; TSan et Archi conservés. Target de session gardé1354861603octets, aucune purge/nettoyage manuel, aucun service/modèle/base de Lucie ouverts. Ce résultat n’est pas une qualification verte ; suite produit à autoriser/revoir distinctement.
