# Une réaction peut écrire dans la cellule qu'un appel d'outil vient de basculer

- **état** : corrigé par les pas b et c de la marche 1 bis (11 octobre 2026). Plus de bascule de cellule
  autour d'un graphe (`rechercher` prend les index et le filtre de la cellule de sa requête) ; un réacteur
  par cellule, sur le bus et les services de cette cellule ; `ReactTransitionNode` écrit dans la cellule de
  ses services, et ne transitionne qu'une cible de cette cellule. Témoin :
  `e2e_demon_hote::une_reaction_ecrit_dans_la_cellule_de_son_evenement` (rouge sans réacteur par cellule).
  Relevé par lecture le 11 octobre 2026, en relisant la page de la forme C.
- **gravité** : réponse fausse — une transition écrite dans une autre cellule que celle de sa source
- **atteignable en service** : en principe oui, depuis `0387fe401` ; rare (il faut un outil qui bascule de cellule pendant qu'une réaction s'exécute) ; jamais observé
- **touche rag3weaver** : oui, et seulement rag3weaver

## Ce que c'est

La bascule de cellule d'une recherche (`options.scope`, et le fan-out
`options.scopes`) se fait **autour** du graphe, en changeant la cellule du
catalogue entier le temps de l'appel (`Catalog::rechercher`, `catalog.rs:8086`),
et le verrou du catalogue est **relâché** entre la bascule et la recherche. La
limite y est déjà écrite : « ce n'est pas plus sûr qu'avant face à une recherche
concurrente sur le même catalogue ».

Jusqu'au 11 octobre, rien ne tournait en même temps qu'un appel d'outil dans le
même processus. Depuis le montage des réactions, le **réacteur** tourne en
parallèle des appels, et `ReactTransitionNode` lit et écrit dans la cellule
**courante** du catalogue. Une réaction qui part pendant qu'un `recall` à
`options.scopes` a basculé la cellule remonte ses ancres et écrit sa transition
dans la cellule de l'appel, pas dans celle de l'événement.

## La recette minimale

Pas de recette jouée. Il faudrait : un backend à réaction et à deux cellules,
un `recall` avec `options.scopes = [a, b]` lancé en boucle, et des écritures de
la source dans la cellule `a` pendant ce temps ; puis chercher une transition
écrite dans `b`.

## Le témoin

Aucun. Une course se prouve mal par un test unique ; le témoin le plus sûr est
structurel : une assertion que la cellule courante est celle de l'événement au
moment où `ReactTransitionNode` écrit.

## La cause

Deux décisions justes séparément : la bascule de cellule autour du graphe
(simple, et sûre tant qu'un seul appel tourne), et un réacteur qui tourne en
parallèle des appels (sans quoi une réaction attendrait la fin de chaque appel).

## Ce qu'il faut pour le fermer

Le même geste que la forme C demande de toute façon (page
`docs/11-octobre-2026-arbre-principal/01-le-demon-possede-le-catalogue.md`, §6) :

1. **sérialiser par backend** les appels **et** les exécutions du réacteur, le
   temps de l'appel entier — sûr tout de suite ;
2. ou **des cellules par requête**, sans état global — le chantier que
   `catalog.rs` nomme déjà ; et alors un événement porte sa cellule, et la
   réaction l'emploie au lieu de la cellule courante.

La 2 est la vraie correction ; la 1 suffit tant qu'un backend n'a qu'un client à
la fois, ce qui est le cas de tout ce qui existe aujourd'hui.
