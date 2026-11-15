# L'uuid d'une ligne par son contenu, sans annexe : une variante à prouver

- **État** : ouvert (15 novembre 2026)
- **Famille** : produit (conception des générations, 2a′-3)
- **Gravité** : coût. Sans elle, une génération écrite hors d'ordre pose un doublon au bord haut du trou,
  qui coûte des lignes, jamais une identité
- **Atteignable en service** : oui, dès que 2a′-3 écrit une génération g après g+1 (la pointe d'abord,
  le rattrapage, la citation qui passe devant)
- **Touche rag3weaver** : oui (l'uuid des lignes `Scope` et `File`, l'écrivain de `code_sync.rs`)
- **Pour** : everything declarative (ED), avec arbre principal
- **Signalé par** : ED, page `docs/everything-declarative/15-11-2026/01-la-mesure-de-2a-prime-3.md`, § 5 ;
  mis en ticket par c7 (15 novembre 2026, 20 h). Elle ne retarde pas 2a′-3, qui se code sur (2)

## Ce que c'est

La mesure de 2a′-3 donne 0 réapparition sur 702 changements, sur 50 commits de code. Elle garde (2), les
deux colonnes `_depuis` et `_jusqu_a` et le doublon au bord haut, devant (3a), à ×2 en lecture, et (3a′),
à ×1,3 à vide parce que le moteur n'élague pas l'annexe vide.

La variante :
- l'uuid d'une ligne est `hash(clé, couche, empreinte)`, l'empreinte étant la rangée sans la position ;
- écrire g après g+1, pour une ligne au même contenu, retrouve la même ligne et descend son `_depuis` à g.
  Ni doublon, ni annexe, et les arêtes gardent leur cible ;
- une vraie réapparition (A → B → A) retombe sur le doublon de (2), avec
  `hash(clé, couche, empreinte, depuis)`, compté dans l'état ;
- le prédicat reste celui de (2).

## Ce qui est à prouver avant de la prendre

1. **La descente de `_depuis` face au cache par rang**, d'abord. Les lignes de 2a′ sont immuables, sauf
   `_jusqu_a` à la retraite. La descente n'ouvre une ligne qu'au rang rattrapé, qui n'avait rien de publié ;
   il faut le prouver contre le cache par rang (page 06 du 14 novembre) : aucune lecture épinglée à g ne
   voit la base avant la pose de g.
2. **L'ordre dans la transaction** : la descente et les naissances de g dans la même écriture, pour qu'aucun
   lecteur ne voie g à moitié.
3. **Le compte de l'état** : les lignes reprises par descente, et les doublons de réapparition.

## Comment le fermer

Par une page qui prouve les trois points, avec un témoin rouge d'abord (une lecture épinglée qui verrait une
ligne descendue avant la pose de son rang), puis la décision d'arbre principal et de c7. Ou bien par un refus
écrit ici : (2) reste alors tel quel.

## L'avis d'arbre principal (15 novembre 2026, 20 h)

Oui, avec deux gardes qui règlent le point 1 :

- **On ne descend que depuis g+1 exactement** : écrire g sur une ligne `hash(clé, couche, empreinte)` ne
  descend son `_depuis` à g que si elle vaut `_depuis = g+1`, avec `_jusqu_a` nul ou au-delà de g+1. Sinon
  c'est une réapparition, même en rattrapage, et elle prend le doublon de (2). Le cas : A à g, B à g+1, A à
  g+2, écrits du plus récent au plus ancien ; descendre hA de g+2 à g rendrait A visible à g+1, où vit B.
- **La ligne `_generation` de g s'écrit dans la même transaction** que les descentes et les naissances. Avant
  elle, aucune lecture ne peut être épinglée à g (le chargeur de 2b′ refuse un rang absent ; `au_rang` doit
  faire de même). Les lectures à g+1 ne voient rien changer, celles sous g ne voient pas la ligne. Reste un
  témoin : les ensembles posés sont indexés par (couche, rang) et jamais recalculés pour g+1 par la descente.
- **L'état** compte aussi les descentes **refusées par la garde g+1**, pour voir le cas A-B-A.
- **Les arêtes** gardent leur cible tant qu'elles n'ont pas de prédicat de génération à elles ; le jour où
  elles en ont, une arête née à g+1 entre deux lignes descendues descend avec elles, sous la même garde.

c7 a tranché (15 novembre, 20 h) : 2a′-3 se code sur (2) ; cette variante est un ticket à prouver, pas un préalable, et la page 03 d'arbre principal ne la porte pas.
