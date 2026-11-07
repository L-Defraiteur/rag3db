# L'équipier finit en silence, et ne dit pas d'où part sa tâche

- **État** : corrigé (8 novembre 2026) par le lot « l'équipier ne finit jamais en silence » (everything declarative).
- **Famille** : produit
- **Gravité** : blocage (le superviseur attend une réponse qui n'arrivera nulle part).
- **Atteignable en service** : oui, dès qu'un modèle finit un pas par du texte sans `step_done`.
- **Touche rag3weaver** : oui (`src/daemon_host/teammate.rs`).

## Ce que c'est

Premier essai réel de c7, le 7 novembre au soir (arbre `code-essais-c7`, tâche
`journaux-annulation-tmp`, Gemini Flash, 12 itérations, 114 s) : l'équipier a lu le ticket, a
passé dix itérations à auditer ses manifestes, puis a fini son pas par une réponse en texte, sans
`step_done` ; la boucle, sans pas déclaré ni plan, est retournée attendre la boîte — **aucune ligne
dans la boîte**, ni question ni compte rendu. Et la tâche partait du HEAD des déclarations : un
`edit_file` ajouté au manifeste sans commit n'y était pas, sans que rien le dise.

## Ce qui le ferme

- Un pas qui se termine sans `step_done` écrit sa réponse finale en `report` de l'équipier
  (« le pas N s'est terminé sans step_done ; sa réponse : … »), donc dans `pending` du superviseur.
- La `request` qui ouvre la tâche écrit dans la boîte « tâche ouverte : branche … depuis <commit> ;
  ses déclarations et son code sont ceux de ce commit (une modification non commitée n'y est
  pas) », et la réponse du POST porte `task: {branche, depart}`.
- Témoin : `teammate_tests::a_step_without_step_done_reports_and_the_request_says_where_the_task_starts`,
  rouge d'abord sur master 76f19526b avec le seul témoin (« la réponse dit le commit de départ »,
  33 s avec le bâti), vert avec le correctif.

## Ce qui reste, ailleurs

L'équipier n'a pas `run_command` (le chat et l'équipier le retirent toujours) : c'est le lot 2 de
la page 07-novembre-2026-everything-declarative/01, les commandes par la porte déclarée, pas un
réglage de `code-tree add` (`commands: off`).
