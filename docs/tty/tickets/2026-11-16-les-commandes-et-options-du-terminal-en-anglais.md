# Les commandes et options du terminal en anglais, avec alias

- **État** : ouvert. Règle de Lucie du 16 nov. 2026 (« stop au code en français, ou aux /commandes en français ») ; rien n'est encore fait dans le terminal.
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

## Préparation ressources G — suivi C7 5b6ade163c

Contrat inactif publié dans
`docs/optimiseur/17-11-2026/01-tty-local-resource-contract.md` et JSON voisin
`tty-local-resources.inactive.json`. Mapping documentaire LOCAL7979/7980 vers
PC7878/7879 ; PGtest LOCAL5433 distinct du PGPC, identité/OID/topologie réelle
UNKNOWN. PorteurG transport/identité/allocation, Embar modèles, TTY pilote/Q,
C7 admission. Aucun ACK/GO3600/lease/ARM inventé, SQL/service/tunnel lancé.
Le contrat PGv1 PC ne devient pas LOCAL sans revue du domaine d'horloge et
transport qualifié. Préflight avant ARM reste HOLD si outil manquant ; le
pilote/Q peut se construire désarmé. Travail préparatoire, pas une clôture.
