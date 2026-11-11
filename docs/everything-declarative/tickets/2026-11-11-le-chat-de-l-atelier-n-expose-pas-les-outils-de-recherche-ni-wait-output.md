# Le chat de l'atelier n'expose pas les outils de recherche ni wait_output

- **État** : ouvert (11 novembre 2026), relevé par archi, transmis à ED pour outils/gabarits.
- **Pour** : ED (primitive, outils, politiques et intégration) ; archi (rendu contextuel, preuves navigateur).
- **Signalé par** : archi, inventaire navigateur `816fc4824` décrit ci-dessous.
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


## Lot commun en validation — 11 novembre

Ce même ticket porte le relais, son exposition réelle et l'intégration UI : pas de ticket
supplémentaire pour chaque étape de cette capacité. Source commune figée
`a92b212334d240fe10bb7a295f12185864e048df`, branche
`cx-archi-ui-contextuel-relais-3`, sur base ED `3a0679b7f` et master `418e47a2e`.

Preuves archi : trois ciblés navigateur **3 réussis, 0 échec, 0 ignoré en 143,02 s** :
`daemon_host::chat_contextuel` couvre recherche et outline réels dans deux dépôts aux mêmes
chemin/symbole mais contenus distincts, clic et rechargement, puis la même preuve avec un
résultat réellement coupé avant `Metadata:` ; le reçu structuré conserve la cible. Le dernier
cas lance une commande après accord dans la page, appelle réellement wait_output sur son
journal encore vide, puis libère sa sortie après l'annonce de l'attente dans le flux.
Journal sur les deux postes :
`~/.cache/rag3weaver-build/cx-ui-contextuel-a92b21233-cibles-20261111/navigateur.log`.
Les rouges diagnostiques précédents restent séparés. Les liens utilisent le reçu structuré,
jamais le texte ; une provenance inconnue les rend inactifs. TypeScript, garde JS de contexte,
bundle identique aux sources et garde de tickets verts sur le code commun.

Reste avant clôture : ED tient features, lib entière, gardes de cible inconnue/politiques,
lectures et PostgreSQL privé ; archi tient navigateur complet (39 témoins) et attestation du
bundle embarqué dans l'ELF testé, sur ce même SHA. Relire les verdicts, mettre à jour les docs,
obtenir le tour d'intégration puis contrôler le push et nettoyer les seuls artefacts du lot.
Ne pas fermer sur les seuls ciblés ; aucune livraison master n'est encore annoncée.

L'accord de commande hors domaine qui redemande l'accord est un travail **distinct**, conservé
ouvert dans `docs/rag3weaver-archi/tickets/2026-11-11-un-accord-de-commande-hors-domaine-redemande-le-meme-accord.md` ;
il n'est pas masqué par le parcours nominal de commande dans sa racine.
