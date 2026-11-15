# Algareth : le nom de l'agent dans l'invite, la boîte et le journal

15 novembre 2026, soir. Lucie : « je veux qu'on puisse leur donner des noms aux agents, et ce
premier-là je voudrais qu'il s'appelle Algareth » (c7). ED a posé la clé, la persona et leurs
lecteurs (`76f1d44ed`). Les parts (2) et (3) sont à archi, la mémoire (4) à 96, l'affichage (5) à tty.

## La conception, en quatre lignes

1. **L'invite d'un pas dit le nom et la persona.** Après les règles de l'équipier, le pas ajoute :
   « Your name is <nom>. In the first answer of a task, introduce yourself by this name, once; then
   just answer. » Le nom est celui de `PreparedBackend::nom_de_l_agent()`. La persona
   (`persona_de_l_agent()`) suit, présentée comme le ton seulement, jamais contre une règle
   au-dessus ; une persona vide n'ajoute rien.
2. **La boîte porte le nom à côté du rôle.**
   - Chaque ligne écrite par l'équipier reçoit `name: "<nom>"`, et garde `from: "teammate"`.
   - Toutes les gardes de la boucle lisent le rôle (`open_question`, `missing_deliverable`,
     `request_without_a_line`, l'acheminement des textes) : un nom à la place casserait ces gardes
     en silence.
   - Le nom vient du registre (`Tache::nom_de_l_agent`), sinon de `agent.name` du manifeste de la
     tâche, sinon « équipier ». Il est posé une fois sur la boîte de la tâche (`teammate_files`), et
     `append_with` l'écrit sur les seules lignes du rôle `teammate`.
3. **Le journal du run** porte le nom de l'agent dans son en-tête d'agent, pour que le juge et l'IDE
   le lisent sans deviner.
4. **Rouge d'abord, sur le faux modèle** :
   - une tâche ouverte avec `agent_name: "Algareth"` et une persona : l'invite du pas dit « Your
     name is Algareth » et la persona ; sans persona, rien de plus ;
   - ses lignes de boîte portent `name: "Algareth"` et `from: "teammate"` ;
   - la garde de la demande (la relance) joue toujours.
