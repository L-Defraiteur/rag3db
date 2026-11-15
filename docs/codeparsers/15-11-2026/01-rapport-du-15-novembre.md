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

## Le soir du 15 et la nuit du 16 : la pile de la résolution d'arêtes

L'orchestration a rangé la famille de la résolution d'arêtes par ce qu'une réponse fausse coûte à
l'usage de Lucie. Quatre lots en pile, chacun rouge d'abord, sondés avant/après par
`tests/sonde_marque_nom.rs` (en mémoire, sur le corpus figé de rag3db-garde). Rien n'est sur master : la
pile passe en une fois, au « à toi » de c7.

| lot | branche rag3db, tête | codeparsers | ce qu'il fait |
|---|---|---|---|
| homonymes + paramètres C++ | `codeparsers/homonymes-et-parametres`, d1c7961c7 | ff82d22 | un uuid par homonyme ; un paramètre C++ par pointeur ou référence garde son nom |
| la marque « nom » | `codeparsers/marque-nom`, 7d2ef159a | fad1b6f | une variante lit son type par le chemin ; une méthode std sur une locale sans type lu ne se relie pas par le nom entre fichiers (listes `data/std_methods`, générées depuis le poste) |
| les receveurs sans type | `codeparsers/receveurs-chaines`, f1bae1e34 | 65b1a93 | une chaîne de méthodes du projet sur un type connu se lit (`DeferredType::OfType`) |
| l'appel C vers l'API C | `codeparsers/prototype-macro`, d7dffe335 | 7702f03 | une source `.c` relève ses `#include` ; la garde `extern "C"` de rag3db.h ne perd plus ses 167 prototypes |

Les chiffres sont dans les tickets : [la marque « nom »](../../tickets/closed/2026-11-07-la-marque-nom-relie-un-appel-de-methode-standard-a-un-homonyme.md)
(moteur : 2 329 fausses arêtes ôtées ; rag3weaver : 646, dont une vingtaine de justes, le prix) ;
[les receveurs sans type](../tickets/2026-10-04-des-receveurs-restent-sans-type.md) (rag3weaver : 864
« nom » → « type », 1 071 couples nés, 178 partis, presque tous des `collect` std reliés à tort) ;
l'appel C (src, examples et test : 27 couples nés, 0 partis).

Ce qui s'apprend :

- **Une règle se mesure sous sa forme large avant de se rétrécir** : « toute locale sans type lu
  s'abstient » ôtait 959 liens justes de plus sur rag3weaver ; la forme étroite (méthodes std seules) a
  été choisie sur ce chiffre.
- **Une cause annoncée se vérifie avant la correction** : le ticket d'ED a d'abord été attribué à la
  macro d'export (déjà masquée) ; une trace temporaire de la visibilité a montré `main.c` sans aucun
  include, puis l'en-tête sans aucun prototype.
- **Une lecture neuve est « au mieux »** : `OfType`, non lu, doit laisser l'appel à la règle d'avant ; en
  faire un receveur non typé ôtait des liens « nom » justes (le témoin de `n.aller()` l'a vu).
- **Une sonde en mémoire a l'enveloppe pour plafond** : l'arbre entier de rag3db-garde en un lot a été
  tué par l'OOM à 32 Go ; une racine de liens symboliques (src, examples, test) suffit à la mesure.
- **Identifiants en anglais** pour ce qui vient (Lucie, 16 novembre, 1 h) ; la prose reste en français.

## Reprendre

- **La pile de quatre lots attend le « à toi » de c7**, après le cœur, l'écrivain d'ED, le lot court
  d'archi, 2b′ et 2b-C. Au mot : rebase sous un nom neuf sur la base donnée, réécriture et fermeture du
  ticket d'ED (`2026-11-15-un-appel-c-vers-une-fonction-cpp-a-definisseur-unique-n-est-pas-resolu.md`,
  cause : les includes d'une source C et la garde `extern "C"`), montée de REGLES_DE_L_ANALYSE, passe
  complète (check_features, lib entière, e2e_code, e2e_postgres), dernier pas par la garde en mode pile.
  Les worktrees : `wt-homonymes`, `wt-marque-nom`, `wt-receveurs`, `wt-proto` sous
  `~/.cache/rag3weaver-build/`, propres.
- **Après la pose** : dire à banc que le pointeur est sur master (ses chiffres DEFINED_IN des bases à
  ajouter au ticket fermé) ; la suite du ticket des receveurs (motif de tuple, locale redéclarée) ; la
  définition liée à l'en-tête ; le membre platform.
- **Le target** `~/.cache/rag3weaver-build/target-import-pointe-pile` (31 Go) se garde jusqu'au mot de
  Lucie sur la fin de session ; c'est le seul de la session.
- **Les journaux** de chaque passe sont sous `~/.cache/rag3weaver-build/archives/codeparsers-*-14-nov` et
  `…-15-nov`.
- **Les branches** codeparsers de la nuit (dépôt du sous-module) : `renommages-python`, `manifestes-dotnet`,
  `alias-resolution`, `appartenance-des-methodes`, toutes descendantes l'une de l'autre jusqu'à `5885e13`,
  le pointeur de master.
- **La suite possible**, dans l'ordre de l'orchestration : le dossier d'en-têtes vcpkg dès qu'une source
  existe ; la table port → paquet CMake (souhait) ; la relecture des manifestes quand l'un d'eux change.
