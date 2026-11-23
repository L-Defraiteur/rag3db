# Après la suppression de toute une région sauf cinq lignes, un survivant peut être injoignable

- **État** : ouvert (23 novembre 2026) ; rangé dans `probabilistic.txt` avec sa fréquence mesurée
- **Famille** : moteur
- **Gravité** : réponse fausse, rare. Une ligne vivante n'est rendue par aucune recherche, et les plus proches d'une
  requête sont faux.
- **Atteignable en service** : à établir. Il faut supprimer presque toutes les lignes d'un index vectoriel (2 995 sur
  3 000, point d'entrée compris), en passant par `SET vec = NULL` avant le `DELETE`.
- **Touche rag3weaver** : à établir (le chemin « repasser par NULL » a été retiré de rag3weaver le 22 novembre)
- **Signalé par** : le juge known_red de la passe du lot `vector-index-special-names` (23 novembre, 03 h 39)
- **Pour** : le moteur, l'index vectoriel

## Le constat

`Cases/RegionDelete.SurvivorsStayReachableAndTheNearestAreRight/SetNullFirstAllButFive` (`concurrence_test`) :
`reached 4/5, nearest 1/5`. Le test n'avait jamais été relevé rouge, ni dans known_red ni dans probabilistic.

## La recette

Les données sont fixes : la graine mt19937 vaut 20261012, avec 3 000 vecteurs de dimension 16 autour de 30 centres,
en l2. L'index est créé avant les lignes, qui arrivent en 30 `UNWIND` de 100. Le test retire les 2 995 plus proches
du centre 0, triés par distance, en trois parts entrelacées (indices j ≡ 0, 1, 2 mod 3) : chaque part passe par
`SET n.vec = NULL`, puis par `DETACH DELETE`. Il cherche ensuite les 5 survivants à efs 5, et les 10 plus proches du
centre 0. Seuls le tirage des niveaux du graphe (pu) et l'ordre des fils varient d'un tour à l'autre.

## La fréquence, mesurée le 23 novembre (Release, sous poste cpu)

| moteur | rouges |
|---|---|
| master 03e649c42c + `vector-index-special-names` (avec le correctif des anciens voisins, 3196d6f66f) | 1 sur 121 (le juge, puis 0 sur 20 et 0 sur 100) |
| le même, le correctif des anciens voisins retiré (diff de cb3624f3ec en sens inverse) | 1 sur 100 |

La même signature des deux côtés. **Ce n'est pas une régression du correctif du 21 novembre** : le défaut est plus
ancien, et rare.

**Une réserve** (relecture du 23 novembre) : 1 sur 121 contre 1 sur 100 ne sépare pas les deux moteurs, mais ne prouve pas non
plus que le correctif du 21 novembre est sans effet sur ce cas. À cette fréquence, il faudrait des milliers de tours
pour départager. C'est le témoin déterministe (la graine du tirage des niveaux fixée sur un tour rouge) qui le dira.

## Ce qu'il faut chercher

Le passage par NULL fait de chaque ligne une mise à jour (retirée du graphe, réparée par `keepFormerNeighboursReachable`
avec ancre), puis une suppression. Avec presque toutes les lignes retirées, entrée comprise, les survivants dépendent de
la réparation du point d'entrée (`repairEntryPoint`) et des arêtes de rattrapage. Un témoin déterministe se cherchera
en fixant le tirage des niveaux (graine du `randomEngine` de l'index), pour retrouver le tour rouge à coup sûr.
