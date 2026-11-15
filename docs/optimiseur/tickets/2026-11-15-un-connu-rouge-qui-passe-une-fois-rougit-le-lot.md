# Un connu rouge qui passe une fois rougit le lot

- **État** : ouvert (15 novembre 2026) — à G, dans sa file après ses quatre lots
- **Famille** : moteur (le harnais du banc de concurrence : `test/transaction/concurrence/compare_known_red.cmake`)
- **Gravité** : faux rouge. Un lot juste est refusé par le juge parce qu'un cas intermittent a passé.
- **Atteignable en service** : non (le juge du banc de concurrence)
- **Touche rag3weaver** : non
- **Pour** : G (optimiseur)
- **Signalé par** : c7, 15 novembre 2026, vers 20 h 40, sur le lot du cœur `colonne-ajoutee-se-relit-2`

## Ce qu'on voit

Sur le lot du cœur, le cas `Cases/SameVectorForEveryRow.NeitherCrashesNorLosesRows/ZeroRowsFirstCopy`, dans
`known_red.txt` sous l'étiquette `same-vector-all-reachable`, a passé une fois dans le banc complet, par un effet
d'ordre ou de graine. Le juge a rendu « known red turned green (remove it from known_red.txt in the commit of the step
that fixed it) » et refusé le lot. Joué seul, le cas est rouge 10 fois sur 10.

## La cause

`compare_known_red.cmake` tient un connu rouge vert pour un correctif. Une seule passe verte suffit (ligne 142 sur
master). Le juge ne distingue pas un cas corrigé d'un cas intermittent. Un seul tour ne le peut pas : il faut
plusieurs passes du cas seul.

## Ce qu'il faudrait

1. Un connu rouge qui verdit dans un tour ne rougit pas le lot. Le juge le dit : « connu rouge, passé cette fois
   (intermittent ?) : <nom> », dans une ligne STATUS, et le compte avec les autres.
2. Le retrait de la ligne de `known_red.txt` n'est exigé qu'après N passes vertes consécutives du cas joué seul.
   N est réglable, 3 par défaut. Le juge ne garde pas d'état entre les tours, il faut donc un mode à part : le juge,
   ou un script à côté, rejoue le cas seul N fois (`--gtest_filter=<nom> --gtest_repeat=N`, dans un processus neuf par
   passe si l'état du processus compte) et dit « corrigé : retirer la ligne » seulement si les N passes sont vertes.
3. Témoin `test_compare_known_red.sh`, avec un faux banc gtest JSON comme pour les cas longs :
   - un connu rouge vert une fois rend un code 0 et la ligne « intermittent » (rouge sur master) ;
   - un connu rouge vert N fois seul rend la demande de retrait ;
   - un connu rouge rouge reste muet.
