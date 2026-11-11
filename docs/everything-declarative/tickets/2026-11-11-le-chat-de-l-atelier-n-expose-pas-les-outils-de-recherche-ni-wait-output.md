# Le chat de l'atelier n'expose pas les outils de recherche ni wait_output

- **État** : ouvert (11 novembre 2026), relevé par archi, transmis à ED pour outils/gabarits.
- **Famille** : produit
- **Gravité** : parcours de code incomplet ; recherche structurée inaccessible depuis ce chat,
  et aucun outil déclaré pour reprendre une sortie de commande différée.
- **Atteignable en service** : oui, avec le gabarit atelier livré.
- **Touche rag3weaver** : oui, gabarit atelier et composition de ses outils.

## Preuve

Le témoin navigateur `le_navigateur_inventorie_les_outils_effectifs_atelier_depot_et_tache`,
branche `cx-archi-chat-erreurs`, code `816fc4824`, relève `GenOptions.tools` à l'arrivée au
modèle après une demande réellement envoyée depuis le formulaire. L'atelier expose lecture,
écriture, commandes et worktrees, mais ni `search_code`, `grep_files`, `list_files`, `outline`,
`usages`, `impact`, `callees`, ni `wait_output`. Dépôt et tâche proposent ces outils de recherche ;
la tâche propose aussi `run_command` et `wait_output`.

Les journaux du témoin sont sous luciepc
`~/.cache/rag3weaver-build/cx-chat-erreurs-816fc4824-20261111-inventaire/navigateur.log`.
L'inventaire est une preuve d'exposition, pas une panne de commande effectivement mesurée.

## Contrat à établir

Composer les capacités déclarées pour que l'atelier puisse explorer et chercher dans le bon
dépôt ou la bonne vue, puis suivre les sorties longues, en préservant ses accords et son
périmètre. Ne pas brancher arbitrairement la recherche sur le premier index disponible.
Le chemin et l'identité du dépôt doivent être explicites. Coordonner avec le chantier des vues.

## Pour fermer

Une demande navigateur recherche réellement un élément dans un dépôt choisi et reçoit le bon
résultat ; une commande différée se suit jusqu'à sa sortie finale ou son erreur. Les limites
et le périmètre sont visibles. Les protections et les cas de reprise ont leurs témoins.
