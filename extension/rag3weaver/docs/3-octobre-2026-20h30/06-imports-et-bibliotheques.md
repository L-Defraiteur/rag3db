# Les imports et les bibliothèques

3 octobre 2026, codeparsers `a569417`. Trou de l'état des lieux (01) : Rust,
Go, C, C++ et C# passaient par l'extracteur d'imports de TypeScript (une
regex `import … from`) et n'en relevaient aucun ; Python marquait tout
import comme local, donc aucun `USES_LIBRARY`. C'est ce qui servira l'idée 4
de la vision produit code (plusieurs dépôts reliés : « qui chez nous
appelle cette fonction de la bibliothèque »).

## Ce qui a été décidé, et pourquoi

| Décision | Pourquoi |
|---|---|
| Rust : chaque `use` lu sur l'AST (listes, alias, globs, à toute profondeur), avec sa ligne ; la source est la racine du chemin (`std`, `serde`) ; `crate`, `self`, `super` sont locaux | La bibliothèque est la crate ; le reste du chemin (`sync::Arc`) reste le symbole de l'arête. |
| Seuls les imports **externes** changent le classement des références Rust | Mesuré : avec les imports locaux, `KBQuerySourceNode::named(…)` se résolvait vers le type importé et plus vers la méthode — 66 appels perdus. Les locaux restent relevés au fichier. |
| Python : seul l'import relatif est local | Le `true` en dur interdisait tout `USES_LIBRARY`. |
| Un import absolu dont la racine est un dossier ou un module du projet analysé n'est pas une bibliothèque | `from paquet.module import X` quand `paquet/` est dans le corpus. Limite : cela dépend du corpus — un en-tête du moteur inclus par `<processor/…>` passe pour une bibliothèque si `processor/` n'est pas analysé. |
| Un import externe ne se résout plus vers un homonyme du projet | `use std::fmt` reliait `fmt::Formatter` à la méthode `fmt` d'un `impl Display` d'un autre fichier. |
| C++ : `#include <…>` externe, `"…"` local, rattaché au scope qui contient sa ligne | Un include ne nomme aucun identifiant ; c'est la zone de fichier qui « utilise » l'en-tête. |
| Le repêchage des noms importés dans le texte d'un scope ignore commentaires et chaînes | Sans repêchage, Rust ne voit pas `Arc::new` (son visiteur écarte les noms de la bibliothèque standard) ; avec un repêchage brut, un `/// … Arc` en commentaire comptait. |

## Mesure

| Corpus | Avant | Après |
|---|---|---|
| `src/dataflow` : `USES_LIBRARY` | 0 | 698 (std 505, serde_json 136, serde 47, async_broadcast 8, jiff 2) |
| `src/dataflow` : `CONSUMES` | 7 672 | 7 669 — les trois retirées sont `use std::fmt` → une méthode `fmt` du projet ; aucune ajoutée |
| scripts Python de rag3weaver (17) | 0 | json 63, pathlib 37, subprocess 18, os 17, time, sys, threading… |
| C++ (24 fichiers de `src/main`, `test/c_api`) | 0 | thread, filesystem, fstream… |

Régénérable : `scripts/aretes_retirees.sh 594fc61 a569417 src/dataflow`.

## Reste

Go et C# (`import`, `using`) ; le chemin complet d'un import local pour
résoudre `crate::a::Foo` vers *ce* `Foo` plutôt que par nom.
