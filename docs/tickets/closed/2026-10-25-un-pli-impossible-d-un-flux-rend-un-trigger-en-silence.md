# Un pli impossible d'un flux rend un Trigger en silence : une entrée Complete voit le dernier élément ou rien, selon la parité

- **État** : corrigé le 25 octobre 2026 (lot pli et ForEachNode de l'arbre principal, branche
  `arbre-principal/pli-et-pour-chaque`) : un pli impossible fait échouer le nœud en le nommant
  (`node::plier`, les deux exécuteurs et le fan-in), et `Complete ← Finite` d'un type qui ne se fond
  pas est refusé au bâti du graphe (`kinds::refuse_complete_on_finite_unfoldable`). Ouvert le même
  jour, en répondant à la mémoire longue sur son graphe `archive`.
- **Gravité** : réponse fausse, en silence. Un nœud lit une valeur qui n'est ni l'élément voulu,
  ni le flux entier, sans aucune erreur ; un nœud d'écriture branché ainsi écrit le dernier
  élément ou rien.
- **Atteignable en service** : pas à ma connaissance aujourd'hui. Les gabarits livrés ne branchent
  sur une entrée `Complete` que des flux de `Results` (qui se fondent) ou de texte. Il le devient
  dès qu'un graphe branche un flux d'objets (les sections de l'archive) sur un nœud
  d'enregistrement.
- **Touche rag3weaver** : oui (`dataflow::concurrent`, `dataflow::port::merge_port_values`).

## Ce que c'est

Une entrée `Complete` branchée sur un flux fini reçoit **le pli** de ses éléments
(`concurrent.rs`, la fusion des canaux d'un port) : chaque élément se fond au précédent par
`merge_stream_elements`, puis `merge_port_values`. Ce dernier ne connaît que quelques types
(`String`, `Results`, `Uuids`, `Children`, les statuts de branche, `SearchMeta`) ; pour tout
autre, il rend une erreur, que le pli remplace par `unwrap_or(PortValue::Trigger)`. Or un
`Trigger` s'efface devant la valeur suivante (`merge_port_values(Trigger, b) == b`). Pour N
objets JSON :

| N | ce que voit le lecteur |
|---|---|
| 1 | l'élément |
| 2 | un `Trigger` |
| 3 | le troisième élément |
| 4 | un `Trigger` |

La vérification des genres refuse `Complete ← Open` (attendre un flux sans fin), mais accepte
`Complete ← Finite` quel que soit le type.

## Le témoin

`dataflow::flux_tests::un_pli_impossible_d_un_flux_vers_une_entree_complete_se_refuse_en_le_nommant`
rouge d'abord (N = 2 accepté, un Trigger vu ; N = 3 accepté, `{"i":3}` seul vu), puis vert : un producteur `Map` est
refusé au bâti, un producteur `Any` à l'exécution, pour N = 2 et 3, en nommant le nœud et le port.

## Pour le fermer

1. À l'exécution : un pli impossible refuse le run, en nommant le nœud, le port et les deux types,
   au lieu de rendre un `Trigger`.
2. À la vérification des genres, quand le type de la sortie est connu et ne se fond pas :
   `Complete ← Finite` refusé au bâti du graphe, avec la même phrase, et le conseil de lire en
   `PerElement` ou de passer par un nœud « pour chaque ».
3. Le témoin retire son `#[ignore]` et passe au vert.
