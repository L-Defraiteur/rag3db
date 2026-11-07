# Un graphe cité qui déclare des paramètres est refusé au chargement

- **État** : ouvert, le 7 novembre 2026. Trouvé par la première exécution complète de l'outil
  `archive` (M4, session mémoire) ; question posée à l'arbre principal, qui tient ForEachNode et le
  GraphNode cité (1bd1be619).
- **Gravité** : blocage. Un graphe cité ne peut recevoir que son élément : ses autres paramètres
  font refuser le manifeste entier.
- **Atteignable en service** : oui, dès qu'un gabarit cite un graphe à paramètres.
- **Touche rag3weaver** : oui (`src/dataflow/graph_tool.rs`, le chargeur des sous-graphes).
- **Famille** : produit

## Ce que c'est

Un `ForEachNode(graph='@graphs/x.mmd', p=$p)` dont le graphe cité déclare `%% param: element json!`
et `%% param: p int = 5`, et l'emploie (`RhaiNode(…, p=$p)`), est refusé à la construction :
« fiche d'outil invalide : le paramètre 'p' n'apparaît nulle part dans le graphe ($p) »
(`graph_tool.rs:1157`, la garde de bijection paramètres ↔ graphe).

## La recette minimale

Le parent :

```
%% tool: parent
%% description: d
%% param: p int = 5
%% result: chaque.results
graph LR
    liste["RhaiNode(script='[#{a: 1}, #{a: 2}]')"]
    chaque["ForEachNode(graph='@graphs/cite.mmd', p=$p)"]
    liste -->|result:elements| chaque
```

Le graphe cité, `graphs/cite.mmd` :

```
%% tool: cite
%% description: d
%% param: element json!
%% param: p int = 5
%% result: r.result
graph LR
    r["RhaiNode(script_id=s, inputs='e,p', e=$element, p=$p)"]
```

## La cause, lue et non prouvée

Les paramètres donnés par le parent semblent substitués dans le texte du graphe cité **avant** qu'il
soit validé comme fiche d'outil : quand la garde de bijection cherche `$p`, il a déjà disparu. Le
témoin de ForEachNode passe parce que son graphe cité ne déclare que `element`.

## Le contournement

L'outil `archive` fait porter ses réglages par chaque élément (`$element.candidates`,
`$element.instruction`…) et son graphe cité ne déclare que `element` (lot de la session mémoire,
page 06 du 25 octobre §3). Il reste une inconnue : le même chemin vaut-il pour un `GraphNode` cité
avec des paramètres (archive appelle `search_structured.mmd` avec `target`, `query`, `options`) ?

## Pour le fermer

La garde de bijection juge le graphe cité avant toute substitution (ou ne compte pas comme absents
les paramètres que le parent fournit) ; témoin : la recette ci-dessus se charge et rend `p` dans
chaque appel.
