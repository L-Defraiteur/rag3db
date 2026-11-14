# Les sous-portées des passes ne passent pas par la file des lourds

- **État** : ouvert (14 novembre 2026), relevé par G pendant la mesure de l'enveloppe CPU.
- **Famille** : produit
- **Gravité** : coût ; plusieurs portées lourdes tournent pour une seule place, hors du quota de la place.
- **Atteignable en service** : oui, au portable (`lourds-max` 1).
- **Touche rag3weaver** : oui, `extension/rag3weaver/scripts/poste` et `scripts/confiner_validation.py`.
- **Pour** : optimiseur (G), avec recherche et ED pour les confineurs.
- **Ouvert le** : 14 novembre 2026

## Ce que c'est

Au portable, le 14 novembre de 15 h 03 à 15 h 13, avec une seule place, **trois portées `run-*.scope`**
tournaient en même temps : `wt-verrou-passe.sh` (jusqu'à 2852 %), `deux-passes.sh` (800 %) et `wt-1b-copie.sh`
(1593 %, plafond mémoire de 32 Gio atteint). Une passe composée ou un confineur qui lance `systemd-run --scope`
crée une portée **sœur** et non fille. Elle ne compte pas dans la place et n'hérite pas du quota de son parent.

## La recette

Le relevé est `~/.cache/optimiseur-notes/mesure-lourds-14nov.tsv` au portable, produit par
`docs/optimiseur/scripts/mesure-lourds.sh`.

## Le témoin

Absent. Il faudrait un lourd factice qui lance lui-même une sous-portée, puis vérifier que les deux
partagent un seul quota et une seule place.

## La forme cible (c7)

L'imbrication sous la portée du parent (`--slice` ou `Delegate=`) : les sous-portées héritent du quota et
comptent dans la place, sans en prendre une de plus.

## Pour le fermer

Le témoin ci-dessus vert, et une mesure au portable qui montre une portée, ou des portées imbriquées, par place.
