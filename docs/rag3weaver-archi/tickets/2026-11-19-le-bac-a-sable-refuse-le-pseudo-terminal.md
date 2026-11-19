# Le bac à sable refuse le pseudo-terminal

- **État** : ouvert, le 19 novembre 2026 ; pour archi
- **Famille** : produit
- **Pour** : archi
- **Gravité** : un agent qui travaille sur le crate du terminal ne peut jamais voir sa suite verte : trois tests rougissent dans le bac à sable, quel que soit son correctif. Il peut prendre ces rouges pour les siens.
- **Atteignable en service** : oui, toute commande d'un agent passe par le bac à sable (`workspace.sandbox`).
- **Touche rag3weaver** : oui, `src/commande.rs` (règles Landlock du bac à sable)
- **Signalé par** : archi, le 19 novembre 2026, au rejeu du cas de campagne tty 09

## Ce qui a été vu

Le rejeu du cas tty 09 (`~/.cache/rag3weaver-build/campagne/09-…/20261119-162624`, tablette) a lancé `cargo test` dans le crate `tty`, par la porte de l'hôte de campagne : `106 passed; 5 failed`. Deux de ces échecs sont les témoins du cas, les trois autres viennent de `src/coquille.rs`, qui panique sur « le pseudo-terminal ne s'ouvre pas : Permission denied (os error 13) » :

- `command_runs_in_its_folder_shows_and_keeps_its_output_and_its_code` ;
- `a_password_read_with_echo_off_never_reaches_the_kept_output` ;
- un troisième test de `coquille`.

Hors du bac à sable, les mêmes tests sont verts.

## Ce qu'il faut faire

1. Vérifier que c'est bien `/dev/ptmx` (et `/dev/pts/*`) que les règles Landlock refusent en lecture-écriture.
2. Les accorder dans les règles du bac à sable : un pseudo-terminal ne donne accès à rien hors du domaine.
3. Sinon, le dire à l'agent dans la sortie : un refus du bac à sable doit se lire comme tel, pas comme un rouge du code.

## Comment le prouver

Un témoin rouge d'abord : une commande qui ouvre un pseudo-terminal sous `sandbox_exec` (par exemple `script -qc true /dev/null`) échoue. Puis vert, et la suite `tty` jouée par la porte d'un hôte n'a plus que ses vrais rouges.

## Taille et blocage

Une demi-journée. Il ne bloque pas le lanceur : le banc sait que ces trois tests sont rouges dans le bac à sable.
