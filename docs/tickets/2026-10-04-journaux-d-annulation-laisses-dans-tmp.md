# Les journaux d'annulation du dataflow restent à vie dans /tmp, qui est de la mémoire vive

- **État** : ouvert
- **Gravité** : perte (de mémoire vive : une fuite qui grossit à chaque indexation)
- **Atteignable en service** : oui — toute ingestion en mode de points de reprise `Full` (le défaut)
- **Touche rag3weaver** : oui (c'est son code)

## Ce que c'est

À la fin d'un graphe, rag3weaver supprime les entrées et sorties sérialisées
de ses nœuds mais garde les fichiers `*.undo.json` de plus de 64 Ko, et rien
ne les supprime ensuite. Ils vivent dans `/tmp/rag3weaver-checkpoints/`, et
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

Aucun pour l'instant. Un test à écrire : une synchronisation de quelques
fichiers dans un `checkpoint_dir` temporaire, puis vérifier qu'il est vide
après la fin.

## La cause

Repérage dans le code (lecture, pas de mesure) :

- `checkpoint_store.rs`, `nettoyer_apres_fin` (l. 184-199) supprime entrées et
  sorties mais garde les `*.undo.json` de plus de 64 Ko (l. 162-169) ;
- l'annulation de `LinkRecordNode` contient tous les couples `{from, to}` du
  vidage (`record_nodes.rs:1057-1064`), d'où des fichiers de 100 Mo et plus ;
- le dossier par défaut est `temp_dir()` ; le cache lucivy aussi
  (`catalog.rs:415`), supprimé seulement au `Drop`, donc laissé par un
  processus tué.

## Ce qu'il faut pour le fermer

- Décider de la durée de vie d'une annulation : à quoi sert un
  `links.undo.json` une fois le graphe fini et validé ? Si rien ne le relit,
  le supprimer avec le reste.
- Mettre les points de reprise et le cache lucivy sous un dossier sur disque
  (à côté de la base, ou `~/.cache`), pas dans `temp_dir()`.
- Nettoyer à l'ouverture les dossiers de pid morts.
- Le nettoyage des 5,5 Go déjà là est à faire à la main ; d'autres sessions
  écrivent dans le même dossier, donc pas pendant qu'elles tournent.
