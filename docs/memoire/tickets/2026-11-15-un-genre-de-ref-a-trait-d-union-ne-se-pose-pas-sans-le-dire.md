# Un genre de Ref à trait d'union ne se pose pas, sans le dire

- **État** : ouvert, le 15 novembre 2026. Contourné : le genre s'appelle `derived_from`.
- **Famille** : produit
- **Gravité** : perte silencieuse. Une Ref attendue n'est pas posée, et l'outil rend « ok ».
- **Atteignable en service** : oui. Tout `create_ref_type` ou `ancrer_fiche` sur un nom de genre à trait d'union.
- **Touche rag3weaver** : oui, `CreateRefTypeNode` et `AddRefNode` (`src/dataflow/ref_nodes.rs`).
- **Pour** : session mémoire (96).
- **Signalé par** : session mémoire (96), en écrivant `scinder_fiche`, le 15 novembre 2026.

## Ce qu'on voyait

`scinder_fiche` déclarait un genre `issue-de` (`CreateRefTypeNode(name='issue-de', confirm=true)`) puis ancrait chaque fiche neuve à une Ref de ce genre. Le geste rendait « ok », mais `cited_by` ne trouvait rien. Le même geste avec le genre `derived_from` pose la Ref. Les genres existants (`agent`, `alias`, `doc`, `handle`, `same_as`) sont tous en minuscules à soulignés.

## Ce qui reste à savoir

Lequel des deux nœuds refuse le nom, et pourquoi ce refus n'arrête pas le geste : le report de `CreateRefTypeNode` ou d'`AddRefNode` n'est lu par personne dans le graphe. Le témoin rouge d'abord : `create_ref_type` sur `issue-de` doit soit poser le genre, soit refuser en le disant.
