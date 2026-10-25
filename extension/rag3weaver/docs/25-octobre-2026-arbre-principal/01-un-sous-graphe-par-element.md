# Un sous-graphe cité, une fois ou par élément — et le pli qui se taisait

Lot de l'arbre principal, 25 octobre 2026, à la demande de l'orchestration, pour l'archive de la
mémoire longue (une écriture et une recherche de candidates par section ; page 06 du dossier
mémoire longue du 25 octobre, §3).

## Ce qui ne se déclarait pas

- **Un graphe dans un graphe.** `GraphTool::as_node_factory` faisait d'un graphe un type de nœud,
  mais seul le code s'en servait (la recherche de base, un test des réactions). Aucun manifeste ne
  pouvait citer un sous-graphe ; un outil (`tools.<nom>.graph`) ne s'appelait pas depuis un autre.
- **Une écriture par élément.** Un nœud d'écriture lit son entrée en `Complete`. Branché derrière un
  flux de N objets, il recevait leur pli : deux objets JSON ne se fondent pas, et le pli rendait
  `PortValue::Trigger` en silence, qu'une valeur suivante effaçait — le dernier élément si N est
  impair, rien s'il est pair. Le ticket : `docs/tickets/2026-10-25-un-pli-impossible-d-un-flux-rend-un-trigger-en-silence.md`.

## Ce qui se déclare maintenant

- **`GraphNode(graph='@graphs/x.mmd', …)`** : le graphe cité, appelé une fois ; son résultat sur
  `result`.
- **`ForEachNode(graph='@graphs/x.mmd', element='element', result='nœud.port', on_error='stop', parallelism=1, …)`** :
  appelé une fois par élément de `elements` (une entrée lue élément par élément ; une liste unique se
  lit élément par élément), l'élément passé au paramètre déclaré du graphe cité
  (`%% param: element json!`), les résultats en flux sur `results`, dans l'ordre des éléments, et un
  rapport `{elements, ok, errors}` sur `report`.
  - `on_error=stop` (défaut) : l'échec d'un élément fait échouer le run en le nommant, et aucun
    élément suivant n'est appelé. `on_error=report` : `{index, error, element}` passe dans le flux,
    et la suite continue.
  - `parallelism` : au plus tant d'appels en même temps, par fenêtres ; l'ordre des résultats reste
    celui des éléments. Les appels concurrents écrivent dans la même transaction du run ; ce n'est
    pas mesuré dans ce lot.
- **La citation**, comme un Plan : `@chemin` relatif au manifeste, ou `@builtin:<chemin relatif à
  templates/>`. Les autres clés de la configuration sont les paramètres du graphe cité, substitués
  comme ceux d'un outil. `$element.<champ>` atteint un champ de l'élément ; un champ absent refuse
  l'appel en le nommant.
- **Sous le run du parent** : un sous-graphe voit ses services (catalogue, modèle, embarqueur,
  cellule) et son annulation, et ses écritures entrent dans la même transaction.

## Ce que le chargement vérifie

`foreach_node::resoudre_les_sous_graphes`, appelé par le chargeur du manifeste juste après
`resoudre_les_plans`, pour chaque outil :

- le graphe cité se lit (absent : refusé en nommant l'outil et le fichier), se parse et se lie au
  registre du backend (faux : refusé de même) ;
- un `ForEachNode` dont le graphe ne déclare pas le paramètre de l'élément est refusé ;
- **jamais l'élément dans un script** : un `$element` substitué dans une clé de script est refusé —
  ce serait exécuter le texte d'un document ; un nœud scripté lit l'élément par son entrée ;
- les citations imbriquées se suivent (8 niveaux au plus) ; la configuration devient un arbre
  `{cite, mermaid, sous}` que la fabrique rematérialise à chaque création ;
- **la politique de l'outil couvre les sous-graphes** : leurs types de nœuds passent par
  `validate_tool_policy` avec ceux de l'outil. `GraphNode` et `ForEachNode` entrent dans
  `BASE_NODES` (40 entrées), sans élargir ce qu'un outil peut faire.

## Le pli qui se dit

- **À l'exécution** : `node::plier` fond les éléments d'un flux pour une entrée `Complete` et refuse,
  en nommant le nœud, l'entrée et l'élément où il casse, avec le conseil (lire en `PerElement`, ou
  `ForEachNode`). Les deux exécuteurs l'appellent, et le fan-in entre producteurs d'un même port
  refuse de même.
- **Au bâti du graphe** : `kinds::refuse_complete_on_finite_unfoldable` refuse `Complete ← Finite`
  quand le type de la sortie ne se fond pas (`Map`, les lots, une sortie de modèle…) ; `Text`,
  `Results`, `Children`, `Uuids` et `Meta` se fondent, `Any` se juge à l'exécution.

## Les témoins

- `dataflow::flux_tests::un_pli_impossible_d_un_flux_vers_une_entree_complete_se_refuse_en_le_nommant` :
  rouge d'abord (N = 2 accepté, un Trigger vu ; N = 3 accepté, `{"i":3}` seul vu), puis vert :
  `Map` refusé au bâti, `Any` à l'exécution.
- `dataflow::foreach_node::tests` : N éléments donnent N appels et N écritures, dans l'ordre ;
  l'échec au deuxième en `stop` n'appelle pas le troisième ; en `report`, l'échec se dit et la suite
  continue ; en parallèle, l'ordre est gardé ; un champ absent est refusé en le nommant ; le chargeur
  refuse un graphe absent, sans paramètre d'élément, ou à `$element` dans un script ; un manifeste
  qui cite un sous-graphe absent ou faux est refusé au chargement.
