# Un centre de gravité sémantique par fiche (souhait)

- **État** : ouvert, le 18 novembre 2026. C'est un souhait, pas un lot : à reprendre après les
  deux étapes de l'archiviste et la mesure des tailles de fiches par le banc.
- **Famille** : produit
- **Gravité** : aucune défaillance. C'est une protection à venir contre une fiche qui avale ce qui
  est distinct (le riche qui s'enrichit), et un rappel moins cher.
- **Atteignable en service** : non, rien n'existe encore.
- **Touche rag3weaver** : oui. Le gabarit memory (deux champs sur Fiche, les scripts du
  rangement), et peut-être un nœud de calcul de centroïde si un RhaiNode ne suffit pas sur 1 024
  dimensions.
- **Pour** : la session mémoire (96).
- **Signalé par** : Lucie, le 18 novembre 2026, par c7. L'esquisse est dans
  `docs/memoire/18-11-2026/02-l-archiviste-en-deux-etapes.md`, section (d), « Une esquisse : le
  centre de gravité sémantique d'une fiche ».

## Ce qui serait juste

1. Chaque fiche porte :
   - un centroïde, la moyenne incrémentale des vecteurs de ses L1 ;
   - une dispersion, son rayon ;
   - une ancre, le vecteur de sa description, fixé à la création.

   La **dérive** (la distance du centroïde à l'ancre) au-delà d'un seuil fait proposer de relire
   ou de scinder. Les catégories n'ont pas de dispersion comme critère : leurs valeurs se
   définissent par leurs différences, et leur qualité se juge par le modèle (une même question,
   des valeurs qui s'excluent).
2. Au rangement, un sujet est absorbé sous le rayon, si la cohésion ne baisse pas au-delà d'un
   seuil.
3. Le rappel des candidates se fait d'abord par centroïde.
4. Une dispersion au-delà d'un seuil fait proposer `scinder_fiche`. Deux centroïdes trop proches
   font proposer `rejoindre_fiches`. Ce sont des propositions, jamais des gestes faits seuls.
5. Tous les seuils sont des réglages du gabarit, avec des défauts mesurés.

## La mesure d'abord

Le banc relève, sur la session de Lucie rangée :

- la distribution des tailles de fiches (une fiche avale-t-elle 40 % des L1 ?) ;
- la dispersion intra-fiche ;
- la dérive des fiches à leur ancre.

Les seuils se posent sur ces chiffres, pas avant.

## Le témoin, le jour du lot

`a_topic_far_from_the_centroid_does_not_join_a_large_fiche` : une grosse fiche au rayon large, et
un sujet distinct proche d'un seul de ses L1. Le sujet ne rejoint pas la fiche, et la dispersion
au-delà du seuil fait proposer `scinder_fiche`.
