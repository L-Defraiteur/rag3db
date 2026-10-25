# Le témoin du sous-graphe de valeurs annulé rougit sous charge

- **État** : ouvert — vu une fois, aucun correctif
- **Gravité** : faux rouge d'un témoin, sans défaut connu du produit
- **Atteignable en service** : non (c'est le témoin qui dépend de l'ordre)
- **Touche rag3weaver** : oui (`src/dataflow/annulation_tests.rs`)

## Ce que c'est

`dataflow::annulation_tests::a_value_subgraph_dies_with_its_levels_parent` arme une
annulation à 50 ms, puis attend que le nœud intérieur `Wait` **ait été réveillé** par elle
(`woken == 1`). Le nœud ne compte un réveil que si l'annulation arrive *pendant* son attente.
Si le fil du sous-graphe démarre après les 50 ms, le jeton est déjà levé : `wait_timeout`
rend vrai sans réveil, ou le nœud ne s'exécute pas du tout. `woken` reste alors à 0, alors que
le sous-graphe est bien mort avec son parent (la borne de temps, < 2 s, passe).

## Vu

Le 25 oct. 2026, 16 h, au rejeu de cited_by sur luciepc (master e5aba0f6b), avec la lib
entière en parallèle : `left: 0, right: 1` (« le nœud intérieur a vu l'annulation »). Le
rejeu suivant, sur 527a0356c, était vert, et le lot ne touche ni le runtime ni l'annulation.

## Piste

Que le témoin juge ce qu'il promet : le sous-graphe s'arrête avant ses 5 s, et le nœud
intérieur n'achève pas son attente. Par exemple, compter « attente achevée sans
annulation » plutôt que « réveillé par l'annulation », ou n'armer l'annulation qu'une fois
le nœud intérieur entré dans son attente (une barrière).
