# Une entité au nom réservé (« Order ») casse l'ouverture du backend, au lieu d'être refusée au chargement

- **État** : ouvert (trouvé le 11 octobre 2026 par l'e2e du backend jouet du
  proto « tout déclaratif », chantier I).
- **Gravité** : blocage — le backend ne s'ouvre pas, avec une erreur du
  parseur Cypher qui ne nomme ni le manifeste ni la règle.
- **Atteignable en service** : oui, pour tout manifeste qui déclare une entité
  dont le nom est un mot réservé de Cypher (`Order`, `Match`, `Return`,
  `Limit`, `Copy`, `Key`…). Un agent qui déclare une boutique écrira `Order`.
- **Touche rag3weaver** : oui (la création des tables d'entités).

## Ce que c'est

`PreparedBackend::load` accepte une entité nommée `Order` (l'identifiant est
valide), puis `Backend::open` envoie au moteur
`CREATE NODE TABLE IF NOT EXISTS Order(…)` sans échapper le nom, et le
parseur refuse :

```text
Parser exception: mismatched input 'Order' expecting {…} (line: 1, offset: 32)
"CREATE NODE TABLE IF NOT EXISTS Order("
```

## La recette minimale

```cypher
CREATE NODE TABLE IF NOT EXISTS Order(key STRING, PRIMARY KEY(key));   -- refusé
CREATE NODE TABLE IF NOT EXISTS `Order`(key STRING, PRIMARY KEY(key)); -- accepté
```

Côté rag3weaver : un manifeste qui déclare `"entities": {"Order": …}`, ouvert
sur une base (`templates/proto/boutique` avant le renommage de son entité en
`Purchase`).

## Le témoin

Aucun aujourd'hui (assumé) : l'e2e du jouet (`tests/e2e_proto_boutique.rs`) a
rougi ainsi avant que son entité soit renommée. Pour l'écrire : un e2e qui
ouvre un backend déclarant une entité `Order` et attend soit une base qui
marche (noms échappés partout), soit un refus **au chargement** qui nomme le
mot réservé.

## La cause

Les noms d'entités (et sans doute de relations et de champs) sont interpolés
tels quels dans le DDL et les requêtes émis par le catalogue et le dialecte,
sans accents graves. Le passage par l'IR (« jamais de Cypher au-dessus ») est
le bon endroit : le dialecte rag3db échapperait tout identifiant qu'il écrit.

## Pour le fermer

Au choix de la session embarquements (qui tient l'IR et le dialecte) :
échapper tout identifiant dans ce que le dialecte rag3db écrit (DDL compris),
avec le témoin ci-dessus vert ; ou, à défaut, refuser au chargement un nom
d'entité, de relation ou de champ réservé, en le nommant.
