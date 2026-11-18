# La synchronisation du répertoire d'un fichier créé ne développe pas « ~ »

- **État** : ouvert (18 novembre 2026, hotfixs, relevé par le cœur) ; correctif en cours sur la branche
  `dir-sync-expands-home`
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
