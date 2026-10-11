# La même fiche écrite dans deux cellules n'a qu'une ligne, et la seconde écriture la déplace

- **État** : ouvert (11 octobre 2026). Le témoin rouge est sur la branche `marche-1-bis`. La
  forme candidate est à la page `extension/rag3weaver/docs/11-octobre-2026-arbre-principal/04-la-marche-1-bis.md`,
  point 1. C'est une migration : **le mot de Lucie avant le code**.
- **Gravité** : perte. La fiche de la première cellule disparaît de cette cellule, sans un mot.
- **Atteignable en service** : oui, dès que deux cellules écrivent la même identité. Une même
  mémoire « globale » et « de ce projet », ou deux projets qui indexent le même fichier, suffisent.
  Sous la forme C, deux sessions sur deux cellules d'un même backend y arrivent directement.
- **Touche rag3weaver** : oui (le catalogue, toute entité).

## Ce que c'est

L'`_uuid` d'une ligne se calcule sur ses champs déclarés (`hashsafe`), ou sur tous ses champs à
défaut (`Catalog::uuid_for`, `catalog.rs:1781`), **sans sa cellule**. La même fiche écrite dans
deux cellules a donc le même `_uuid`. L'écriture dans la seconde cellule retrouve la ligne par sa
clé et la réécrit avec la nouvelle cellule (`_org`, `_project`) : la fiche quitte la première
cellule.

## Témoin

`tests/e2e_scope.rs::la_meme_fiche_dans_deux_cellules_fait_deux_lignes` (branche
`marche-1-bis`), rouge sur master :

```
une ligne par cellule ; uuids : ["d820690e-ccdd-a554-a26e-5f0bf1380e97"]
  left: [("acme", "beta")]
 right: [("acme", "alpha"), ("acme", "beta")]
```

## Pour le fermer

L'identité dérivée de (cellule, identité déclarée), avec la migration des lignes existantes (la
cellule par défaut garde ses `_uuid` si on ne fait entrer dans le hachage que les cellules autres
que le défaut). Il faut aussi trancher les liens entre cellules : le lien générique de la mémoire
nomme ses bouts par leur identité, et doit dire dans quelle cellule il les cherche. Le témoin vert,
et `e2e_scope` entier.
