# Un fichier de plus apparaît sous `templates/` pendant la lib entière, une fois

- **État** : ouvert (13 novembre 2026), arbre principal.
- **Famille** : produit
- **Gravité** : la lib entière rougit sans cause dans le lot qui la joue, une fois.
- **Atteignable en service** : non (un test).
- **Touche rag3weaver** : oui (`src/builtin_templates.rs`, le témoin ; le fautif, s'il y en a un, est
  un autre test).

## Ce que c'est

`builtin_templates::tests::every_shipped_template_is_embedded_and_sorted` a rougi une fois, au tour de
la lib entière du lot 1a (branche `arbre-principal/scope-version-2`), au portable, lib `e93710f36` :

```
assertion `left == right` failed: chaque fichier de templates/ est embarqué
  left: 224
 right: 223
```

Après coup, `templates/` compte 223 fichiers, exactement les 223 que git suit, et aucun fichier
ignoré. Un fichier a donc existé un moment sous `templates/` pendant la lib, puis disparu. Il n'est
pas nommé. À luciepc, sur un target neuf, la même lib entière est verte (2345/0/70).

Ma piste, non prouvée : un test joué en parallèle écrit à côté d'un gabarit qu'il prend pour source
(une base, un verrou, une marque d'écrivain), puis l'efface. Une vingtaine de tests ouvrent
`templates/backends/code` ou `templates/` comme source.

## Ce qu'il faut pour le fermer

Nommer le fichier. Il faut, à la prochaine lib entière au portable, une boucle qui écrit chaque nom au
fil de l'eau, et non un `sort -u` final : le premier essai, à luciepc, n'a rien enregistré. Ensuite,
faire écrire ce test hors de `templates/`.
