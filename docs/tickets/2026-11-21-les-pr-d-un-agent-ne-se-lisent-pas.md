# Les PR d'un agent ne se lisent pas

- **État** : ouvert (21 novembre 2026, 8 h 30), arbre principal.
- **Famille** : produit
- **Gravité** : un manque de lecture. L'IDE ne sait pas montrer, sous un agent, les PR qui sont les siennes. Toutes les
  PR sont ouvertes sous le même compte GitHub : rien ne dit aujourd'hui laquelle vient de quel agent.
- **Atteignable en service** : oui, dès qu'un agent ouvre une PR depuis une branche de ses vues ou de ses tâches.
- **Touche rag3weaver** : oui (`src/daemon_host.rs`, une route en lecture seule ; l'état des vues et le registre des
  tâches).
- **Pour** : arbre principal ; au mot de c7 du 21 novembre, 8 h 2x, sur une idée de Lucie : « la liste des PR owned par
  cet agent, par défaut seulement celles encore ouvertes ».
- **Signalé par** : Lucie, relayée par c7.

## Ce que c'est

- **Qui possède une PR.** Une PR est « à un agent » quand sa branche de tête est une branche de ses vues (`ouvertes.<nom>.branch`
  dans l'état gardé de sa session Claude Code) ou de ses tâches (`agent/<nom>`, dans le registre des tâches, pour une tâche
  dont la session est la sienne).
- **Ce qui existe.** L'hôte sait les branches des vues (`GET /poste`, `open_views[].branch`) et celles des tâches
  (`state().tasks`). Il n'appelle jamais GitHub.

## Le correctif attendu

- Une route en lecture seule : `GET /agents/prs?session=<id>|agent=<nom>&etat=ouvertes|toutes`. Par défaut, `ouvertes`.
- Les branches de l'agent, croisées avec `gh pr list --head <branche> --state open|all --json number,title,url,state,headRefName,updatedAt`,
  une fois par branche, bornée en nombre de branches et en temps. Sans aucune écriture GitHub : `gh pr list` seulement.
- Le compte : celui de `gh` sur le poste de l'hôte, dit dans la réponse (`gh api user --jq .login`). Jamais `gh auth switch`.
  Un `gh` absent ou non connecté est dit, comme un état, pas comme une panne.
- La réponse : `{agent, session, account, prs: [{number, title, url, state, branch, updated_at, from: "vue"|"tache", name}], unreachable?}`.
- **Le diff d'une PR ouverte** (c7, 21 novembre, 8 h 4x) : `GET /agents/prs/diff?pr=<n>`, par `gh pr diff <n>`, borné, de
  la même forme que `/vues/diff` (la stat d'abord, `[{path, added, removed, binary?, from?}]` ; `&path=<fichier>` pour le
  patch d'un seul fichier ; `truncated`). Seulement pour une PR de l'agent (sa branche de tête parmi les siennes), en
  lecture seule. recherche l'ouvre dans la même vue que les diffs non poussés : un changement poussé mais pas encore revu
  ne doit pas disparaître.
- Le témoin, rouge d'abord, dans son fichier : sur un faux `gh` (un script en tête de `PATH` qui rend un JSON fixe et journalise ses
  arguments), une session avec une vue et une tâche reçoit les PR de leurs deux branches, et pas celle d'une autre branche ;
  `etat=ouvertes` passe `--state open` ; `/agents/prs/diff?pr=<n>` d'une de ses PR rend la stat et le patch borné, et celui
  d'une PR d'une autre branche est refusé ; le journal du faux `gh` ne contient aucune commande d'écriture.

## Ordre

Après la route des diffs des vues (`GET /vues/diff`). recherche l'affiche sous l'agent.
