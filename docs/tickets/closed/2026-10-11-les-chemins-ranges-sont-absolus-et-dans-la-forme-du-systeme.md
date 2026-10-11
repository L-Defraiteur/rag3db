# Les chemins rangés sont absolus et dans la forme du système, et les comparaisons supposent « / »

- **État** : corrigé (12 octobre 2026, optimiseur, lot B4, branche `chemins-relatifs`) : identité relative en « / » par `workspace_path`, une racine locale par base (source nommée pour une seconde), comparaisons normalisées, base d'avant la règle refusée et dite ; témoins `workspace_path`, `e2e_base_par_arbre` (reflink, delta seul) et les e2e du code passés au relatif
- **Gravité** : blocage sous Windows (doublons et lignes périmées, filtres qui ne trouvent
  rien), **sécurité** (`probable_secret` n'exclut plus `config\.env` ni `keys\id_rsa`)
- **Atteignable en service** : oui, sur tout poste Windows. Sous Linux, l'identité absolue
  empêche aussi deux copies d'un même dépôt de partager une identité de fichier.
- **Touche rag3weaver** : oui (`code.rs`, `code_tools.rs`, `code_sync.rs`, `watch.rs`,
  `generated.rs`, `dataflow/*`), et codeparsers (`get_relative_path`)

## Ce que c'est

`File.path`, `Scope.file_path` et les clés de scope valent `root.join(rel)` : un chemin absolu,
dans la forme du système. Les chaînes relatives naissent en cinq endroits sans normalisation,
plus un dans codeparsers. Une vingtaine de comparaisons supposent ensuite « / ». Sous Windows :

- `indexed_name` (edit_file, reingest, remove_file) fait `root.join(saisie)` et donne
  `C:\r\src/a.rs`. Ce chemin ne retrouve pas la ligne `C:\r\src\a.rs` de la synchro, d'où des
  doublons et des lignes périmées.
- `probable_secret` prend le nom par `rsplit('/')` : `config\.env` n'est plus exclu.
- Les préfixes de `grep_files` et `list_files`, les `paths` de la montre, le filtre `path`
  d'usages, l'impact d'un fichier, `note_change`, `generated.rs` et `module_designe_fichier`
  ne trouvent rien, ou se dégradent sans le dire.

Le relevé complet, avec les fichiers et les lignes, est dans les notes de G
(`~/.cache/rag3weaver-build/notes-G/releve-separateurs-2026-10-11.md`). Il sera repris dans le
commit qui ferme.

## La règle (décision (a))

Tout chemin rangé en base, comparé ou rendu par un outil est relatif à la racine de l'espace de
travail et écrit en « / ». La forme du système n'apparaît qu'au moment de toucher le disque :
ouvrir, lister, écrire. Dans une requête, elle ne passe que par `cypher_path_literal`. L'absolu ne
reste que dans `absolute_path`, dans la forme du système, et n'est jamais comparé.

L'identité relative est aussi ce que demande la page des manques : « une base par arbre née d'un
reflink de celle de master puis tenue par le delta » exige qu'un fichier ait la même identité
dans deux copies du dépôt.

## Le témoin

Il manque. À écrire : un index bâti sur des chemins Windows, simulé par la fonction de
normalisation testée avec des « \ ». Il doit ranger des clés en « / », et le filtre
`path=src/storage` doit trouver. La CI Windows du paquet le jouera pour de vrai.

## Ce qu'il faut pour le fermer

- Un seul helper pour la chaîne relative, appelé aux cinq sites et dans codeparsers.
- Un seul constructeur de l'identité, qui fond `code.rs` (`analyze_in_project`) et
  `indexed_name`.
- Les comparaisons sur la forme normalisée.
- L'état d'index qui dit « base d'avant la règle, à réindexer » quand il ouvre une base aux clés
  absolues, au lieu de doublons silencieux. La réindexation des bases de code est acceptée.
