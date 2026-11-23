# Un tour écrit avant le signal dense n'a pas de vecteur

- **État** : ouvert
- **Famille** : produit
- **Gravité** : faible aujourd'hui. Le rappel automatique ne cherche que dans les tours mis en jachère après sa pose :
  ceux-là sont embarqués. Un tour plus ancien reste trouvable par ses mots, pas par son sens.
- **Atteignable en service** : oui, pour une mémoire écrite quand `Turn` ne déclarait que bm25 et qui passe au gabarit
  livré (où `Turn` déclare `vector` depuis le 23 novembre 2026). C'est le cas de la mémoire longue de Lucie le jour où
  son manifeste suit le binaire.
- **Touche rag3weaver** : oui. Une entité qui gagne un signal à la réouverture ne rattrape pas les lignes déjà posées.
- **Signalé** : le 23 novembre 2026, par un témoin du lot qui donne le signal dense à `Turn`.

## Ce qu'on voit

Une base rouverte sous un gabarit dont `Turn` déclare `vector` se charge. Un tour écrit après la réouverture se trouve
par le seul signal vectoriel ; un tour écrit avant ne se trouve pas par lui. `check_proposal` le dit déjà à sa façon :
un changement d'entité demande une migration.

## Ce qu'il faudrait

Une migration déclarée, ou un rattrapage au chargement, qui embarque les lignes d'une entité qui vient de gagner le
signal vectoriel, en tâche de fond et sous le régime GPU. Le témoin
`a_memory_written_before_the_vector_signal_reopens_and_embeds_its_new_turns` passe alors à « le tour écrit avant se
trouve par le sens ».
