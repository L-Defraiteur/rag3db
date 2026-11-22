# Le chemin des mots en `contains_split`

22 novembre 2026, matin, suite de [01-lucivy-4-4-2-sur-la-recherche-de-code.md](01-lucivy-4-4-2-sur-la-recherche-de-code.md).
La page 01 avait montré que la recherche par défaut n'atteignait pas `contains_split`. Recherche a fait passer le chemin
des mots (`Auto` sur une phrase, et le mode `Words`) au type `contains_split` de lucivy 4.4.2, avec la distance du flou.

## Le banc

Deux côtés, tous deux en lucivy 4.4.2 (git `972fbb7`, prouvé par le `Cargo.lock`), qui ne diffèrent que par ce
changement, sur la même copie reflink de `code-principal-migree-repassee-20261120`, chacun par son hôte :

| côté | commit | le chemin des mots |
|---|---|---|
| a | `f531e33c56` | texte pur + 4.4.2 + `ContainsSplit`, mais toujours un booléen de `contains` |
| b | `0337b78be9` (`words-contains-split`) | `contains_split` avec la distance |

Puis les mêmes jeux qu'en page 01, et la sonde du **signal BM25 seul** (17 requêtes, dont deux fautives).

Deux départs ont échoué avant le bon, pour des raisons de bâti, pas de code :
- le premier a dépassé ses 50 minutes de bâti sur un target neuf ;
- au second, la garde de cargo a refusé un target copié d'un autre worktree. Sa reprise documentée,
  `RAG3WEAVER_TARGET_REPRENDRE=1`, l'a permis au troisième.

## Les résultats

**Le changement atteint lucivy, et il bouge beaucoup le BM25.** 16 listes du BM25 seul sur 17 changent, la plupart
sans un seul résultat commun dans le top 10.

**Les premiers du BM25 seul deviennent souvent des pages Markdown entières**, au lieu de la définition de code :

| requête | côté a | côté b |
|---|---|---|
| construire la requête lucivy du BM25 à partir des mots | `search.rs:320` | `docs/…/05-journal-migration-fts.md:1` |
| la configuration du bac à sable d'une commande | `backend_code.rs:367` | `docs/…/18-comparatif-local-bonsai-qwen-magic.md:1` |
| index du plein texte | `catalog.rs:681` | un ticket fermé, `docs/tickets/closed/….md:1` |
| fusionner les listes classées par rang réciproque | `catalog.rs:9708` | `docs/…/02-la-mission-et-son-critere.md:1` |
| recherce plein texe (fautive) | un en-tête de test C++ | `catalog.rs:241` |

Un budget d'erreurs par document favorise les documents longs : une page de notes contient tous les mots de la
question, et n'y dépense presque rien. C'est un biais de longueur. La fusion le rattrape en partie, pas en entier.

**Dans la recherche complète, après la fusion, les questions françaises reculent un peu** :

| | côté a | côté b |
|---|---|---|
| noms exacts : or premier / rappel@10 | 13/20 / 18/20 | 13/20 / 18/20 |
| questions françaises : or premier | 2/9 | **1/9** |
| questions françaises : rappel@10 | 6/9 | **5/9** |
| premiers changés (les 10 de G / le jeu commun) | — | 0/10 / **2/10** |
| recouvrement du top 10 (G / jeu commun) | — | 98 % / 96 % |
| temps médian d'une recherche | 1 755 ms | 1 782 ms |

Ce qui recule :
- « le budget de recherche pris dans la requête » : rang 2, puis absent du top 10 ;
- « la configuration du bac à sable d'une commande » : rang 1, puis 2 ;
- « reconstruire l'index vectoriel » : rang 6, puis 7.

Ce qui monte : « relire les premiers résultats avec un cross-encoder », du rang 5 au rang 2.

Les deux premiers changés du jeu commun :
- « rebuild the vector index » quitte `catalog.rs:2288` (sa définition) pour un test C++ ;
- « translate a hop into a query » quitte `dialect.rs:325` pour `neighborhood_nodes.rs:1407`.

**L'avertissement de lucivy reste sur 17 requêtes sur 17 du côté b** : « fuzzy `contains` in one boolean ». Un autre chemin
envoie donc encore un booléen de `contains` flous, sans doute une seconde branche BM25 du graphe. Le passage à
`contains_split` n'est que partiel.

## Ce que j'en conclus

1. **Le chemin des mots en `contains_split`, tel quel, n'est pas à fusionner.** Il recule un peu sur la recherche de
   code : l'or premier passe de 2 à 1 sur 9, le rappel@10 de 6 à 5, et deux définitions perdent la première place du jeu
   commun. Il met des pages de notes devant le code dans le BM25.
2. **La cause à traiter est le biais de longueur du budget par document.** Une normalisation par la longueur du
   document dans le score de `contains_split`, comme le fait le BM25, ou une borne sur la part du score qui vient du
   budget, sont à voir avec lucivy. C'est une question de conception de lucivy : elle est à Lucie et à la session
   lucivy, pas au banc.
3. **Le reste du chemin est à trouver** : l'avertissement ne disparaît pas, et une autre branche construit encore le
   booléen à la main.
4. **La publication de 4.4.2 n'est pas touchée** : rag3weaver n'utilise `contains_split` qu'avec cette branche. 4.4.2
   publiée sans elle ne change rien à la recherche de code (page 01).

Les sorties sont à luciepc, sous `~/.cache/rag3weaver-build/banc-parite/sorties/lucivy-mots-*` : `lucivy-*.json` et
`bm25-*.json`. Le banc est `lucivy-mots.sh` (`REF_REV`, `WORDS_REV`).
