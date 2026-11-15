# L'estimation d'une réindexation compte toute la source comme un premier index

- **État** : ouvert (15 novembre 2026)
- **Famille** : produit
- **Gravité** : réponse fausse et blocage. L'outil `index` refuse sans `confirm=true` une réindexation
  qui ne coûte presque rien, sur la foi d'une estimation d'un premier index (environ 10 min annoncées
  à la répétition de la bascule)
- **Atteignable en service** : oui. Toute réindexation d'une base déjà indexée dont la source passe le
  seuil de confirmation
- **Touche rag3weaver** : oui (`src/dataflow/index_nodes.rs`, `estimate_of` ; `src/estimate.rs`)
- **Pour** : everything declarative (ED)
- **Signalé par** : c7, répétition de la bascule par arbre principal (15 novembre 2026, 18 h)

## Ce que c'est

`estimate_of` estime le temps des vecteurs à partir des fichiers de la source, de leurs octets et du
rythme d'embarquement. Il ne lit ni la dette réelle de la base (`dense_missing`) ni ce qu'une
réindexation ajoutera. Une réindexation d'une base indexée s'estime donc comme un premier index.

## Ce que coûte vraiment un repassage de l'écrivain

Les `Scope` sortent identiques, avec les mêmes morceaux et sans vecteur neuf. Chaque ligne `File` neuve
reçoit en revanche son morceau (`File_CHUNKED_FROM` 7 608 → 15 218 chez banc), soit environ un petit
morceau par fichier. Le coût de l'écrivain est à part : un `File` dont seule l'analyse change pourrait
garder son morceau.

## Le témoin

Rouge d'abord : sur une base déjà indexée, sans dette, l'estimation d'une réindexation sans changement
annonce zéro vecteur et ne demande pas de confirmation.

## Ce qu'il faut pour le fermer

L'estimation d'une base déjà indexée part de la dette réelle et de ce qui changera (les fichiers dont
l'empreinte diffère), pas de la source entière ; le témoin vert.
