# 474 relations dépendent encore de la taille du paquet

- **État** : corrigé — 9b89ec823 (clés stables) et 4fe5a3bfc (pointeur codeparsers d51e933)
- **Gravité** : réponse fausse (le graphe dépend du découpage)
- **Atteignable en service** : oui (première indexation, synchronisation)
- **Touche rag3weaver** : oui

## Ce que c'est

Remesure du dépôt entier par la session embarquements, à corpus identique :
305 357 relations par paquets de 64, 304 883 à 512. La sortie de codeparsers
n'en est pas la cause : en fichier seul, un appel et 85 paquets y rendent
les mêmes 371 289 relations, une à une. L'écart naît dans
`code::analyze_in_project` ou après.

## Témoin

`tests/sonde_liste_analyse.rs::relations_selon_le_paquet` (branche rag3db
`liens`, à jouer) : relations et rendez-vous en attente seulement d'un côté,
par type et par extension.

## Ce que la sonde a trouvé (4 octobre, 12 h 20)

`analyze_in_project` sur le dépôt entier, 64 contre 512 : 314 029 contre
313 510 relations ; 660 seulement à 64, 141 seulement à 512 ; rendez-vous :
720 138 des deux côtés, 59 qui ne diffèrent que par la clé.

**Tous les écarts sont dans un même fichier, et ce sont des clés** : le
suffixe d'homonyme de la clé stable change avec le paquet —
`declarations_of.Closure:lambda#5` contre un autre numéro,
`main.set_comprehension:lambda#1` à 64 et `#2` à 512, `s:variable#1`
contre `s:variable`. S'y ajoutent des auto-arêtes de classe C++
(`SelectionView:class → SelectionView:class`, seulement à 64 : deux scopes
de même clé dans un paquet — sans doute une déclaration anticipée et sa
définition) et 282 `HAS_PARENT` / `PARENT_OF`.

## Cause probable

La numérotation des homonymes (`stable_scope_keys`, `code.rs`) dépend de
quelque chose qui varie avec le paquet — un ensemble ou un ordre construit
par paquet (repli des lambdas ?). La sortie de codeparsers n'y est pour
rien : identique, ordre compris, quel que soit le paquet (empreinte
canonique de chaque fichier). À l'arbre principal.

## Pour le fermer

La sonde, puis le correctif, puis le test d'égalité des graphes de l'arbre
principal étendu au dépôt entier.

## Après les clés stables (9b89ec823, rejoué le 4 octobre, 15 h 37)

Sur master `18b1a6889` : 110 relations seulement à 64, 0 seulement à 512,
rendez-vous identiques — les numéros d'homonymes sont réglés. Les 110
restantes sont 55 `CONSUMES` (et leurs inverses), **toutes des boucles sur
soi** de classes C++ d'en-tête (`graph.h#rag3db.Graph:class → la même`) :
la classe mentionne son propre nom, et le rendez-vous la relie à elle-même
quand elle est le seul définisseur du paquet. Pour le fermer : le
rendez-vous ne relie jamais un scope à lui-même.

## Fermé (4 octobre, 17 h)

La cause des boucles n'était pas le rendez-vous mais codeparsers : le nom
d'un destructeur C++ (`~Graph`) était lu comme une référence à la classe,
et la résolution locale reliait le scope à lui-même. Corrigé dans
codeparsers d51e933 (un identifiant `destructor_name` est une définition ;
`resolve_local_scope_references` ne rend jamais de relation vers soi),
pointeur sur master en 4fe5a3bfc.

Sonde rejouée sur master 6380dd8a5, dépôt entier :

```
[paquets] relations 64 : 264156, 512 : 264156 ; rendez-vous 64 : 601929, 512 : 601929
[relations seulement à 64] 0      [relations seulement à 512] 0
[rendez-vous seulement à 64] 0    [rendez-vous seulement à 512] 0
[boucles] 0 CONSUMES d'une clé vers elle-même, paquets de 64
```

Témoins : `tests/sonde_liste_analyse.rs::relations_selon_le_paquet` et
`::boucles_sur_soi` (`#[ignore]`, dépôt entier).
