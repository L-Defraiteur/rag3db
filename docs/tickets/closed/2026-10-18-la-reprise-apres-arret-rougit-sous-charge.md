# La reprise d'une réaction coupée rougit sous charge : la lecture voit encore les fichiers retirés

- **État** : fermé (18 octobre 2026) — une course du test, pas un défaut du moteur ni de 113e388fd ; le test attend ce qu'il garde
- **Gravité** : test instable (un rouge sur une lib entière, sous charge) ; en service, peut-être
  une lecture qui précède la reprise
- **Atteignable en service** : peut-être — un appel lu juste après le redémarrage d'un hôte dont
  la réaction avait été coupée
- **Touche rag3weaver** : oui (`backend.rs`, `dataflow/reindex_nodes.rs`, `catalog/retard.rs`)
- **Ouvert le** : 18 octobre 2026, everything declarative, au rejeu de `labels_from` sur luciepc
- **Pour** : everything declarative

## Ce qu'on voit

`backend::tests::stopping_cuts_a_long_reaction_and_the_restart_resumes_it`, sur luciepc, à
2cb6b2524 (avec le patch de `labels_from`, qui ne touche que `decide_nodes.rs`), lib du magasin
0fc7dd10f :

```
panicked at src/backend.rs:4595:9:
le reste n'est pas repris : … « Une écriture de l'index est en cours : la réponse porte sur
l'état validé, d'avant elle. » … ## Usages (101) … function f0 — lot/f0.rs:1 …
```

- **Une fois**, dans la lib entière (1601 verts, 1 rouge), pendant que le cœur C++ jouait
  `transaction_test` et `c_api_test` en lourd sur le même poste.
- **Six fois sur six vert, seul**, juste après (9,1 à 9,4 s chacun).
- Vert sur ce poste dans la lib entière à 113e388fd (base d888b6365), le même jour.

## Ce que le test suppose

Il supprime cent fichiers, attend que le retard soit noté, arrête les réactions, les remonte,
puis **attend que le retard tombe** avant de lire les usages une fois. La lecture a répondu
pendant qu'un écrivain tenait le catalogue, et la réponse portait encore les cent fichiers :
quand le retard est tombé, la reprise n'avait pas fini.

## Hypothèses, non départagées

1. **Le retard n'est pas encore noté** quand la boucle le lit. La première attente ne demande
   qu'un fichier noté. Le premier lot de la réaction en compte quatre. Sous charge, il est
   possible que seuls les fichiers déjà notés aient été traités avant l'arrêt. Au redémarrage,
   aucun retard ne serait alors visible tant que la réconciliation n'a pas noté le reste : la
   boucle sortirait tout de suite. C'est une course du test, plus ancienne que la montre en lots.
2. **Le retard tombe trop tôt** depuis 113e388fd : `lag.done` vient après l'écriture du lot,
   avant l'embarquement hors du verrou. Pour des retraits, l'embarquement n'a rien à faire, mais
   il prend deux fois le verrou.

La première se vérifie en relevant `index_states()["lag"]` à chaque tour de la boucle, et le
nombre de fichiers notés avant l'arrêt. La seconde, en rejouant sous charge à d888b6365.

## À ne pas faire

Allonger les délais du test sans avoir départagé : la ligne « le reste n'est pas repris » est ce
que le test garde.

## Le verdict (18 octobre 2026, sur luciepc)

Dix tours alternés, chacun la lib entière plus seize boucles CPU, les deux binaires bâtis une fois
(lib du magasin 87a362e30) :

| côté | reprise rouge | reprise verte |
|---|---|---|
| 113e388fd (le lot de l'embarquement hors du verrou) | 2 | 8 |
| 2b0c3739d (son parent) | 1 | 9 |

Le parent rougit aussi : ce n'est pas le lot. L'hypothèse 1 tient. Le retard n'est noté qu'au
premier tour de la montre remontée, et une réaction coupée a pu solder tout ce qu'elle avait
noté. Juste après le remontage, la boucle ne voyait donc aucun retard et lisait avant la reprise.

**Le correctif est dans le test**, pas dans le moteur. Il attend ce qu'il garde : plus aucun usage
dans `lot/`, lu seulement quand le retard est tombé, dans le même délai de 120 s. Si la reprise
ne se fait pas, il rougit toujours sur « le reste n'est pas repris ».

Les mêmes tours ont fait rougir trois autres tests sensibles au temps, des deux côtés : un ticket
à part, `2026-10-18-trois-temoins-rougissent-sous-charge.md`.
