# La lecture des déclarants coûte autant que le choix des cibles, pour un lot C/C++

- **État** : ouvert — mesuré sous charge, non profilé ; à profiler à une fenêtre calme de luciepc.
- **Gravité** : lenteur (le verrou de la montre tenu plus longtemps pour un fichier C/C++ touché).
- **Atteignable en service** : oui, dès qu'INCLUDES est sur master (chaque réindexation d'un lot C/C++).
- **Touche rag3weaver** : oui (`src/code.rs`, `Declarants::charger_par`).

## Ce que c'est

La forme exacte d'INCLUDES lit, une fois par lot qui a du C/C++, les déclarants des définisseurs du
lot : les conteneurs (classes, namespaces) par un `In` sur leur nom, leur genre et les en-têtes des
fermetures, puis relit leur JSON `declarations`. À la montre, « symboles · déclarants » pèse autant
que « symboles · choisir et lier ».

## Ce qui est mesuré (25-26 octobre 2026, luciepc, **sous charge**)

Le témoin de la montre (rag3db entier, deux `.rs` renommés, deux `.cpp` touchés), deux binaires du
même lot, sans puis avec la borne par fichier :

| | lignes lues | « symboles · déclarants » | « choisir et lier » | verrou tenu |
|---|---|---|---|---|
| sans la borne | 4 075 | 230 ms | 133 ms | 2 771 ms |
| avec la borne | 2 233 | 264 ms | 147 ms | 2 834 ms |

La charge de luciepc était de 43 au premier témoin, de 15 au second (d'autres compilations et e2e) :
les temps absolus ne valent rien. Le rapport entre les phases d'un même verrou, lui, dit que la borne
ôte 45 % des lignes sans ôter de temps : le coût n'est pas dans le nombre de lignes. Les deux requêtes
portaient 923 noms de conteneurs et 1 184 en-têtes de fermeture (le moteur tire presque tout par ses
includes communs).

## Le témoin

Le témoin de la montre (`temoin.py`, `TEMOIN_CPP`) et sa ligne `[montre] déclarants : N requête(s),
M ligne(s) lue(s), …`.

## Ce qu'il faut pour le fermer

1. D'abord deux chronos séparés, la requête contre la relecture JSON, sur luciepc seul (une fenêtre
   calme donnée par c7 ; eu-stack ou le profil du moteur).
2. Puis réduire la cause mesurée. Une piste : ne lire que les conteneurs des définisseurs restés
   invisibles après inclusion et entete, comme on le fait déjà pour les fonctions libres — sans doute
   une poignée au lieu de 923.
