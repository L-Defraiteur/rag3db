# Un cas known_red qui passe une fois par effet d'ordre rougit le juge

- **État** : ouvert (15 novembre 2026, cœur c++, au mot de c7)
- **Famille** : moteur (le juge des tests du moteur)
- **Gravité** : gêne (un lot vert se lit rouge ; sans le mot de c7, il ne pousse pas)
- **Atteignable en service** : non
- **Touche rag3weaver** : non
- **Signalé par** : cœur c++. Le rejeu du lot de la colonne ajoutée (`68769674f`).
- **Pour** : G (optimiseur), qui porte le juge `test/transaction/concurrence/compare_known_red.cmake`

## Ce qu'on voit

`concurrence_test.known_red` sur `68769674f`, le 15 novembre à luciepc : le juge est rouge sur un
seul point, « known red turned green (remove it from known_red.txt in the commit of the step that
fixed it) : Cases/SameVectorForEveryRow.NeitherCrashesNorLosesRows/ZeroRowsFirstCopy » (13 rouges,
318 verts, aucun rouge neuf). Joué seul 10 fois sur la même tête, ce cas est rouge 10 fois sur 10,
toujours à 1 222/1 402 lignes atteintes. Le vert du banc complet tient donc à l'ordre des cas ou à
la graine du graphe, pas à un correctif. Le lot ne touche pas HNSW. Il a poussé au mot de c7, et
la ligne known_red reste.

## Ce qu'il faudrait (c7)

Un cas known_red qui passe une fois ne rougit pas le juge. Le juge le dit, comme il dit
« INCOMPLETE » : « connu rouge, passé cette fois (intermittent) ». Il n'exige le retrait de la
ligne qu'après N passes vertes consécutives du cas joué seul. N et la forme de la ligne sont à
écrire.

## Pour fermer

La règle dans le juge et dans son témoin (`test_compare_known_red.sh`) : un faux banc où un cas
connu rouge passe une fois rend vert, avec la ligne « intermittent ».
