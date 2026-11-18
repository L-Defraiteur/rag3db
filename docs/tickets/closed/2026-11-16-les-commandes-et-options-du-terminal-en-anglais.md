# Les commandes et options du terminal en anglais, avec alias

- **État** : fait. Lot tty-21 (18 nov. 2026), repris du travail de Codex sur `cx-tty-english-aliases-7` (16 et 17 nov.) : les commandes et options anglaises à côté des françaises, et la complétion de `/task` et `/repo` ; page `docs/tty/18-11-2026/02-tty-21-commandes-et-options-en-anglais.md`.
- **Famille** : produit
- **Gravité** : forme. Les commandes et options françaises restent, mais toute nouveauté est en anglais, et l'existant prend un alias quand on y touche.
- **Atteignable en service** : oui, à chaque lancement du terminal et à chaque `/commande`.
- **Touche rag3weaver** : oui (le crate `extension/rag3weaver/tty`).

## Ce qu'il faut faire

Ajouter un nom anglais à chaque `/commande` et option française, en gardant l'ancien nom, qui marche toujours et que `/aide` dit « ancien nom ».

| Français (gardé) | Anglais (ajouté) |
|---|---|
| `/aide` | `/help` |
| `/quitter` | `/quit` |
| `/arreter` | `/stop` |
| `/theme` | `/theme` (déjà) |
| `/tache` | `/task` |
| `/depot` | `/repo` |
| `/journal` | `/log` |
| `/effacer`, `/effacer historique` | `/clear`, `/clear history` |
| `/context` | `/context` (déjà) |
| `/etapes` | `/steps` |
| `/model` | `/model` (déjà) |
| `/nom` | `/name` |
| `--depot` | `--repo` |
| `--tache` | `--task` |
| `--demande` | `--request` |
| `--journal` | `--log` |
| `--etapes` | `--steps` |
| `--depuis` | `--since` |
| `--adresse` | `--address` |

- `src/commandes.rs` : `lire` accepte les deux noms ; `COMMANDES` montre le nom anglais en premier dans `/aide`, avec « (ancien : /nom) » ; la complétion par Tab propose les noms anglais.
- `src/main.rs` : les options anglaises, et l'aide (`AIDE`) avec les noms anglais ; les anciens sont lus sans avertissement pour l'instant (pas de bruit pour Lucie), et le README dit qu'ils sont anciens.
- `extension/rag3weaver/tty/README.md` : les noms anglais d'abord.
- Ne rien retirer : les scripts de Lucie et des sessions lancent `--depot`, `--tache`, `--demande`, `--journal` (voir `docs/orchestration/16-11-2026/01-essais-empiriques-algareth.md`).
- Le texte affiché reste en français.

## La preuve

- Test de `commandes.rs`, rouge d'abord : chaque nom anglais rend la même `Commande` que le français ; Tab sur `/st` propose `/steps` et `/stop`.
- Test de lecture des options de `main.rs` : extraire la lecture des arguments dans une fonction testable si elle ne l'est pas (`lire_options`), puis un test qui lit les deux jeux de noms.
- Passe courte : crate, `check_features`, harnais entier (les séances lancent le terminal avec les anciens noms : elles doivent rester vertes).

## Taille et blocage

3 heures. Rien ne le bloque.

## Ce qui a été fait (tty-21)

- Les `/commandes` du tableau ont leur nom anglais. L'ancien marche toujours, et `/aide` (`/help`) montre l'anglais d'abord, avec « (ancien : …) ».
- Les options aussi. Les anciennes sont lues en silence, `--backend` compris.
- La lecture des options est une fonction testable, `parse_options`.
- La complétion par Tab de l'argument de `/task` et `/repo` (et de `/tache`, `/depot`) choisit ses sources par la commande lue, et non plus par un préfixe : avant, `/task mes` ne complétait rien.
- Différences avec ce que ce ticket demandait : `/clear` remplace `/effacer` (avec `/clear history`) ; `--since` et `--address` s'ajoutent à la liste ; les anciens noms d'options ne sont pas dits dans le README, seulement dans l'aide.
- Les tests de Codex, vus rouges sur sa branche, puis rouges à nouveau sur master pour les commandes.
