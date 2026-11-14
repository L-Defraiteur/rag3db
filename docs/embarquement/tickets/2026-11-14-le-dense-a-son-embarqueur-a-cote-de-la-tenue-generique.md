# Le dense a son embarqueur à la demande à côté de la tenue générique

- **État** : ouvert (14 novembre 2026, embarquement, laissé par le lot des activations nommées)
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
