# Le dense a son embarqueur à la demande à côté de la tenue générique

- **État** : fermé (14 novembre 2026, embarquement) : le dense passe par `Tenue<C>`, `ALaDemande` et `Maintenu` ont disparu ; ouvert le 14 novembre, laissé par le lot des activations nommées
- **Pour** : embarquement
- **Famille** : produit
- **Gravité** : dette. Deux codes font la même chose : `ALaDemande` et `Maintenu` pour le dense, et
  `Tenue<C>` pour le creux, le relecteur et l'OCR. Une correction dans l'un peut manquer à l'autre.
- **Atteignable en service** : non, c'est le code.
- **Touche rag3weaver** : oui (`src/model_source/activation.rs`)

## Ce que c'est

Le lot des activations nommées a écrit une tenue générique (`Tenue<C>`) : l'activation au premier
appel, le bail rendu après le délai, la relance unique et le refus dit. Elle porte les trois capacités
neuves. Le dense, joué en vrai le 13 et le 14 novembre, a gardé son code pour ne pas risquer de
régression dans le même lot.

## Ce qu'il faut pour le fermer

Porter le dense sur `Tenue<C>`, en gardant ce que `ALaDemande` cache après l'activation (factice,
lot conseillé). Les témoins `a_la_demande_*` et `maintenir_*` restent verts.

## Le portage (14 novembre 2026, embarquement, branche `dense-sur-tenue`)

- **Le dense** passe par `Tenue<C>`, comme le creux, le relecteur et l'OCR : `DenseTenu` dit son modèle
  et sa dimension déclarés, et garde ce que le client activé dit de lui (factice, lot conseillé,
  troncatures). `ALaDemande`, `Maintenu`, `connecter` et `Gere` disparaissent.
- **Un dense maintenu** s'active à l'ouverture, sous l'annulation de l'appelant, et rend l'adresse que
  le fournisseur a prise (port 0 compris), comme avant.
- **Pas de rouge** : c'est un remaniement à comportement égal (c7 : « rouge d'abord veut dire ne pas
  prétendre »). Le seul trou réel, le témoin A qui ne lisait pas `relances: 1` au /sante, a été posé
  avant le portage. Il était vert avant tout code, parce qu'`ALaDemande` notait déjà sa relance.
- **Les témoins**, verts après : les vingt tests de l'activation (trois tours), ceux du service (28) et
  le bâti sans feature.
