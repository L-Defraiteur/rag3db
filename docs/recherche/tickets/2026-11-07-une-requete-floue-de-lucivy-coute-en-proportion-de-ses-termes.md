# Une requête floue de lucivy coûte en proportion de ses termes, sans borne

- **État** : ouvert, le 7 novembre 2026, à la demande de c7 ; pour la session lucivy. Le pendant
  produit est le ticket « Le plein texte explose sur une requête longue » (même jour), qui borne
  les termes dans `BM25SearchNode` en attendant.
- **Gravité** : blocage. Une requête de 197 mots tient le moteur 33 s, une de 102 mots 13 s, pour
  2 ms à un mot (mesures de rag3weaver archi sur l'hôte en service, deux fiches).
- **Atteignable en service** : oui, pour tout client de lucivy qui passe une valeur longue à une
  requête `parse` sans syntaxe (un OU de `contains` flous par mot et par champ) ou un `contains`
  relâché d'un long texte.
- **Touche rag3weaver** : non (le correctif vit dans lucivy ; rag3weaver le contourne).
- **Famille** : produit

## Ce que c'est

Le coût d'une requête floue croît avec son nombre de termes, et ne dépend presque pas de la
taille de la base : deux fiches suffisent. Rien, dans l'analyseur de requête de lucivy, ne borne
le nombre de clauses floues ni ne prévient l'appelant.

## Le témoin

La courbe durée selon le nombre de termes distincts (1 à 197, distance 0 et 1) est mesurée par la
session recherche sur deux bases réelles et portée au ticket produit ; un témoin lucivy pourrait
reprendre la même série sur un index de deux documents.

## Ce qu'il faut pour le fermer

Dans lucivy : une borne de termes (ou de clauses floues) dans l'analyseur de requête, avec un
avertissement dans `query_warnings` qui dit la coupe — les clients en profitent tous ; ou un coût
qui ne croît plus ainsi. Puis la borne du produit se relâche, mesure à l'appui.
