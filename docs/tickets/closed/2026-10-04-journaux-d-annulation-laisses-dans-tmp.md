# Les journaux d'annulation du dataflow restent à vie dans /tmp, qui est de la mémoire vive

- **État** : corrigé
- **Gravité** : perte (de mémoire vive : une fuite qui grossit à chaque indexation)
- **Atteignable en service** : oui — toute ingestion en mode de points de reprise `Full` (le défaut)
- **Touche rag3weaver** : oui (c'est son code)
- **Famille** : produit

## Ce que c'est

À la fin d'un graphe, rag3weaver supprime les entrées et sorties sérialisées
de ses nœuds mais gardait les fichiers `*.undo.json` de plus de 64 Ko, et rien
ne les supprimait ensuite. Ils vivent dans `/tmp/rag3weaver-checkpoints/`, et
`/tmp` est un tmpfs sur ce poste : chaque fichier occupe de la mémoire vive
jusqu'au redémarrage.

Constaté le 4 octobre 2026 vers 13 h : **5,5 Go** dans
`/tmp/rag3weaver-checkpoints` (4 128 fichiers `*.undo.json`, dont des
`links.undo.json` de 28 à 148 Mo), plus **231 Mo** de caches lucivy laissés
par des processus morts dans `/tmp/rag3weaver_cache/<pid>/`.

## La recette

Une synchronisation de dépôt quelconque (`sync_source`), puis
`du -sh /tmp/rag3weaver-checkpoints` : la taille ne redescend pas à la fin du
processus. Les vidages de liens laissent les plus gros fichiers.

## Le témoin

Test unitaire `dataflow::checkpoint_store::tests::fin_de_graphe_supprime_les_fichiers_undo_meme_sup_64ko` dans `extension/rag3weaver/src/dataflow/checkpoint_store.rs`.
Il pose un journal d'annulation de plus de 64 Ko pour une exécution ordinaire (`sync-source-fichiers`), appelle `nettoyer_apres_fin` et vérifie que les fichiers `*.undo.json` ne subsistent pas.

1. **Rouge avant correctif** :
```
test dataflow::checkpoint_store::tests::fin_de_graphe_supprime_les_fichiers_undo_meme_sup_64ko ... FAILED
panicked at src/dataflow/checkpoint_store.rs:1592:9: assertion `left == right` failed: les fichiers *.undo.json de plus de 64 Ko ne doivent pas subsister après la fin du graphe — left: 1, right: 0
test result: FAILED. 0 passed; 1 failed
```

2. **Vert avec le correctif** :
```
test dataflow::checkpoint_store::tests::fin_de_graphe_supprime_les_fichiers_undo_meme_sup_64ko ... ok
test result: ok. 1 passed; 0 failed
```

3. **Module complet validé** :
```
test result: ok. 9 passed; 0 failed.
```

## La cause

`checkpoint_store.rs`, `nettoyer_apres_fin` filtrait les exécutions « éphémères »
via une liste blanche incomplète restreinte à `ingest-` et `drain-`, conservant par
défaut les fichiers `*.undo.json` pour toute autre exécution (dont les synchronisations
`sync_source` et autres graphes de calcul).

## La correction

- Dans `checkpoint_store.rs`, la règle est corrigée au plus étroit : `annulation_doit_survivre`
  réserve la conservation de l'annulation après succès aux seules migrations (`migration-…`),
  car elles seules doivent pouvoir être annulées ultérieurement par un processus neuf via
  `MigrationRunner::rollback`.
- Pour tout autre graphe qui finit, tous les fichiers `*.undo.json` sont supprimés avec le reste,
  quelle que soit leur taille.
- Piste notée pour plus tard : purger également le dossier sur disque lors d'un `delete()`
  explicite sur le magasin de checkpoints.
