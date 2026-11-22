# Une phrase avec une faute ne se retrouve plus, jusqu'à la 4.4.2

- **État** : ouvert (22 novembre 2026, session recherche, lot `recherche-whole-phrase-exact`), voulu : fermé par lucivy 4.4.2.
- **Gravité** : résultat manquant
- **Atteignable en service** : oui.
- **Touche rag3weaver** : oui (la recherche par défaut de `search_code`, passe de la requête entière et recherche par mots).
- **Famille** : produit
- **Pour** : recherche
- **Signalé par** : recherche, en posant la passe entière à distance 0 (avis de lucivy, décision de c7).

## Le pas exact

« refreh index states » (une requête du tableau de Lucie du 25 octobre 2026) : la passe de la requête entière la
cherchait à distance 1 et la trouvait, mais un `contains` flou ignore les séparateurs, et cherchait la phrase collée
en un mot à une faute près. Plusieurs mots se cherchent maintenant à distance 0 ; la recherche par mots, sans
distance sur master, ne rattrape pas « refreh » non plus. La phrase avec une faute n'est plus trouvée.

## Pourquoi pas de rattrapage provisoire

Des mots flous dans un booléen (un `contains` à distance 1 par mot) ramèneraient la somme des pénalités par clause
qu'on quitte : un document qui trouve plus de mots avec une faute passe derrière un document qui en trouve un seul.

## Pour le fermer

La recherche par mots en `contains_split` avec la distance du flou et le budget d'erreurs de lucivy 4.4.2 (branche
`words-contains-split`), à la publication de la 4.4.2 ; un témoin : « refreh index states » trouve le document qui dit
« refresh index states ».
