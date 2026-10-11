# Les marques CONSUMES sur rag3weaver, avant et après

*11 octobre 2026, après-midi, session codeparsers. La mesure que la ligne B9 du plan
« indexer nos propres dépôts » disait non faite (« deux passes de sonde »). Elle a trouvé trois
défauts, corrigés dans la foulée (dernière section).*

## Le protocole

`tests/sonde_marques.rs` (rag3weaver), indexation en mémoire, embarqueur de hachage. **Un seul
corpus** pour toutes les passes : `src/` et `tests/` de rag3weaver copiés une fois (283 fichiers
`.rs`, 8,7 Mo) sous `~/.cache/codeparsers-notes/b2d/corpus-rw/`, la sonde pointée dessus
(`SONDE_RACINE`, changement local non commité). Sans cela, chaque arbre aurait indexé son propre
`src/`, et la différence de corpus se serait lue comme un changement d'analyse.

| passe | rag3db | codeparsers | ce qui change depuis la colonne d'avant |
|---|---|---|---|
| début de nuit | `d6a2827ce` | `55c27d1` | (la référence de la page 03) |
| master 13 h 52 | `2306d30f9` + lot 5 | `35faa0f` | B9 (T1-T5), B2d (chemins sur un type, receveurs non typés visibles, lien « nom » dans le fichier), prototypes C ; le lot 5 (C++) ne touche pas le Rust |
| master 14 h 51 | `fd10b8c27` | `9531860` | côté rag3weaver : les réexportations (`50bdc57e3`), la racine relative |
| lot 6 | `35e3949b1` (`nom-seul`) | `a6111cc` | le nom seul et les segments de module, seul (la branche = `fd10b8c27` + le pointeur) |

## Les arêtes, par marque

| marque | début de nuit | master 13 h 52 | master 14 h 51 | lot 6 |
|---|---|---|---|---|
| type (receveur typé) | 3 521 | **4 692** | 4 692 | 4 692 |
| import | 3 611 | **7 008** | 7 297 | 6 828 |
| fichier | 13 396 | 13 420 | 13 420 | 13 319 |
| nom (deviné) | 11 338 | 13 408 | 13 419 | **11 549** |
| total | 31 866 | 38 528 | 38 828 | 36 388 |

## Les arêtes « nom », par forme de la référence

| forme | début de nuit | 13 h 52 | 14 h 51 | lot 6 |
|---|---|---|---|---|
| nom seul (type, valeur) | 3 188 | 5 597 | 5 423 | 5 127 |
| nom par chemin de module (`m::T`) | 2 557 | 3 111 | 3 025 | **1 453** |
| chemin par un type (`T::f`) | 1 087 | **175** | 175 | 175 |
| receveur : variable (`v.f()`) | 1 241 | 1 260 | 1 260 | 1 261 |
| receveur : chaîne d'appels | 1 179 | 1 174 | 1 174 | 1 173 |
| receveur : chaîne sur plusieurs lignes | 796 | 791 | 791 | 790 |
| appel nu (`f()`) | 514 | 528 | 512 | 512 |
| nom absent de la ligne | 0 | 3 | **300** | 300 |
| receveur : champ d'une variable | 320 | 299 | 299 | 299 |
| appel par chemin de module (`m::f()`) | 262 | 261 | 251 | 251 |
| receveur : champ (`self.x.f()`) | 99 | 104 | 104 | 104 |
| receveur : `self` | 59 | 59 | 59 | 58 |
| chemin par `Self` | 36 | 45 | 45 | 45 |

## Ce qui se lit

- **Les chemins sur un type** (`T::f`, codeparsers `f91fc58`) : 912 arêtes « nom » deviennent
  sûres ; « type » gagne 1 171.
- **Les imports doublent** (3 611 → 7 008) entre le début de nuit et 13 h 52, puis +289 avec les
  réexportations.
- **Le lot 6, prouvé arête par arête** (vidage complet des CONSUMES des deux passes, différence) :
  2 480 retirées, 40 ajoutées.
  - « nom » −1 875 : les segments de module (`m::T` : `m` n'est plus une référence) et le
    paramètre d'un bloc `impl`.
  - « import » −469 : **468 visaient une déclaration de module** (`pub mod node_registry;` de
    `dataflow/mod.rs`, 431 ; `tests/common/mod.rs`, 35 ; `daemon/mod.rs`, 2) depuis un chemin
    `crate::dataflow::node_registry::X`. C'est l'objet du lot. La 469ᵉ
    (`Catalog` → `checkpoint.rs::execution_id`) reste à regarder.
  - « fichier » −135 : un bloc `impl` vers ses propres méthodes par un paramètre homonyme
    (`Arc<T>` → `run`).
  - Les 40 ajoutées : 34 « fichier » (`EditFileNodeFactory` → `name`), 5 « nom » vers une
    déclaration de module de `lib.rs` (`dataflow`, `daemon_lifecycle`), 1 « type ».
- **« nom absent de la ligne » 3 → 300 à 14 h 51** : les réexportations, pas le lot 6 (voir
  plus bas).
- Les receveurs (variable, chaîne, champ) ne bougent pas : les retours d'appel et les paramètres
  de fermeture sont le poste suivant de B2d.

## Les trois défauts trouvés, et leur correctif

1. **Un mot de chaîne ou de commentaire devenait une référence** (codeparsers `1f5506f`, branche
   `chaines-hors-references`). Les scopes de fichier et de conteneur lisent leurs références sur
   le texte : les mots de `FINAL_NUDGE` (« calls », « is », « with ») reliaient le scope de
   fichier d'`agent.rs` aux homonymes. Le correctif masque par l'arbre chaînes et commentaires
   (les interpolations restent lisibles). Sur rag3weaver : **6 728 références retirées, 0
   ajoutée ; 6 720 dans une chaîne ou un commentaire** (vérifié par un lexeur qui suit les
   chaînes sur plusieurs lignes), 8 où le même mot est aussi dans le code (l'occurrence du code
   reste). Le recul du banc (`refs.other.relies_pct` 25,1 → 23,1) est de la précision : 1 496
   liens par le nom de moins.
2. **Une réexportation partait du premier scope de fichier, sans ligne, marquée « nom »** (mon
   lot des réexportations). Les 300 « nom absent de la ligne » : `lib.rs` → `RefError`,
   `GenOptions`… Elle part désormais du scope de son `pub use`, à sa ligne, sur l'arête que
   l'import pose déjà. Sur master : `5d0223d6e`, `39734d772` (relu par l'arbre principal).
3. **Le nom du scope cachait l'appel récursif et la méthode homonyme d'un autre receveur**
   (codeparsers `af7bc52`, branche `nom-propre-par-position`) — trouvé par le cœur sur
   `acquireLocksBeforeExecution`, et présent ici aussi (`self.inner.call_in` dans
   `ToolOutputLimit::call_in`). Sur rag3weaver, référence par référence : 482 ajoutées, 73
   retirées (le type d'un bloc `impl` vers lui-même, règle T1).

Les correctifs 1 et 3 attendent la reprise de `nom-seul` par l'arbre principal (ils reposent sur
`a6111cc`) ; leur effet sur les marques au rendez-vous se remesure alors, sur ce même corpus.
