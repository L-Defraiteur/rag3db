# Le chat de l'atelier n'expose pas les outils de recherche ni wait_output

- **État** : corrigé, source ED `09e86e689` et UI `36865616d`, preuves finales ci-dessous ; clôture dans le lot intégré.
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
ouvert dans `docs/everything-declarative/tickets/2026-11-11-un-accord-de-commande-hors-domaine-redemande-le-meme-accord.md` ;
il n'est pas masqué par le parcours nominal de commande dans sa racine.


### Validation archi terminée sur a92 (intégration encore à faire)

Navigateur complet sous jeu large : **39/0/0 en 137,71 s** ; attestation ELF du binaire testé
`4ad3a31b…70b7f3e` : table de son `.d`, 220 chemins actuels et 220 contenus présents, aucun absent.
Preuves sur les deux postes : `cx-ui-contextuel-a92b21233-navigateur-entier-20261111/` dans le
cache de build. Reste verdict ED sur a92, puis combinaison avec le lot recherche 4.3.3 après
sa fenêtre, diff et rejeux requis, clôture documentaire et intégration autorisée/nettoyée.
Le ticket reste ouvert ; aucune preuve de la future combinaison n'est anticipée.


### Rejeu archi après recherche 4.3.3 : source 458

Sur `458e50b46a277618f08bb0f37d5bcbebe1250db7`, trois ciblés réels sous jeu large **3/0/0 en
135,93 s**. Attestation ELF `701a37d1…e1cc91f` : 220 chemins et contenus actuels, zéro absent,
JS/manifeste inchangés. Preuves sauvegardées aux deux postes dans
`cx-ui-contextuel-458e50b46-cibles-final-20261111/`. Pas de répétition des 39 sur cette source :
leur résultat a92 reste distinct. ED tient les autres contrôles du même SHA.

L'intégration attend aussi le départage du ticket de revue ED
`2026-11-11-une-invocation-peut-executer-une-autre-version-que-celle-verifiee.md`, avant tout
« à toi ». La passe 458 n'a pas été arrêtée ni modifiée pour cette revue. Ce ticket commun
reste ouvert et le target archi conservé pour le rejeu qu'un correctif justifierait.

## Clôture du lot commun

ED runtime/intégration et archi UI : verdict agrégé VERT relu et tour « à toi »
donné par orchestration le 11 novembre. ED sur `09e86e689` : features 5/5,
gardes 10/10 (dont le rouge949 puis vert074 du remplacement de déclarations),
lib entière 2114 réussites / zéro échec / 64 ignorés en 42,84 s, lectures24/0/0
en10,24s, PostgreSQL privé34/0/0 en7,67s ; 2/2 suites, aucun SAUTÉ.
UI sur `36865616d` : trois parcours réels3/0/0 en132,16s et attestation ELF
`6782d8a52d41986af857b99ba85d051dd79557dc97f67419b782caba9673a607`,
220 chemins et contenus courants présents. Entre 368 et09, seul `#[cfg(test)]`
est ajouté au raccourci privé call_tool : réutilisation UI relue et acceptée,
sans attribuer ces tests à une autre source. Les39 navigateurs a92 restent distincts.

Preuves sauvegardées aux deux postes : `I-luciepc/relais-20261111/`
(`verdict-final`, `verdict-final.json`, `*-commun-5.log`,
`version-rouge-949dcb556.log`, `version-vert.log`) et
`cx-ui-contextuel-36865616d-cibles-final-20261111/` (log et attestation).
L'intégration utilise le garde VERT, poste push, la relecture de master puis les
trois contrôles ; le script supprime les seuls worktrees/branches locales/target
et base PostgreSQL privée du relais, en préservant les archives et branches distantes.

Restent ouverts et distincts : accord de commande hors domaine, erreurs SQL de
dette et remaining/juge. Aucun succès sur ces autres tickets n'est annoncé ici.
