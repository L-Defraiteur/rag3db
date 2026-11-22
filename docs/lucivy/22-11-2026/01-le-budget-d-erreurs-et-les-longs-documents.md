# Le budget d'erreurs et les longs documents

Session lucivy, 22 novembre 2026, après la mesure de banc
(`02-le-chemin-des-mots-en-contains-split`). La 4.4.2 (non publiée) donne à `contains_split` avec une distance un budget
d'erreurs par document : `−1000 × (fautes des mots trouvés + (distance + 1) par mot absent) + somme des BM25`. Le banc
mesure un recul léger : les questions en français perdent un « premier » (2 → 1) et un rappel (6 → 5), et « rebuild the
vector index » ne rend plus `catalog.rs:2288`.

## Ce qui se passe

Deux effets, qui s'additionnent :
1. **Un mot vide coûte autant qu'un mot rare.** La définition de code nommée par « rebuild the vector index » ne
   contient pas « the » : ce mot absent lui coûte un palier entier, autant que l'absence de « vector ». Une page
   Markdown qui contient les quatre mots, quelque part, reste au palier 0.
2. **La dispersion ne coûte rien.** Le palier ne regarde que la présence de chaque mot dans le document, pas leur
   distance. Un long document contient plus de mots, donc atteint plus souvent le palier 0. Le BM25 normalise bien la
   longueur, mais seulement à l'intérieur d'un palier, et un palier vaut 1 000.

Le premier effet suffit à expliquer l'exemple cité. Le second explique le reste du recul.

## Trois corrections, qui gardent l'esprit du choix de Lucie

Toutes gardent un `contains` seul tel quel, et l'exact devant le flou à mots égaux.

**(a) Le coût d'un mot absent pondéré par son IDF.** Un mot absent coûte `(distance + 1) × poids`, avec
`poids = idf(mot) / idf moyen des mots de la requête`, borné à [0, 1]. « the » est dans presque tous les documents : son
absence ne coûte presque rien. « vector » coûte plein tarif. Les fautes des mots trouvés gardent leur coût. C'est le
moins cher : les fréquences sont connues à la construction du poids, sans changer le chemin de recherche. Ça règle
l'exemple, pas la dispersion.

**(b) Le budget compté dans une fenêtre.** Le budget se calcule sur la meilleure fenêtre du document, de
`k × (nombre de mots)` jetons (`k` vers 4). Les mots présents ailleurs dans le document ne comptent pas comme trouvés.
Une définition qui réunit « rebuild … vector index » sur une ligne passe devant une page qui les disperse sur trois
sections. C'est la correction de fond de la dispersion. Elle demande les positions de chaque mot trouvé : elles existent
déjà pour les surlignages (`highlights` du préscan). Le coût est un balayage de fenêtre par document candidat.

**(c) Des paliers plus doux.** Le palier vaut `λ` au lieu de 1 000, et le BM25 est normalisé par requête dans [0, 1],
pour que la normalisation de longueur départage plus tôt. Avec `λ` vers 0,3, une faute coûte un tiers d'une
correspondance pleine. « L'exact devant le flou » ne tient plus qu'à pertinence comparable. C'est le plus simple à
écrire, mais il touche au choix de Lucie : je le garde en dernier recours.

## Ma recommandation

(a) d'abord : peu cher, il explique l'exemple cité. Puis (a) et (b) ensemble si le banc voit encore les longs documents
devant. Les trois peuvent vivre derrière une option de requête (`budget`: `"flat" | "idf" | "window" | "soft"`), sans
changer le comportement par défaut, pour que le banc les mesure sur ses quatre jeux sans une version par variante. Rien
n'est publié avant le mot de Lucie.
