# L'équipier — un agent sur Gemini qui travaille seul, par pas, et parle à un superviseur

Décidé par Lucie et c7 le 25 octobre 2026 au soir. C'est le passage à l'usage réel du produit
code : un agent qui accomplit une tâche en autonomie et échange avec un superviseur (c7, puis
Lucie), comme le font les sessions Claude. Le juge (pages 04 et 05) en est l'instrument de
fiabilité. Ce lot est équipier-1.

## Ce qui existe déjà, et sur quoi il se bâtit

- **La tâche** (`ouvrir_une_tache`, `taches.rs`) : une branche `agent/<nom>`, un worktree, une
  copie de la base, un second backend hébergé. Sa relecture, sa fusion et son rejet restent à
  Lucie ou à une session Claude.
- **Le tour de chat dans une tâche** (`chat_turn` sur l'id de la tâche, ae127d634) :
  - la conversation est gardée et continuée ;
  - les outils sont ceux du chat qui code : lire, chercher, éditer dans la tâche, jamais de
    commande ;
  - chaque tour est un run du journal, et un bilan du juge si le backend déclare `bilans`.
- **L'arrêt** (C4, `stop_run`) : un drapeau que la boucle lit au token suivant et avant chaque
  outil.

## La boucle

1. **La demande** arrive par la boîte aux lettres. L'hôte ouvre la tâche si elle n'existe pas,
   puis lance le premier pas.
2. **Le mini-plan.** Le premier pas écrit le mini-plan par `edit_plan` ; l'agent le
   réédite ensuite librement (plus bas). Il contient la
   demande telle que l'agent la comprend, ce qu'il propose, puis des éléments. Chaque élément
   porte :
   - un statut : `todo`, `doing`, `done`, `blocked`, `dropped` ;
   - `alone` (faisable sans validation), ou bien `needs_validation` avec la question posée.

   Chaque élément à valider part dans la boîte à l'édition qui le pose : les bloquants
   d'abord. L'agent attaque ensuite le premier élément faisable seul, sans attendre la réponse.
3. **Le pas.** L'agent travaille un élément, puis appelle
   `step_done { item, succeeded, done, remaining }`. L'appel finit le pas :
   - réussi : l'élément passe à `done` ;
   - raté : l'élément garde son statut, avec sa raison ;
   - dans les deux cas, une ligne part dans la boîte.

   Un pas est un tour de chat, donc un run du journal et un bilan du juge.
4. **Le budget du pas.** Au-delà de douze appels d'outil sans `step_done`, l'hôte refuse tout
   autre appel par le même message : « déclare d'abord step_done (réussi ou non, ce qui est fait,
   ce qui reste) ». Le refus est une réponse d'outil : l'agent le lit et s'y plie.
5. **La frontière.** Entre deux pas, l'hôte injecte dans la conversation, comme un message, ce
   qui est arrivé dans la boîte pendant le pas (réponses aux bloquants, consignes nouvelles) et le
   mini-plan courant. La conversation est la même pour ce lot. La compaction par les fiches est
   pour équipier-2.
6. **La fin se juge après le pas, sur les plans.** L'équipier s'arrête quand il n'y a plus
   aucun plan vivant (ni actif, ni en pause) et que le pas a été déclaré fini : « plus rien à
   faire : compte rendu » s'il y a eu un plan, « fait sans plan » sinon. Un plan actif qui n'a
   plus rien à faire n'est pas la fin : la frontière dit de le clore, puis de reprendre un plan
   en pause ou de le clore aussi. Avec seulement des éléments qui attendent le superviseur, il
   attend la boîte ; une ligne qui arrive le relance. La fin s'écrit dans la boîte (`ended`,
   de `host`), et l'équipier garde dans `mailbox.read` combien de lignes il a lues : relancé
   plus tard, il ne relit pas les anciennes.

## Les messages

Il y a deux voies :
- **L'arrêt** passe tout de suite : `stop_run` sur le run du pas en cours, et l'équipier
  s'arrête là, sans pas suivant.
- **Tout le reste** attend la frontière. Un agent seul finit ce qu'il a commencé avant
  d'écouter.

## La boîte aux lettres

- **Un fichier par tâche**, `<dossier de la tâche>/mailbox.jsonl`, lisible sans IDE. Une ligne
  par message : `{at, from, kind, item?, text}`. Les genres (`kind`) sont `request`, `instruction`,
  `answer` (à un bloquant ou à une proposition), `question` (un bloquant posé), `proposal` (le
  hors-plan), `step_done`, `report`, `stop` et `ended` (la fin, écrite par l'hôte).
- **Un sujet du bus** par tâche, `mailbox.<task>`, où chaque ligne est aussi publiée.
- **Le superviseur écrit** par `POST /ide/teammate/mailbox {backend, task, kind, item?,
  text}`. La route ajoute la ligne au fichier et la publie. Un réacteur de l'hôte écoute le
  sujet :
  - une `request` à une tâche absente l'ouvre ;
  - un message à un équipier qui attend le relance.

  Une ligne ajoutée au fichier à la main est lue à la frontière suivante, mais ne réveille pas
  un équipier qui attend (pas de surveillance de fichier dans ce lot).
- **L'agent écrit** par `report` et `propose`, et `step_done` y ajoute sa ligne.
- **L'IDE** lit `GET /ide/teammate?backend=&task=` : la boîte, les plans vivants, les plans
  clos, et si l'équipier tourne.

Le canal des sessions Claude n'est pas utilisé.

## Le mini-plan : libre, puis gardé en mémoire (Lucie)

**Autonomie maximale** (Lucie). L'agent clôt seul ses éléments et son plan (`accomplished`) ; il
ne demande rien pour ce qui est dans le plan. Une `question` ne sert qu'à un vrai bloquant. Le
**hors-plan** — une direction que la demande n'a pas prévue — part dans la boîte comme une
`proposal` (« voudrais-tu qu'on pousse dans ce sens ? », avec l'élément qu'elle ouvrirait), et
l'agent continue le plan courant sans attendre ; il n'ouvre pas cette direction sans le mot du
superviseur. La réponse arrive à une frontière, comme pour un bloquant.

**Pas de harnais sur le plan.** L'agent édite ses éléments quand il veut, comme il le sent. Il
peut juger son plan accompli ou obsolète, le clore avec sa raison et en ouvrir un autre. Les
seules contraintes dures sont le budget du pas (douze appels sans `step_done`) et l'arrêt (C4).

**Le plan courant** vit dans `<dossier de la tâche>/plan.json`, et seulement là (96). L'hôte le
lit à chaque frontière (reste-t-il un élément faisable ou bloqué ?) et l'injecte. On ne le
double pas en base :
- un champ de Fiche changerait le schéma du gabarit `memory` ;
- un L1 réécrit à chaque édition casserait l'histoire.

L'état vivant d'une fiche, dans ce modèle, c'est son L2, recalculé depuis les L1 (M5).

**Les plans clos vont en mémoire, sur la fiche de session** (la vision du 4 octobre : le
contexte comme une vue des fiches ; forme relue par 96).
- La fiche de session est celle que le juge range déjà (page 04, nom stable par
  `nom_de_tache`), dans la mémoire déclarée (`bilans.memoire`). Pour un équipier, son nom vient
  de la demande, pas du premier message d'un pas : chaque pas commence par l'injection de la
  frontière, et le juge doit ranger tous les pas sur la même fiche.
- **Un L1 par plan clos, section exactement `plan`.** Le numéro va dans la source,
  `sources/<jour>-<tâche>-plan-<n>.md`, dont le L0 est le plan rendu lisible. Le texte donne la
  demande, la proposition, les éléments par statut final et la raison de la clôture, puis le
  bloc ```json entier.
- **Ses références sont sur l'apport, par `citer_source`** (pas des ancres de la fiche : ce
  qu'un plan a touché est son histoire) :
  - `path:<chemin relatif au dépôt>` pour un fichier, et `symbol` (`chemin#Nom`) pour un
    symbole ;
  - `outil:<nom>`, la valeur de l'ancre des fiches d'outils ;
  - `doc:sources/<jour>-<run>.md#bilan|#defauts`, la même Ref que le juge a posée pour les
    apports de ses pas.

  `cited_by` rend ainsi le plan depuis un fichier, un outil ou le journal d'un run.
- **Les concepts ne sont ni un champ ni une Ref** : un concept est une fiche, et c'est
  l'archiviste qui classe (M4). Une intuition de l'agent va en texte dans le bloc json
  (`concepts`), comme indication.
- Relancé, l'agent lit le plan courant et peut remonter aux plans clos. Le juge, qui range son
  bilan sur la même fiche, voit le plan à côté du run.

## Ses outils

- Ceux du chat qui code dans une tâche, rien de plus. Les tests, la fusion et le push restent à
  une session Claude ou au superviseur.
- `edit_plan` :
  - poser le plan entier ;
  - changer, ajouter ou retirer un élément (statut, raison, question) ;
  - ou clore ce plan (`accomplished` ou `obsolete`, avec sa raison) et en ouvrir un autre.
- `step_done { item, succeeded, done, remaining }` : il finit le pas. La boucle de l'agent apprend
  un **outil qui termine** : un nom déclaré terminal arrête la boucle après sa réponse
  (`StopReason::Terminal`). C'est générique : `pause_dialogue` en est le cas d'origine.
- `report { text }`.
- `propose { text, item? }` : le hors-plan, dans la boîte ; l'agent continue le plan.

## La montée

1. Une doc sur un dépôt de test (le témoin Gemini de ce lot).
2. Une doc de rag3db.
3. De petits lots de code.

La tendance des bilans de la tâche, lue par le juge, autorise la marche suivante. La décision
reste au superviseur.

## Les témoins

- **Sur faux modèle** : un plan à trois éléments, dont un à valider.
  - Le bloquant part dans la boîte, et les deux autres se font en deux pas : deux runs
    journalisés, deux `step_done`.
  - La réponse du superviseur écrite pendant le pas 1 est lue à la frontière.
  - Le budget force un `step_done` sur un pas qui s'étire.
  - Une proposition écrite pendant un pas ; le plan continue ; la réponse « oui » à la frontière
    ouvre l'élément.
  - L'arrêt passe tout de suite.
- **Sur Gemini (exigé)** : la tâche « doc sur un dépôt de test » de bout en bout. Le juge rend un
  bilan à chaque pas, et la doc est sur la branche de la tâche.

## Les noms

Les identifiants sont en anglais (règle du 6 septembre) : l'agent `teammate`, `edit_plan`,
`step_done`, `report`, `plan.json`, `mailbox.jsonl`, le sujet `mailbox.<task>`, la route
`/ide/teammate/mailbox`, `StopReason::Terminal` et `with_terminal_tools`. Le français reste
pour la doc, les commits et les textes rendus à l'humain.

## Ce qui n'est pas décidé

- Le nombre de pas avant compaction (équipier-2).
- Le grain laissé à l'agent : combien un élément doit-il contenir ? On lira dix tâches jugées
  avant de régler.
- Le juge qui parle au pas suivant (équipier-2).
- La surveillance du fichier de la boîte (un ajout à la main qui réveille l'équipier).
