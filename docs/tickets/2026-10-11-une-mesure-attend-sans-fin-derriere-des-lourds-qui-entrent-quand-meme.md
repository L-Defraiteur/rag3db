# Une mesure attend sans fin derrière des lourds qui « entrent quand même »

- **État** : ouvert (11 octobre 2026, optimiseur, à la demande de l'orchestration)
- **Gravité** : blocage. La mesure des fils du cœur a expiré à 21 h 45 sans avoir couru : une
  heure perdue, et codeparsers et hotfixs attendaient derrière.
- **Atteignable en service** : oui, sur le poste, dès que des travaux lourds se suivent pendant
  qu'une mesure attend
- **Touche rag3weaver** : oui (`~/.cache/rag3weaver-build/poste`, le verrou du poste ; le script
  entre dans le dépôt sous `extension/rag3weaver/scripts/poste`)

## Ce que c'est

`poste mesure` prend la porte, puis attend la place exclusive (`poste.lock`). Un `poste lourd`
attend la porte au plus `POSTE_PORTE_MAX` secondes (600), puis **entre quand même** en partagé.
Sous une charge continue, il y a toujours un lourd dans la place : la mesure ne passe jamais.
Son délai, posé par l'appelant (`timeout … poste mesure …`), se consomme à attendre.

## La recette

Un lourd tient la place ; une mesure arrive ; d'autres lourds arrivent derrière elle, avec
`POSTE_PORTE_MAX=1`. Chaque nouveau lourd entre au bout d'une seconde, la place partagée ne se
vide jamais, et la mesure ne court qu'une fois tous les lourds finis.

## Le témoin

`extension/rag3weaver/scripts/test_poste.sh`, sur des verrous d'essai (`POSTE_LOCK_DIR`) :

1. Un lourd tient la place, une mesure arrive, deux lourds arrivent derrière elle. La mesure
   court dès la fin du premier lourd, et avant les deux suivants.
2. `POSTE_MESURE_MAX` : le délai d'une mesure ne court qu'une fois la place prise. Une mesure
   qui attend plus que son délai court quand même, et une mesure qui dépasse son délai une fois
   dans la place est arrêtée (124).
3. `poste --qui` dit qui tient et qui attend, en mesure ou en lourd, et depuis quand.

## Ce qu'il faut pour le fermer

La forme d'un ordonnanceur qui donne la priorité au travail exclusif :

- le délai d'une mesure ne court qu'une fois la place prise (`POSTE_MESURE_MAX`) ;
- dès qu'une mesure attend, la place est refusée aux nouveaux lourds, et le « entre quand même »
  ne vaut plus ;
- `poste --qui` dit qui attend quoi, depuis quand ;
- le témoin vert, le script déployé sur les deux postes, la règle dans la page `poste push`.
