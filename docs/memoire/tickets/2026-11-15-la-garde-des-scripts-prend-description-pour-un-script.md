# La garde des scripts prend la clé « description » pour un script

- **État** : ouvert, le 15 novembre 2026. Contourné dans `gestes_creer.mmd` (les genres déclarés en dur), pas corrigé.
- **Famille** : produit
- **Gravité** : refus à tort. Un gabarit juste est refusé au chargement.
- **Atteignable en service** : non. Tout graphe cité qui substitue `$element` dans une clé contenant « script » (`description`, `transcript`…) le rencontre.
- **Touche rag3weaver** : oui, `src/dataflow/foreach_node.rs`, `script_qui_lit` (`k.contains("script")`).
- **Pour** : session mémoire (96), ou la session qui tient le dataflow.
- **Signalé par** : session mémoire (96), en écrivant `scinder_fiche`, le 15 novembre 2026.

## Ce qu'on voyait

« le nœud « genre » substitue $groupe dans sa clé « description » — un script lit l'élément par son entrée, jamais par substitution ». La garde est juste dans son principe : un texte de document substitué dans du code serait exécuté. Mais elle reconnaît une clé de script par `k.contains("script")`, et « description » contient « script ».

## Ce qui reste à faire

Reconnaître les clés de script par leur nom exact (`script`, `script_id`), ou par la liste des clés que le nœud déclare comme du code. Le témoin rouge d'abord : un `CreateRefTypeNode(description=$element.x)` dans un graphe cité doit se charger.
