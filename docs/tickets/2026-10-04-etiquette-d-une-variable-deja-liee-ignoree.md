# L'étiquette d'une variable liée plus tôt sans étiquette est ignorée

- **État** : corrigé le 4 octobre 2026, commit `26e099dea` (« fix(binder): l'étiquette d'un nœud lié plus tôt sans étiquette le restreint »)
- **Gravité** : réponse fausse
- **Atteignable en service** : oui
- **Touche rag3weaver** : non (vérifié par l'arbre principal)
- **Ouvert le** : 4 octobre 2026, revue des amonts
- **Pour** : binder (banc, périmètre levé le 4 octobre)

## Ce que c'est

Une variable d'abord liée sans étiquette, puis redonnée avec une étiquette plus loin (même motif, MATCH suivant, après un WITH) : l'étiquette est ignorée.

## Recette minimale

```cypher
CREATE NODE TABLE L1(id INT64 PRIMARY KEY);
CREATE NODE TABLE L2(id INT64 PRIMARY KEY);
CREATE NODE TABLE L3(id INT64 PRIMARY KEY);
CREATE REL TABLE T5(FROM L1 TO L3);
CREATE (:L1 {id: 1})-[:T5]->(:L3 {id: 2});
MATCH (n1), (n0:L3)<-[:T5]-(n1:L2) RETURN count(*);   -- attendu 0, obtenu 1
```

## Témoin

`test/transaction/concurrence/upstream_fixes_test.cpp` — ExplicitLabelOfAVariableBoundEarlier

## Cause

`src/binder/bind/bind_graph_pattern.cpp:573` : `addEntries` ne fait rien quand la variable existe déjà.

## Correctif de l'amont (à lire, ne pas copier)

Ladybug `413079079` (18 juillet 2026).

## Pour le fermer

restreindre les tables de la variable à l'intersection ; le témoin passe au vert.
