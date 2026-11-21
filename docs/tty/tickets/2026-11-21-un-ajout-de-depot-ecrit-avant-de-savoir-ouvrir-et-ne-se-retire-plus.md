# Un ajout de dépôt écrit avant de savoir s'il pourra ouvrir, et ne se retire plus

- **État** : ouvert le 21 novembre 2026 par la session tty, dans les essais de la nuit (scénario 1). G confirme : ce sont deux défauts du produit, indépendants du bâti de l'hôte.
- **Famille** : produit
- **Gravité** : un dépôt ajouté qu'on ne peut pas ouvrir reste inscrit à l'hôte, avec sa configuration et sa base, et le produit ne sait pas le retirer.
- **Atteignable en service** : oui, chaque fois que l'hôte ne peut pas ouvrir le backend d'un ajout (une feature absente du bâti, un gabarit plus neuf que le binaire, un modèle introuvable).
- **Touche rag3weaver** : oui (`code-tree add`, `code-tree remove`).

## Ce que c'est

Le 21 novembre à 01 h 30, l'hôte de service de la tablette avait été bâti sans `token-count`, et ne savait pas ouvrir le gabarit de code de sa propre tête (qui découpe pour granite-278m).

1. **`rag3weaver-backend code-tree add <copie> --from templates/backends/code/backend.json`** répond « l'hôte http://127.0.0.1:7970 n'ouvre pas ce backend : … il faut la feature token-count ». Il a pourtant déjà écrit :
   - `~/.config/rag3weaver/code-projet-tty/` ;
   - la base `~/.local/share/rag3weaver/code-projet-tty.rag3db`, avec `.extensions` et `.readers` ;
   - le backend 76ad218634aa, inscrit à l'hôte.
2. **`rag3weaver-backend code-tree remove code-projet-tty`** répond la même chose : pour retirer, il veut ouvrir.

## Le pas qui bloque

Le retrait. Le dépôt reste inscrit, et `--sans-hote` laisserait l'inscription dans l'hôte en mémoire.

## Contournement

Attendre un hôte qui sait ouvrir le backend, puis `code-tree remove` (G, le 21 novembre).

## Ce qu'il faut

- **L'ajout ne laisse rien derrière lui quand l'hôte refuse d'ouvrir.** Soit il vérifie d'abord que l'hôte saura ouvrir ce manifeste, soit il défait ce qu'il a écrit quand l'ouverture échoue.
- **Le retrait ne demande pas d'ouvrir le backend.** Il désinscrit, et retire la configuration et la base, en le disant.
