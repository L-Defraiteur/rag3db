# Un cas known_red qui passe une fois par effet d'ordre rougit le juge

- **État** : fait (18 novembre 2026, branche known-red-intermittent) : un connu rouge qui passe une fois devient une ligne STATUS « known red passed this time (intermittent?) », comptée dans « bench matches … N intermittent », sans faire échouer le juge ; test/transaction/concurrence/confirm_known_red_fixed.sh <banc> <cas> [N=3] joue le cas seul N fois, chaque fois dans un processus neuf, et ne demande le retrait de la ligne qu'à N/N verts ; trois cas neufs au témoin, rouges d'abord
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

## Ce que le juge ne peut pas faire seul (G)

Le juge ne garde aucun état d'un tour à l'autre, et le banc complet ne dit rien d'un cas seul : il ne peut donc pas
compter « N passes vertes consécutives seules ». Il faut un rejeu à part du cas seul, N fois
(`--gtest_filter=<nom> --gtest_repeat=N`, ou N processus neufs si l'état du processus compte). Le juge, ou un script
à côté, ne demande « corrigé : retirer la ligne » que si les N passes sont vertes. N est réglable, 3 par défaut. Le
même défaut était aussi ouvert chez G (599239704) : ce ticket-ci les réunit, déplacé chez G qui porte le juge.

## Pour fermer

La règle dans le juge et dans son témoin (`test_compare_known_red.sh`) : un faux banc où un cas
connu rouge passe une fois rend vert, avec la ligne « intermittent » ; un cas vert N fois seul rend la demande de retrait ; un connu rouge resté rouge reste muet.
