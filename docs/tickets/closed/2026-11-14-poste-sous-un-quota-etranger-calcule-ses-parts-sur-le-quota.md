# Un poste lancé sous un quota étranger calcule ses parts sur le quota

- **État** : corrigé par le lot garde-pile (14 novembre 2026, `fils_par` lit `nproc --all`, cas 9 de `test_poste.sh`).
- **Famille** : produit
- **Gravité** : coût ; un lourd lancé dans une portée à quota reçoit une part réduite (1 cœur au lieu de 5 à luciepc), et le témoin de l'enveloppe se saute.
- **Atteignable en service** : oui, un `poste lourd` lancé dans une portée `systemd-run` à `CPUQuota`, hors d'un ancêtre `poste`.
- **Touche rag3weaver** : oui, `extension/rag3weaver/scripts/poste` (`fils_par`) et `scripts/test_poste.sh` (cas 8 et 9).
- **Pour** : optimiseur (G).
- **Ouvert le** : 14 novembre 2026

## Ce que c'est

Sur ce poste, `nproc` lit le quota du cgroup (`cpu.max`) : dans un lourd de 500 % à luciepc, il rend **5** et non 24.
`fils_par` (3a654c4d9) partait de `nproc` : un `poste` lancé sous un quota qui n'est pas celui d'un ancêtre `poste`
calculait ses parts sur ce quota. Le cas 8 de `test_poste.sh`, joué dans le rejeu de tenant-signal-pile sous un
lourd, s'est **sauté** (« moins de 6 cœurs ici »), puis il a été rejoué hors lourd et il était vert.

## La recette

`systemd-run --user --scope -p CPUQuota=200% -- poste lourd sh -c 'echo $POSTE_FILS'` rend 1 au lieu de
(cœurs − 2) / lourds-max.

## Le témoin

`scripts/test_poste.sh`, cas 9 (« sous un quota étranger de 200 %, un lourd reçoit N fils ») : rouge sur cd103b619
(lu : 1), vert avec `nproc --all` (lu : 7 au portable). Le cas 8 lit lui aussi `nproc --all`.

## Pour le fermer

Le lot garde-pile sur master, puis `poste` recopié aux deux postes.
