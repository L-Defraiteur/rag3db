# La pause n'adresse pas son interlocuteur par l'annuaire

- **État** : ouvert
- **Gravité** : réponse fausse (le pair n'est pas prévenu)
- **Atteignable en service** : oui, dès qu'un agent nommé autrement que son run se met en pause envers un pair
- **Touche rag3weaver** : oui (`src/agent.rs`, `handle_pause`)

## Ce que c'est

Depuis le lot 3 des agents qui se parlent, `send` adresse un participant par son **nom**
grâce à l'annuaire de la session (`src/directory.rs`). `pause_dialogue` et
`confirm_pause` prennent encore leur `avec` comme un **run** : `handle_pause` écrit
`bus.send_message(run_id, nom, avec, …)`, et `avec` sert de run destinataire. Un
agent qui se met en pause « avec bob » écrit donc dans la boîte du run « bob », qui
n'existe pas. Bob n'est pas prévenu.

## Recette

Deux agents inscrits à l'annuaire sous des noms distincts de leurs runs
(`alice` → `run-a`, `bob` → `run-b`). Le modèle d'alice appelle
`pause_dialogue(avec: "bob", genre: "finished", raison: "…")`. Aucun message
n'arrive sur `run.run-b.inbox`.

## Témoin

Aucun pour l'instant. Il s'écrit dans `src/directory/tests.rs`, sur la forme
de `an_agent_answers_another_by_send` : alice se met en pause avec « bob », et la
boîte de `run-b` reçoit l'avis.

## Cause

`handle_pause` est antérieur à l'annuaire. Les tests de la pause posent nom = run,
ce qui masquait la différence.

## Pour le fermer

Quand l'agent a un annuaire, `handle_pause` résout `avec` par
`Directory::resolve`, qui accepte un nom ou un run, et écrit dans la boîte du run
résolu. Un `avec` inconnu est dit dans le résultat d'outil, comme pour `send`.
Sans annuaire, le comportement d'aujourd'hui reste.
