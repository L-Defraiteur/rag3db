# Le juge de session — un agent qui relit un tour, et sa fiche de bilan

Demandé par Lucie le 25 octobre 2026, relayé par c7 : « un genre d'agent de benchmark qualitatif,
pas juste un appel structuré ». Le juge vient d'abord ; le banc (des tâches de référence
rejouées par le juge à chaque lot important) vient après, sur lui.

## Ce qui existe, et ce qui manque

- **Le run** : les événements `agent` d'un tour (RunStarted, LlmCall, ToolCallStarted avec ses
  arguments, ToolCallFinished, Consumed, RunFinished) passent sur le bus et ne sont gardés nulle
  part, hormis le tampon de 2 048 événements du flux de l'IDE. `ToolCallFinished` ne porte pas le
  texte du résultat. La session du chat (`chats_root/<id>.json`) garde les tours, avec le texte des
  résultats (borné à 32 Kio), mais sans les rattacher à un run.
- **Les réactions** ne peuvent pas porter d'agent : leur liste blanche de nœuds ne le permet pas.
- **Un agent déclaré** n'a pas de forme à lui : on a les `chat.json` des apps (`ChatConfig`). La
  sortie structurée existe (`GenOptions.response_format`), mais ni `ChatConfig` ni
  `ChatSession::run_with` ne la passent.
- **La mémoire** a des fiches génériques (Fiche = sujet, Apport = L1, Source = L0). Le L2 (`Etat`)
  est décrit, avec son schéma, mais rien ne l'écrit : c'est l'archiviste, à venir chez la mémoire.
- **Les propositions** valident sans écrire, mais seulement des fichiers du dossier du backend. La
  description d'un outil se surcharge par la clé `description` de son entrée dans `backend.json`.

## Ce que le lot pose

1. **Le journal des runs de l'hôte** (c7 : générique, pas propre au chat). À la fin de **tout**
   run (un appel d'outil, une réaction, un tour de chat), l'hôte écrit
   `runs_root/<backend>/<run>.json`. Il y met :
   - les événements du bus pour ce run ;
   - les résultats tels que le modèle ou l'appelant les a reçus (les tours d'un chat, la sortie
     d'un appel) ;
   - la tâche et les propositions.

   Le juge le lit, et le DAG vivant pourra rejouer un run passé. Le journal est borné en taille
   par run et en nombre par backend, par un réglage ; les plus vieux sont effacés, et l'hôte le
   dit.
2. **Le juge, un agent déclaré** : `templates/agents/juge/juge.json`, un `ChatConfig` (son message
   système, son modèle, ses outils permis), plus `bilan.schema.json`, le schéma de sa sortie
   finale. `ChatConfig` gagne `response_schema`, que `run_with` pose en `response_format` sur le
   dernier appel. L'appel structuré est la dernière étape du juge, pas le juge.
3. **Ses outils**, servis par l'hôte et déclarés comme ses outils :
   - `lire_le_run` : le journal du tour (filtrable par outil) ;
   - `rejouer_un_outil` : rejoue un appel du tour avec les mêmes arguments, sur le même backend,
     **seulement si l'outil ne fait que lire** (sans `write_files` ni `run_commands`), pour voir ce
     que le modèle a reçu ;
   - `ouvrir_le_resultat` : la relecture de la tâche (fichiers, diff), une proposition, un fichier
     au moment du tour ;
   - `bilans_precedents` : les apports antérieurs de la fiche de la même tâche (`recall_fiches`,
     puis `cited_by` sur la fiche) ;
   - `ecrire_le_bilan` : la fiche (créée si besoin), un L0 (le journal), un L1 (le bilan rendu en
     texte, depuis la sortie structurée) ;
   - `proposer_un_ticket` : une proposition sur la clé `description` d'un outil du backend jugé (le
     texte exact du retour ambigu, et ce que le modèle a cru), appliquée ou rejetée comme les
     autres propositions.
4. **Le déclenchement**, pour ce soir (c7). Ce n'est pas une réaction, puisqu'elles ne portent
   pas d'agent : c'est l'hôte qui lance le juge, en fond, à la fin d'un tour, quand le manifeste du
   backend jugé dit `"bilans": { "memoire": "<manifeste de la mémoire>" }`. On peut aussi le
   lancer à la demande, par `POST /ide/bilans {backend, run}`. Un juge par tour, et **jamais sur
   ses propres runs**.

   **La voie générale, à trancher par Lucie** : ouvrir aux agents la liste blanche des nœuds d'une
   réaction. Une réaction déclarée sur `RunFinished` porterait alors le juge, comme tout autre
   agent, sans code dans l'hôte.
5. **La fiche de bilan**, dans la mémoire, sous la forme relue par 96 :
   - **Une Fiche ordinaire** : `via: "juge"`, `author` vide (l'hôte le posera, lot M7 de la
     mémoire). Elle a une ancre (`ancrer_fiche`) par outil ou gabarit jugé, genre `outil` ou
     `gabarit` (créé une fois par `create_ref_type`), valeur `outil:<nom>` ou `gabarit:<nom>`.
     `cited_by(value: "outil:read_file")` rend alors tous les bilans de cet outil.
   - **Son nom est stable**, puisque l'identité d'une Fiche est `name`, au caractère près. Une
     tâche du banc a le sien, déclaré. Un tour ordinaire prend `bilan : ` suivi de l'intitulé du
     premier message, passé par une seule fonction, `nom_de_tache` : minuscules, espaces repliés,
     ponctuation de bord retirée. Le message entier va dans `about` et dans le bilan.
   - **La Source (L0)** est le journal du run, écrit à son chemin `sources/<date>-<uuid>.md`.
     Elle est immuable.
   - **Un Apport (L1) par session**, section `bilan`. Son texte est le bilan lisible, tel que Lucie
     le lit dans `fiches/<uuid>.md`, suivi d'un bloc ```json du schéma, qui se relit par programme
     et se cherche en plein texte.

   Les rubriques du bilan suivent la demande de c7 :
   - note et confiance ;
   - objectif atteint, partiel ou non, chaque fait *dit* ou *déduit* ;
   - par outil : appels, échecs, retours ambigus, appels inutiles ;
   - frictions et attentes ;
   - coût par slug, cache compris ;
   - ce qui manquait ;
   - à corriger ;
   - **la comparaison avec les bilans précédents** : amélioration, régression ou pareil, et sur
     quoi.

   Le L2 attend l'archiviste : le juge ne l'écrit pas.
6. **La vue « bilans »** dans l'IDE, par fiche : ses apports dans l'ordre, la tendance de la note,
   les tickets proposés.

## Les témoins

- **La tuyauterie, sur faux modèle.**
  - Un tour du chat laisse son journal.
  - Le faux juge lit le run, rejoue un outil de lecture (un outil qui écrit est refusé), ouvre la
    tâche, écrit la fiche (fiche, L0, L1) et propose un ticket.
  - Un second tour de la même tâche retrouve le premier apport.
- **Gemini, jugé sur le résultat.**
  - Une session porte un retour ambigu volontaire (la description d'un outil de banc qui ment) :
    le bilan doit le nommer et proposer un ticket sur cet outil.
  - Une seconde session de la même tâche, corrigée, doit être comparée à la première (« amélioration
    », avec sur quoi).
  - Comme K3 : #[ignore], exigé, avec sa mesure.

## Ce qui est tranché, et ce qui reste

- Tranché par c7 :
  - le journal est générique, celui des runs de l'hôte, borné ;
  - ce soir, le juge est lancé par l'hôte (`bilans`, ou à la demande) ;
  - le L2 revient à l'archiviste ; le juge écrit la fiche et les L1.
- Relu par la mémoire (96) : la forme de la fiche, reportée au §5.
- À trancher par Lucie : ouvrir les réactions aux agents (la voie générale du déclenchement).
