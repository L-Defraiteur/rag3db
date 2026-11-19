# Cas de campagne : une demande s'écrit une fois, et /nom retient le nom gardé

Session tty, 19 novembre 2026, pour la campagne des correctifs passés (chantier de Lucie ; lanceur d'archi, juge du banc). C'est un cas court, en Rust, dans le crate du terminal, avec deux témoins.

## Le point de départ

- **Commit parent** : `b1163f64e1` (16 novembre 2026, 0 h 10). Les deux témoins y sont, rouges ; le correctif n'y est pas.
- **Arbre** : `extension/rag3weaver/tty` suffit (crate autonome, sans la lib rag3db).
- **À savoir** : au parent, `tui::nom_garde` existe déjà comme souche (`-> None`), posée avec le témoin pour qu'il compile. Le cas le laisse tel quel : la souche est un indice, et Algareth la voit.

## Le signalement, tel qu'il était

> Essai réel de `/nom` dans le terminal de l'agent, 16 novembre. Deux défauts :
> 1. **Une demande s'écrit deux fois.** Ce que l'on tape et envoie s'écrit tout de suite à l'écran (« › … »). Puis la boîte de l'équipier rend la même ligne, et elle s'écrit une seconde fois. Une ligne du superviseur venue d'ailleurs (l'IDE), qui n'a pas été tapée dans ce terminal, doit, elle, s'écrire.
> 2. **`/nom` ne retient pas le nom gardé par l'hôte.** Après `/nom Ombre`, l'hôte garde le nom (`POST /ide/teammate/name` rend `agent_name`), mais la vue ne le retient pas : `/nom` seul ne le redit pas, et les lignes de l'agent ne le portent pas. Un nom vide doit prendre celui que l'hôte rend (le nom du dépôt). Un refus de l'hôte ne doit en garder aucun.

## Les témoins

- **Filtres** :
  - `tui::tests::the_name_kept_by_the_host_is_the_one_the_view_keeps`, dans `extension/rag3weaver/tty/src/tui.rs` ;
  - `vue::tests::what_you_send_is_written_once_even_when_the_mailbox_gives_it_back`, dans `extension/rag3weaver/tty/src/vue.rs`.
- **Suite** : `cargo test` du crate `extension/rag3weaver/tty`, avec la vue (défaut). Les deux témoins sont identiques sur master (vérifié le 19 nov. sur 63e24ff7a3).
- **Rouge au parent** :
  - `nom_garde` rend `None` là où l'hôte a gardé « Ombre » ;
  - la ligne rendue par la boîte s'écrit une seconde fois.
- **Suites exigées par les chemins** : `tty` (`tools/poste/required_suites.txt`, depuis `cx-passe-tty`). C'est le crate avec et sans vue, clippy `-D warnings`, et les deux mesures `plain_*`.

## Notre correctif (caché à Algareth, pour le juge)

`3ad8950c4f`, « fix(tty): une demande s'écrit une fois (la copie que rend la boîte ne se réécrit pas), et /nom retient le nom que l'hôte garde » : `extension/rag3weaver/tty/src/tui.rs` (+7 −2), `extension/rag3weaver/tty/src/vue.rs` (+29 −1), et une page de docs (`docs/tty/15-11-2026/10-nom.md`, +16, hors du cas). Au total +52 −3.
