# Un ajout de dépôt écrit avant de savoir s'il pourra ouvrir, et ne se retire plus

- **État** : ouvert le 21 novembre 2026 par la session tty, dans les essais de la nuit (scénario 1). G confirme : ce sont deux défauts du produit, indépendants du bâti de l'hôte.
- **Famille** : produit
- **Gravité** : un dépôt ajouté qu'on ne peut pas ouvrir reste inscrit à l'hôte, avec sa configuration et sa base, et le produit ne sait pas le retirer. Un dépôt retiré avec une tâche ouverte laisse la tâche orpheline.
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

## Un retrait qui laisse une tâche orpheline (21 novembre, 06 h 2x)

Sur l'hôte du matin (78f2f2d885), `code-tree remove code-projet-tty` ferme le dépôt, et met de côté son manifeste et sa base, alors que sa tâche `essai-scenario-1` est encore ouverte. Ensuite, `POST /ide/taches/rejeter {backend, nom, why}` échoue : « git worktree remove --force …/taches/76ad218634aa/essai-scenario-1/arbre : fatal : cannot change to '…/.config/rag3weaver/code-projet-tty' ». L'hôte liste encore le backend 76ad218634aa, et le dossier de la tâche reste.

**Ce qu'il faut de plus** : `code-tree remove` refuse un dépôt dont une tâche est ouverte, en la nommant, ou bien il la ferme d'abord ; et un dépôt retiré quitte l'état de l'hôte.
Et le rejet d'une tâche ne dépend pas du dépôt encore présent (G, le 21 novembre). G retire les restes, hôte arrêté, à la pose du second hôte : la tâche du registre, son dossier et les fichiers restants, mis de côté sous `instantanes/menage-tty-20261121/` (jetable à 72 h).
