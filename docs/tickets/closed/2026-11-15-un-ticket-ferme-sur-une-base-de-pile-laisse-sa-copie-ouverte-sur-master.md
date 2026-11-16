# Un ticket fermé sur une base de pile laisse sa copie ouverte sur master

- **État** : fait (15 novembre 2026, master 2fb7fdcec) : la garde retire la copie ouverte d'un ticket que le lot ferme, au dernier pas ; fermé à la passation de G le 16 novembre
- **Famille** : produit
- **Gravité** : CI rouge ; master porte deux copies d'un même ticket jusqu'au commit de docs qui retire l'ouverte, poussé à la main.
- **Atteignable en service** : non, c'est l'outillage du push.
- **Touche rag3weaver** : non ; `tools/poste/push_garde.sh`.
- **Pour** : optimiseur (G), lot derrière igpu:0.
- **Ouvert le** : 15 novembre 2026

## Ce qui se passe

Un lot ferme un ticket par `git mv docs/<session>/tickets/<t>.md docs/tickets/closed/<t>.md`. Sa base de pile est
antérieure au ticket ouvert. Rejoué par la garde sur master, son commit ne fait donc que créer la copie fermée, et
l'ouverte reste. `check_tickets` voit les deux et rougit la CI. Chaque session pousse ensuite à la main un commit de
docs qui retire l'ouverte.

## La correction attendue

Au dernier pas, après le rebase et la vérification de la pile, la garde relève les fichiers créés sous
`docs/tickets/closed/` par les commits du lot (`master..HEAD`). Pour chacun dont un homonyme existe encore ailleurs sous
`docs/*/tickets/` dans HEAD, elle ajoute dans le même push un commit de docs seules qui retire cet homonyme, et le dit
au journal. Rouge d'abord dans `test_push_garde.sh`, sur un dépôt jouet : sans la correction, master garde les deux
copies ; avec, il n'a que la fermée.
