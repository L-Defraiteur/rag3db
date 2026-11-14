# Les sous-portées des passes ne passent pas par la file des lourds

- **État** : non reproduit (14 novembre 2026, 15 h 25) : les trois portées se suivaient, la file à une place a fait son travail.
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

## Non reproduit (14 novembre 2026, 15 h 25)

Relues **par heure** (première et dernière ligne de chaque portée du relevé), les trois portées du portable se
suivent : `deux-passes.sh` de 15 h 03 min 16 s à 15 h 04 min 16 s, `wt-1b-copie.sh` de 15 h 04 min 36 s à
15 h 07 min 36 s, `wt-verrou-passe.sh` de 15 h 07 min 56 s à 15 h 12 min 57 s. La conclusion d'origine venait
d'un résumé par portée sur toute la fenêtre, sans les heures. Dans le dépôt, seul `extension/rag3weaver/scripts/poste`
lance `systemd-run`, et `confiner_validation.py` n'en lance pas (il prend le poste ou passe par celui de l'ancêtre).

Pour refaire le relevé par heure :
```
awk -F'\t' 'NR>1 {k=$3; if(!(k in a)) a[k]=$1; b[k]=$1; c[k]=substr($7,1,60)} END{for(k in a) print a[k], b[k], k, c[k]}' \
  ~/.cache/optimiseur-notes/mesure-lourds-14nov.tsv | sort
```
Deux portées dont les intervalles se recouvrent, pour une seule place, rouvriraient ce ticket.
