# Un chemin de la bibliothèque standard se relie à un homonyme du projet

- **État** : corrigé (18 novembre 2026, branche `codeparsers/rust-stop-words-4`) : la racine du chemin se lit à la position exacte de la mention (`reference_path_root`, repris de Codex) ; une racine `builtin` ou de provenance sûre ne prend pas de rendez-vous par le nom
- **Famille** : produit
- **Gravité** : réponse fausse (une arête « nom » vers une fonction homonyme du projet)
- **Atteignable en service** : oui, dès que le lot des mots vides de Rust atterrit (branche rag3db
  `codeparsers/rust-stop-words`) ; masqué jusque-là parce que `var` était un mot vide (la liste de JavaScript)
- **Touche rag3weaver** : oui (`src/code.rs`, la collecte du rendez-vous)
- **Pour** : la session qui reprend codeparsers (Codex)
- **Signalé par** : session codeparsers

## Ce que c'est

`std::env::var("X")` : la référence `var`, qualifiée par le chemin `std::env`, prend rendez-vous ; le seul
définisseur `var` du projet (`src/ide/clic_tsx.rs`) conclut par le nom. Sur le corpus figé de rag3db-garde,
**107** arêtes « nom » de ce genre naissent avec le lot des mots vides de Rust (`var` n'y est plus un mot vide),
plus 9 `var` nus. La règle de la marque « nom » (méthodes std) ne couvre que les receveurs locaux, et celle des
bibliothèques sûres (`via_qualifier`) que les qualificatifs importés : un chemin écrit en entier
(`std::env::var`), sans `use`, n'entre dans aucune.

## Ce qu'il faut

Dans `materialiser`/la collecte de `code.rs` (là où `externes_sures` écarte déjà le membre d'une bibliothèque
de provenance sûre) : une référence dont le qualificatif est un chemin (`chemin` vrai) dont **le premier
segment** est une bibliothèque de provenance `builtin` ou `dependency` (`std`, `core`, `alloc`, une crate
déclarée) ne prend pas de rendez-vous. Le premier segment se lit sur le qualificatif (`std::env` → `std`).

## Pour le fermer

Un témoin rouge dans `tests/e2e_usages.rs` (`std::env::var("X")` dans un fichier, un `pub fn var()` dans un
autre : aucune arête), la sonde avant/après (`tests/sonde_marque_nom.rs`) : les 107 partent, rien de juste ne
part.

## Reprise préparée, sans calcul pendant la bascule du 16 novembre

WIP conservé depuis 5ef2935d29/b48da05 dans un worktree neuf
`/home/lucied/git_workspaces/cx-codeparsers-rust-stop-words`, branches
rag3db `cx-codeparsers-rust-stop-words` et codeparsers `cx-rust-stop-words`.
Le contexte de ligne est partagé et trimé (`line_at_byte`), tandis que la colonne
AST est un offset UTF-8 depuis le début de ligne non trimé : ne pas appliquer
cette colonne directement au contexte. Le filtrage doit porter sur la mention
exacte, pas toutes les occurrences homonymes de sa ligne.

Deux témoins supplémentaires publiés avant correctif : appel std puis appel
projet nu sur la même ligne, puis ordre inverse avec préfixe UTF-8. **NON JOUÉS**
(pas de Cargo/rustc/test pendant la fenêtre de bascule). Le rouge est à observer
sur la forme 5ef avant de remplacer ce filtrage ; aucun vert n'est revendiqué.
Les rouges historiques et les 14 pertes de la première sonde restent conservés.

## Fermeture, 18 novembre 2026

Le correctif posé lit la racine du chemin **à la colonne de la mention** dans la ligne brute
(`reference_path_root`, de Codex, branche `cx-codeparsers-rust-fix-1`), et non plus dans toute la ligne :
`std::fs::remove_file(…)` ne se relie plus à un `remove_file` du projet, et un appel nu du projet écrit sur
la même ligne qu'un chemin de std reste relié. Les deux rouges de Codex (deux ordres d'appel sur une ligne,
0 passé sur 2, « the project call must remain visible ») sont devenus les témoins
`a_standard_library_path_does_not_hide_a_bare_project_call_on_the_same_line` et
`a_bare_project_call_before_a_library_path_on_the_same_line_remains_visible` ; le premier rouge du ticket
est `a_path_into_the_standard_library_is_not_a_project_homonym`.

Sonde de marque sur l'arbre figé (`rag3db-garde`, 290 fichiers de rag3weaver/src, 1 550 du moteur), avant =
master (codeparsers bd13385), après = la branche (34c74e4) :

| | avant | après |
|---|---|---|
| rag3weaver, `nom` | 8 701 | 8 453 |
| rag3weaver, `type` | 9 691 | 9 884 |
| rag3weaver, `fichier` | 18 970 | 19 094 |
| moteur | — | identique au couple près |

258 arêtes retirées, toutes un chemin de std vers un homonyme du projet (`std::process::id` 78,
`std::fs::remove_file` 49, `std::fs::metadata` 37, `std::sync::mpsc::channel` 37, `std::fs::copy` 22,
`std::sync::atomic` 19, `std::env::args` 6, `std::path::Component`, `hash_map::Entry`, `std::ptr::eq`,
`std::task::Context`). Aucune vraie arête perdue à la lecture des 258.

