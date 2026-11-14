# Le 14 novembre au soir : les règles d'import en données, la provenance par langue

*Session codeparsers, rapport de la soirée (17 h – 23 h 20). Les pages 01 et 02 du jour en sont la
conception.*

## Ce qui a atterri sur master

| Rang | Lot | master | Pointeur codeparsers |
|---|---|---|---|
| 14 | Un import Python pointé trouve sa bibliothèque (`import requests.adapters`) | `b45056943` | `d425495` |
| 22 | Les règles d'import en données, sept langues, plus aucune liste dans le code | `7215d18ad` | `a1972c0` |
| 28 bis | La provenance par langue : manifestes par langue, `go.mod` lu et ses `replace` suivis, imports Go par l'arbre, `requirements/` et PEP 508 `@`, C# tiré de la doc .NET | `a12f08bdf` | `6ba8478` |

Chaque fichier `data/imports/<langue>.toml` dit en tête sa source, sa version, sa date et la commande qui
le relit (Python `sys.stdlib_module_names` 3.14.9 ; Node 22.23.3 ; rustc 1.98.1 ; la règle de la chaîne
Go ; C23 §7.1.2 ; libstdc++ de GCC 16.2.1 ; `dotnet/dotnet-api-docs` a010100).

## Ce qui attend son rang

- **Rang 30 quater — les renommages Python** (`codeparsers/renommages-python-2`, pointeur `85fe233`) :
  `[distributions]` dans `python.toml` (1 094 distributions, pipreqs 48dbafd Apache-2.0, plus 13 vues dans
  `packages_distributions()`), `workspace.imports` au manifeste (par langue, `replace`, refus nommé d'une
  langue inconnue). Base : la tête du dual du creux d'embarquement. Passe : check_features, la lib entière,
  e2e_code, e2e_postgres, sous `RAG3WEAVER_MOTEUR_ANCIEN=1` dit.
- **Après, dans l'ordre de c7** : les manifestes .NET (`.csproj`, `packages.config`) avec le plus long
  préfixe déclaré pour C# ; les alias de résolution (`#…` de Node, `paths` de tsconfig, `@/` de vite, en
  données et déclarables) ; `go.work` ; C/C++ (conan, `find_package`/`FetchContent`, le tiret des ports
  vcpkg par la table des renommages). Un lot par point, rouge d'abord.
- **Ticket souhait** : une crate dont `[lib] name` diffère du paquet
  (`docs/codeparsers/tickets/2026-11-14-une-crate-dont-la-bibliotheque-porte-un-autre-nom.md`).

## La règle du faux rouge par artefact repris

Deux fois ce soir, un témoin a jugé l'ancien code : cargo juge un paquet local frais par ses dates, et
- `cp -a` remettait les fichiers neufs avec leurs anciennes dates ;
- un `cargo test` lancé à la main dans un autre worktree, sur le même target, reprenait l'artefact
  codeparsers bâti ailleurs (le checkout de `85fe233` plus vieux que le bâti de `6ba8478`).

`RAG3WEAVER_TARGET_REPRENDRE=1` n'agit que dans les scripts de passe (check_features, run_e2e.sh). **La
règle** (c7, 14 nov. 23 h 1x) : avant un bâti hors script de passe, `touch` des fichiers que nomme
`git diff --name-only <ancien> <neuf>` du sous-module, et des sources rag3weaver changées — ou passer
par check_features, qui le fait. Restaurer une copie par `cp` sans `-a`. Dit à G pour la garde du target.

## Reprendre

- Le seul target de la session : `~/.cache/rag3weaver-build/target-import-pointe-pile` (à effacer en fin de
  session). Les journaux des passes : `~/.cache/rag3weaver-build/archives/codeparsers-*-14-nov`.
- Le worktree du lot en attente : `~/.cache/rag3weaver-build/wt-renommages`, branche
  `codeparsers/renommages-python-2`. Au mot de la base : `rebase --onto <base> a12f08bdf2` sous un nom
  neuf, puis la passe, puis le dernier pas en mode pile (`PILE_AVANT=<base>`).
