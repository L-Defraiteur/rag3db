# La sonde de débit d'embarquement rougit dans la lib entière

- **État** : corrigé (8 novembre 2026) par rag3weaver archi, sur demande de c7 : la sonde reçoit son horloge (`probe_rate_avec`), et le témoin avance une horloge partagée au lieu de dormir. Le débit est exact (100 000 puis 50 000 car./s), quel que soit l'ordonnanceur. Le rouge n'a pas pu être forcé à la demande : 48 copies parallèles, quatre boucles occupées sur un cœur, `SCHED_IDLE`, deux libs entières, tout passait. Il est attesté par les deux passes de 8 h 46 et 9 h 43.
- **Famille** : produit
- **Gravité** : faux rouge (témoin de durée), qui arrête une passe.
- **Atteignable en service** : non, c'est le témoin seul qui est en cause.
- **Touche rag3weaver** : oui, `src/estimate.rs`, le témoin `estimate::tests::la_sonde_mesure_l_embarqueur_attache`.

## Ce qui s'est passé

Le témoin mesure le débit d'un embarqueur factice, qui dort 10 µs par caractère. Il exige de 40 000
à 110 000 caractères par seconde. Dans la lib entière, sous le jeu large et épinglée sur la lib
dadbf0041, sur le portable :

| heure | débit lu | charge du poste sur 1 min, 5 min |
|---|---|---|
| 8 nov., 8 h 46 | 33 240 car./s | 4,4 / 3,7 (relance des hôtes à 8 h 45) |
| 8 nov., 9 h 43 | 29 772 car./s | 2,4 / 1,1 |

Rejoué seul, il passe : trois fois sur trois à 8 h 47, et trois fois sur trois à 9 h 43 sous
`poste lourd`, en 0,05 s. Une troisième passe complète, à 8 h 47, l'a vu VERT.

## Ce que c'est, sans preuve

Ce n'est pas la charge du poste, basse à 9 h 43. C'est la lib entière, qui fait tourner ses tests en
parallèle : les `sleep` du factice s'allongent, et le débit mesuré tombe sous la borne basse. La
borne de 40 000 laisse 2,5 fois de marge sous les 100 000 théoriques. Un `sleep` de 10 µs coûte
plus cher qu'il ne dit, et la mesure porte sur 4 000 caractères seulement.

## Ce qu'il faut

Un témoin qui ne dépend pas de l'ordonnanceur. Par exemple :
- un embarqueur factice qui avance une horloge injectée au lieu de dormir ;
- ou une borne relative, qui vérifie seulement que la sonde rend un débit, et que deux factices
  plus ou moins lents se rangent dans le bon ordre.
D'ici là, la règle du jour s'applique : rejoué seul une fois, nommé, pas de `known_red`.
