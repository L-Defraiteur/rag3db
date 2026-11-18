# La synchronisation du répertoire d'un fichier créé ne développe pas « ~ »

- **État** : fermé (18 novembre 2026, hotfixs) : `VirtualFileSystem::openFile` prend le chemin développé (6f1543219, branche `dir-sync-expands-home`).
- **Famille** : moteur
- **Gravité** : refus d'une écriture valide. Un `COPY … TO "~/…"` (ou toute ouverture qui crée un fichier sous `~`)
  échoue sur « IO exception: Failed to open directory ~ to sync it: No such file or directory ». C'est une régression
  de master depuis le lot de la durabilité (0818e6819, `syncDirectory` à la création d'un fichier).
- **Atteignable en service** : oui, par tout chemin qui commence par `~` (`COPY … TO`, `EXPORT`, une base ouverte
  par `~/…` si elle n'existe pas encore).
- **Touche rag3weaver** : par le moteur, si un chemin passé au moteur commence par `~`.
- **Pour** : hotfixs

## Ce qu'on voit

`e2e_test --gtest_filter='copy~copy_multiple_files.*'`, à luciepc, sur master da7dbf61c (bâti Release de hotfixs) :

```
EXPECT OK BUT GOT ERROR: IO exception: Failed to open directory ~ to sync it: No such file or directory
[  FAILED  ] copy~copy_multiple_files.CopyFilesWithHomeDir (30 ms)
```

Les quatre autres cas du fichier passent. Le cœur l'a vu le premier, dans une passe d'`e2e_test` entier sur sa branche
`dictionary-append-reserves-first` (master 67dd642a1, plus un correctif qui ne touche pas au système de fichiers).

## La cause

`VirtualFileSystem::openFile` (`src/common/file_system/virtual_file_system.cpp`) décide si l'ouverture crée le
fichier par `fileOrPathExists(path)`, puis synchronise `parent_path(path)`, et cela sur le chemin brut. Or
`LocalFileSystem::openFile` développe `~` (`expandPath`, par le réglage `home_directory`) avant d'ouvrir. Pour
`~/id.csv`, le répertoire synchronisé est donc « ~ », qui n'existe pas.

## Le correctif

Dans `VirtualFileSystem::openFile`, prendre le chemin développé (`expandPath`) pour le test d'existence et pour le
répertoire à synchroniser, quand il commence par `~` et qu'un contexte est donné. Rien d'autre ne change.

## Pour fermer

Le correctif sur master, `CopyFilesWithHomeDir` vert, `e2e_test` entier sans autre rouge, et les témoins `PowerCut*`
toujours verts.

## Fermeture (18 nov.)

La passe, à luciepc, sur 6f1543219 (bâti Release de hotfixs, lib bâtie de la tête) :

| Suite | Résultat |
| --- | --- |
| `e2e_test --gtest_filter='copy~copy_multiple_files.*'` | 5 verts, dont `CopyFilesWithHomeDir`, rouge sur master |
| `transaction_test` entier | 393 verts, 2 sautés (les outils à variable), dont les 9 `PowerCut*` |
| `e2e_test --gtest_filter='*'` | 1 867 verts, 1 rouge : `transaction~ddl~ddl_empty.DropNodeTableReclaim` |

`DropNodeTableReclaim` est un rouge de master, que ce lot ne touche pas. Rejoué avec `virtual_file_system.cpp` de master
(le seul fichier du lot), il échoue à l'identique : après un `DROP TABLE` puis un `CHECKPOINT`, la table recréée fait
grossir `file_info().num_pages` au lieu de réemployer les pages. C'est le ticket du cœur
`docs/coeur-cpp/tickets/2026-11-14-apres-un-drop-la-table-recreee-ne-reemploie-pas-les-pages-rendues.md`. c7 l'a admis
comme rouge connu de master le 18 novembre.

Une première passe avait lancé `e2e_test` sans `--gtest_filter` : il n'enregistre alors aucun cas (« 0 tests »).
Elle n'a pas été comptée.
