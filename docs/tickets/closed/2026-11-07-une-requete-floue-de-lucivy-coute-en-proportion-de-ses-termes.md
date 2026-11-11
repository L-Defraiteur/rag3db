# Une requête floue de lucivy coûte en proportion de ses termes, sans borne

- **État** : **corrigé** dans lucivy 4.3.2 (publiée le 7 novembre 2026 : crates.io, npm, PyPI, tag
  `v4.3.2`). La cause n'était pas le nombre de termes mais la longueur de la requête entière cherchée
  en flou : sur un index à plusieurs shards, le planificateur des briques de lucivy comptait dans le
  dictionnaire toutes les sous-chaînes de la requête (O(n²)) avant de chercher, alors qu'à distance 1
  seuls les préfixes et les suffixes servent. Il ne prépare plus que les pièces que la recherche peut
  tarifer ; à distance 2 et plus, au-delà de 64 octets, les coupes sont cherchées près de la coupe
  égale. Témoin lucivy `test_fuzzy_many_terms` : 200 mots, 1 356 caractères, à distance 1 sur deux
  documents et quatre shards, 15 799 ms avant, 96 ms après ; rappel inchangé (une requête de
  300 caractères avec une faute, et des passages avec deux fautes à distance 2, retrouvés).
  rag3weaver prend `lucivy-core = "4.3.2"` depuis `0a8ff53d9` et a relevé sa borne L à 900 caractères
  (`217dd10a1`). Le pendant produit est le ticket « Le plein texte explose sur une requête longue »
  (même jour, fermé).
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
