# Les textes d'invite encore écrits dans le code

- **État** : ouvert, le 23 novembre 2026 ; pour le lot qui suit `prompt-templates`, avant la jachère
- **Famille** : produit
- **Gravité** : moyenne : chaque mot coûte un bâti, une passe et une pose ; ce sont ces textes qui ont fait parler le modèle de l'utilisatrice à la troisième personne
- **Atteignable en service** : oui
- **Touche rag3weaver** : oui (`src/daemon_host/teammate.rs`, `src/daemon_host/chat.rs`, `src/daemon_host/observed.rs`, `src/backend.rs`, `src/llm.rs`)
- **Pour** : archi
- **Signalé par** : la règle de Lucie (tout texte qui atterrit chez le modèle vient d'un gabarit à variables), relevé le 23 novembre 2026

## Ce que `prompt-templates` a déjà sorti du code

`templates/teammate/prompts/` : `system.md` (l'invite de l'équipier), `reply_first.md` (la consigne de tête),
`text_first.md` (le refus d'un `step_done` sans texte), `sign_nothing.md`, surchargeables par
`<config>/templates/teammate/prompts/<nom>.md`, relus à chaque pas.

## Ce qui reste dans le code

- **Les relances de l'équipier** (`teammate.rs`, `relance = Some(…)`) : le pas fini sans `step_done` ; le tour où l'agent
  n'a rien écrit ; la demande sans réponse dans la boîte ; le texte demandé manquant (« your reports describe the
  work ») ; la porte de vérification rouge.
- **Le message de chaque pas** (`frontier_message`, une quinzaine de phrases : la boîte, les plans, « Réponds ou agis »,
  « Il ne reste rien de faisable », la demande nouvelle pendant un plan actif…), et les ajouts du pas (le livrable
  exigé, « ton arbre n'est pas son dossier »).
- **La règle du contenu observé** (`observed.rs`, `observed_content_rule!`), collée à l'invite système.
- **L'invite du projet** (`chat.rs`, `project_first` : « The person's project is the folder… »).
- **La notice des dépenses du mois** (`chat.rs`, « The month's spending of paid models… »).
- **La relance d'une réponse coupée** (`backend.rs`, `cut_relaunch_text_default`) et la phrase de la coupe
  (`llm.rs`, `cut_reply`).
- **Le résultat d'un appel interrompu** (`llm.rs`, `INTERRUPTED_TOOL_RESULT`).

## À faire

Chacun devient un fichier de `templates/teammate/prompts/` (ou d'un dossier voisin pour le chat), avec ses
variables (`{$step}`, `{$user_folder}`, `{$tool.step_done}`…), lu par `user_prompt` au moment de servir ; un témoin
par famille : le fichier surchargé change le texte envoyé au tour suivant.
