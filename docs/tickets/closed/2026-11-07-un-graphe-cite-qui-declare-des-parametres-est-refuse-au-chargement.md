# Un graphe cité qui déclare des paramètres est refusé au chargement

- **État** : corrigé par le commit « fix(dataflow): les paramètres du parent ne se substituent plus dans le graphe cité » (lot juge-claude-code de l'arbre principal, 7 novembre 2026) : `substitute_definition` laisse intacte la valeur résolue de `graph` d'un GraphNode ou d'un ForEachNode ; témoin `dataflow::foreach_node::tests::les_parametres_du_parent_ne_se_substituent_pas_dans_le_graphe_cite`, rouge d'abord. Trouvé par la première exécution complète de `archive` (M4, session mémoire).
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

## La cause et le correctif

Le défaut n'était pas dans le chargeur : `resoudre_les_sous_graphes` lit le graphe cité tel quel et range son texte dans la configuration du nœud (`graph = {cite, mermaid, sous}`). C'est à l'appel que `substitute_definition` (`src/dataflow/graph_tool.rs`) passait les arguments du parent sur toute la configuration, ce texte compris : un `$p` du graphe cité, de même nom qu'un paramètre du parent, y était remplacé avant que `ForEachNode` ne bâtisse le graphe cité et que sa garde de bijection le cherche. Le témoin de ForEachNode ne l'avait pas vu, son graphe cité n'employant que `$element`. Le correctif (`substitute_config`) laisse la clé `graph` intacte quand sa valeur est le graphe cité résolu ; le graphe cité substitue ses propres paramètres à son instanciation.
