# Un même texte ne rend pas les mêmes bits selon son lot

- **État** : ouvert (16 novembre 2026, embarquement ; observation, demande de c7)
- **Pour** : embarquement
- **Famille** : produit
- **Gravité** : observation. 1e-5 sur un cosinus ne change pas une réponse ; rien d'aujourd'hui ne dépend
  de l'égalité des bits d'un vecteur.
- **Atteignable en service** : oui (tout embarquement sur GPU par burn/cubecl)
- **Touche rag3weaver** : oui, s'il faut un jour le corriger (le découpage en lots du service, les
  embarqueurs burn)

## Ce que c'est

Sur le 7878 (granite-278m, card2), un même texte rend des bits identiques dans une même forme de lot, et des
bits différents (écart absolu max 3,77e-5, cosinus 0,99999995) quand il passe seul puis au milieu de 7
textes plus longs. Le banc l'a vu à grande échelle le 16 novembre : 195 morceaux sur 412 différents d'au
plus 5,86e-5 entre deux index de la même source. Cause attendue : le lot se remplit jusqu'à la longueur du
plus long texte, et l'ordre des réductions flottantes, voire le noyau choisi par l'autotune de cubecl,
dépendent de la forme du lot.

## Ce qu'il faudrait

Rien tant que rien n'exige l'égalité des bits (un dédoublonnage ou un cache indexé par le vecteur, une
comparaison d'index octet par octet). Si un tel besoin apparaît : voir le ticket
`2026-11-16-un-reglage-deterministe-de-cubecl-a-chercher.md`, ou embarquer chaque texte dans un lot de forme
fixe (coûteux). La parité se juge par tolérance : ticket
`2026-11-16-la-preuve-de-parite-des-services-n-a-pas-de-tolerance-mesuree.md`.

Fermer ce ticket quand la tolérance est mesurée et écrite, en le disant : observation tenue, pas de
correctif.
