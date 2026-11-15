# Le 15 novembre, la nuit : la provenance des imports dans tous ses manifestes

*Session codeparsers, rapport de la nuit du 14 au 15 novembre (00 h – 02 h 05), après
[le rapport du 14 au soir](../14-11-2026/03-rapport-du-14-novembre.md). Tout est passé par la file en pile
de l'orchestration, chaque lot sur la tête du précédent, poussé par la garde en mode pile.*

## Ce qui a atterri sur master

| Lot | master | Pointeur codeparsers | Ce qu'il change |
|---|---|---|---|
| Les renommages Python | `e519cfd69f` | `85fe233` | `PyYAML` s'importe `yaml` : 1 094 distributions tirées de pipreqs, plus `workspace.imports` au manifeste (par langue, `replace`, refus nommé d'une langue inconnue) |
| Les manifestes .NET et les `using` C# | `75492be97a` | `4194456` | `.csproj`, `packages.config`, `Directory.Packages.props` ; un `using` a sa bibliothèque, le paquet NuGet au plus long préfixe ; un alias lie son nom |
| Les alias de résolution | (le même push) | `7c2c5aa` | `#…` de Node, les `paths` de tsconfig, `local_prefixes` déclarables |
| L'appartenance des méthodes | `f257b531b0` | `5885e13` | le récepteur Go générique, le nom court d'un bloc `impl`, `parent_path`, `extends_type`, le genre `method` d'une définition C++ hors classe, IMPLEMENTS sous toutes ses formes |
| `go.work`, conan et CMake | `fc206cba69` | (inchangé) | `go.work` lu comme `go.mod` ; `[requires]`, `requires`, `find_package`, `FetchContent_Declare` |
| Les manifestes Python et Node | (le même push) | (inchangé) | `setup.cfg`, `setup.py`, `Pipfile`, `[dependency-groups]`, `bundledDependencies` |

Deux colonnes neuves sur `Scope` (`parent_path`, `extends_type`), vides sur une base d'avant : la migration
additive des champs déclarés pose le défaut typé du dialecte (`''`), le même mécanisme que 2a′-1. Les
définitions C++ hors classe qui deviennent `method` changent de version une fois (ED en est prévenu).

## Ce qui reste ouvert

Les tickets de la nuit, sous `docs/codeparsers/tickets/` :
- **le genre d'une méthode définie dans un autre fichier** que sa classe (pour ED et archi : il se décide à
  la publication, par l'écrivain, ou la navigation le lit sans le genre) ;
- **le dossier d'en-têtes d'un port vcpkg** n'est écrit nulle part dans le registre (source à trouver ; un
  port garde son nom, on ne coupe pas au tiret) ;
- **rapprocher un port vcpkg de son `find_package`** (souhait) ;
- **une crate dont la bibliothèque porte un autre nom** (souhait).

Plus anciens et toujours ouverts : relire les manifestes quand l'un d'eux change ; l'oracle (témoins alias
et instance) ; le membre externe `platform` ; une mention d'un import ES externe sans cible ; les
receveurs sans type ; et ceux d'octobre listés dans le dossier. Le ticket des listes de builtins figées dans
le code se ferme avec ce rapport.

## Ce que la nuit a appris

- **Un témoin de bout en bout trouve ce qu'un témoin unitaire ne voit pas** : celui de Go rendait
  `libraries = []` (le parseur Go lisait ses imports avec les expressions de JavaScript), celui de C# aussi.
  Les deux défauts ont été corrigés dans le lot qui les a vus.
- **Lire avant d'affirmer** : j'ai dit qu'une colonne ajoutée valait NULL sur les lignes d'avant ; le
  témoin demandé par l'orchestration a montré `''` (le défaut typé de `alter_add_column`). Un ticket inutile
  a été évité.
- **Une source se lit avant d'écrire le script qui la tire** : le registre vcpkg ne donne pas le dossier
  d'en-têtes d'un port ; la table faite à la main a été refusée comme une devinette.
- **La règle du faux rouge par artefact repris** tient toujours (page du 14) ; le contrôle du worktree
  compte maintenant le retard par `--cherry-pick` (G).

## Reprendre

- **Aucun lot en attente** : la file de codeparsers est vide.
- **Le target** `~/.cache/rag3weaver-build/target-import-pointe-pile` (31 Go) se garde jusqu'au mot de
  Lucie sur la fin de session ; c'est le seul de la session.
- **Les journaux** de chaque passe sont sous `~/.cache/rag3weaver-build/archives/codeparsers-*-14-nov` et
  `…-15-nov`.
- **Les branches** codeparsers de la nuit (dépôt du sous-module) : `renommages-python`, `manifestes-dotnet`,
  `alias-resolution`, `appartenance-des-methodes`, toutes descendantes l'une de l'autre jusqu'à `5885e13`,
  le pointeur de master.
- **La suite possible**, dans l'ordre de l'orchestration : le dossier d'en-têtes vcpkg dès qu'une source
  existe ; la table port → paquet CMake (souhait) ; la relecture des manifestes quand l'un d'eux change.
