# Le branchement du socle dans toute tâche : la conception

15 novembre 2026, soir. C'est le lot qui suit la pile de la garde de l'équipier (B) et du
[côté pas](08-conception-le-pas-du-socle.md) ; il lit les primitives d'ED (branche
`socle-agent-de-code`, `8ef56dd5e`). La référence est la
[vision du 15 novembre](../../orchestration/visions/15-11-2026-l-agent-est-un-agent-de-code-qui-gere-ses-worktrees.md),
et l'inventaire est la [page 07](07-le-socle-d-un-agent-de-code-dans-une-tache.md).

## La conception, en quatre lignes

1. **Le socle est un jeu d'attachements livré, greffé au chargement de toute tâche.** Ce sont les
   neuf outils que le gabarit atelier déclare : `read_file`, `replace_text`, `replace_lines`,
   `insert_lines`, `write`, `glob`, `grep_files`, `list_files` et `run_command`, chacun avec son
   graphe et sa politique.
   - Quand un backend est une tâche (sa surcharge) et a un workspace, `avec_surcharges` ajoute
     ceux qui lui manquent.
   - Sous le même nom, l'outil du backend gagne : `edit_file` reste celui d'un backend de code, et
     son index se met à jour au retour.
   - Chaque outil greffé se valide comme les autres (`validate_tool_policy`), sous la politique de
     la tâche : `read_only = !ecrit`, relu à l'ouverture depuis le côté pas.
2. **La porte des écritures et l'accord dans le tour.**
   - Tout outil proposé dans une tâche dont la politique écrit passe par la même porte que
     `run_command`, réglable par `workspace.commands`, avec `auto` par défaut dans le gabarit de
     code.
   - Une action qui demande ne rend plus « en attente » au modèle. L'appel attend la réponse dans
     la boîte (la ligne `approval`, o/n dans tty), au plus un délai réglé, puis continue dans le
     même tour, accordée ou refusée en le disant.
3. **Le décideur modèle de Lucie** (page 07, point 10) est le sous-lot suivant, sur cette porte. Il
   reçoit la demande, le plan et l'action ; ce que la demande couvre passe ; dans le doute, « passe »
   tant que Landlock tient. Le banc le mesure sur le taux de demandes inutiles.
4. **Rouge d'abord, de bout en bout, avec ED** : un manifeste sans aucun outil d'écriture (la forme
   de celui de Lucie). Une tâche y lit (avec l'empreinte), édite par `replace_text`, et le diff est
   au run. Un `replace_lines` sur une empreinte périmée est refusé sans rien écrire ; un `write` qui
   écraserait est refusé. Sous `approval`, l'écriture attend le « o » dans le même tour.

## La question ouverte avant le code

Les nœuds de l'atelier (`LireFichierNode`, `EcrireFichierNode`, `WorktreeHost`) demandent-ils des
services que seul le backend atelier monte (ses worktrees attachés) ? Si oui, le greffage monte
aussi ces services pour une tâche, sur sa seule racine, ou passe par le repli `FileTools` étendu
aux mêmes primitives. Je le lis dans le code d'ED avant d'écrire le premier rouge.
