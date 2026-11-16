
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
