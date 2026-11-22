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

## Le même geste pour les champs : `contains` sur `fields`, un seul budget

Besoin de recherche (relayé par c7) : la passe de la requête entière (règle de Lucie du 25 octobre : la phrase entière,
l'exact devant) émet un `contains` flou **par champ** (`content`, `name`…) dans un booléen. Un document qui contient la
phrase avec une faute dans deux champs paie −2000, contre −1000 s'il ne l'a que dans un. C'est l'avertissement « plusieurs
`contains` flous dans un booléen » vu sur 17 requêtes sur 17.

**La proposition** : `contains` accepte `fields` (pluriel), avec **un seul compte d'erreurs par document**. C'est le geste
de `contains_split`, appliqué aux champs au lieu des mots. Deux variantes du combinateur :
- **(f1) le meilleur champ** : le score du document est celui de son meilleur champ (le moins de fautes, puis le meilleur
  BM25), comme un `disjunction_max` qui saurait lire les paliers. C'est le plus sûr : un champ de plus ne change rien ;
- **(f2) les fautes du meilleur champ, les BM25 de tous** : le compte d'erreurs est le minimum sur les champs trouvés,
  et le BM25 la somme des champs trouvés. Un document qui contient la phrase dans son nom et dans son contenu passe
  devant celui qui ne l'a que dans son contenu, sans payer deux fois la même faute.

Ma recommandation : (f2), qui récompense la présence dans plusieurs champs sans la punir. Il n'y a ici pas de mot absent,
puisqu'un champ n'est pas un mot de la requête : les champs sans correspondance ne coûtent rien, seule la meilleure faute
compte.

C'est une extension d'API (`fields` lu par `contains`, qui l'ignore aujourd'hui avec un avertissement). Avec les
corrections ci-dessus, cela fait **une seule version, plutôt 4.5.0** que 4.4.2. Le banc mesure le tout ensemble. Rien
n'est publié avant le mot de Lucie.
