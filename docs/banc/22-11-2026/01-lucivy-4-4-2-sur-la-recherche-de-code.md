# lucivy 4.4.2 sur la recherche de code

22 novembre 2026, nuit. Demande de c7 : mesurer lucivy 4.4.1 contre 4.4.2 (branche `fuzzy-error-budget`, rev
`972fbb7bf1e1`) sur la recherche de code de rag3weaver, avant que Lucie décide de la publication.

**Le choix de Lucie pour 4.4.2** : seul le type `contains_split` avec une distance porte un budget d'erreurs par
document (un mot absent coûte distance + 1). Le booléen générique ne change pas : il garde sa somme et ajoute un
`query_warnings`. Un `contains` seul est inchangé.

## Le banc

Quatre côtés, chacun bâti de son commit avec les features de service, sur la même copie reflink de
`code-principal-migree-repassee-20261120` (indexée le 20 nov.), chacun par son hôte. La lib épinglée est 2c4bddcd3c.

| côté | lucivy (prouvée par le `Cargo.lock`) | rag3weaver | commit |
|---|---|---|---|
| 1 | 4.4.1, crates.io | master | `dd874f5468` |
| 2 | 4.4.2, git `972fbb7` | `build_bm25_query` inchangé | `0dcbd31acf` |
| 3 | 4.4.2, git `972fbb7` | `ContainsSplit` émet `contains_split` | `bf01b828e9` |
| 4 | 4.4.2, git `972fbb7` | 3, et `Auto` envoie une phrase à `ContainsSplit` | `47c237363a` |

**Une première tentative a échoué** : la branche de recherche ne compilait pas avec les features de service, parce que
`lucistore` était en double (crates.io par `sparse-vector`, git par le patch). Elle épinglait aussi encore lucivy
4.4.0. Recherche a corrigé les deux : le patch couvre maintenant `lucistore` et `sparse-vector`, et les commits sont
reposés sur master.

**Les jeux** :
- 20 noms exacts, l'or étant leur ligne de définition ;
- 15 questions en français avec une définition attendue, dont 9 dont l'or existe dans la base du 20 ;
- les 10 requêtes de G et les 10 du jeu commun ;
- puis une sonde avec **le signal BM25 seul** sur 17 requêtes, dont deux fautives (« recherce plein texe »,
  « fusioner les listes »).

## Les résultats

| comparé à 4.4.1 | côté 2 | côté 3 | côté 4 |
|---|---|---|---|
| noms : or premier / rappel@10 | 13 → 13 / 18 → 18 | 13 → 13 / 18 → 18 | 13 → 13 / 18 → 18 |
| questions françaises : or premier / rappel@10 | 2 → 2 / 6 → 6 | 2 → 2 / 6 → 6 | 2 → 2 / 6 → 6 |
| premiers changés (G, jeu commun) | 0/10, 0/10 | 0/10, 0/10 | 0/10, 0/10 |
| recouvrement du top 10 | 100 % | 100 % | 100 % |
| temps médian d'une recherche | 1 739 → 1 848 ms | 1 739 → 1 741 ms | 1 739 → 1 797 ms |
| BM25 seul (côté 4) | — | — | listes identiques sur 17/17 |

**Rien ne recule, et rien ne bouge.** Le BM25 seul est identique, donc ce n'est pas la fusion qui noie un changement :
lucivy ne reçoit pas une autre requête.

## Pourquoi rien ne bouge

La réponse du côté 4 porte l'avertissement que 4.4.2 ajoute au booléen générique : « 3 fuzzy `contains` in one boolean:
their typo penalties add up and a word not found costs nothing… — for the words of one query, use `contains_split` with
`distance` ». **Même avec `Auto` → `ContainsSplit`, la requête de `search_code` arrive à lucivy comme un booléen de
`contains` flous construit à la main.**

Le côté 4.4.1 le montre aussi : « parse without boolean operators: … runs as OR of substring contains, one per word ».
Le chemin de la recherche par défaut passe donc par la construction du mode « parse », pas par `build_bm25_query` en
`ContainsSplit`. Le changement de recherche n'y est pas atteint.

## Ce que j'en conclus

1. **Pour la publication de 4.4.2 : rien ne recule sur la recherche de code de rag3weaver.** Mêmes résultats partout,
   temps de recherche de +0 à +6 %. De ce côté, rien ne s'oppose à la publication. Elle reste au mot de Lucie.
2. **4.4.2 n'apporte encore rien à la recherche de code** : aucune requête de `search_code` n'atteint `contains_split`.
3. **Pour que le budget d'erreurs serve**, le chemin qui fabrique le booléen de `contains` dans le mode « parse » doit
   émettre `contains_split`. C'est le travail de recherche. Le banc est prêt pour le remesurer (`lucivy-442.sh` à
   luciepc, quatre côtés, puis `bm25-seul.sh`). Le critère : l'avertissement de lucivy disparaît, et des listes BM25
   changent sur les requêtes fautives.

Les sorties sont sous `~/.cache/rag3weaver-build/banc-parite/sorties/lucivy-442-20261122/` à luciepc :
`lucivy-*.json`, `bm25-*.json`, les journaux des bâtis et des hôtes.
