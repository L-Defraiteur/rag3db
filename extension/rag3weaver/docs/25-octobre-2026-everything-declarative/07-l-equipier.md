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
2. **Le mini-plan.** Le premier pas écrit le mini-plan par `editer_le_plan` ; l'agent le
   réédite ensuite librement (plus bas). Il contient la
   demande telle que l'agent la comprend, ce qu'il propose, puis des éléments. Chaque élément
   porte :
   - un statut : `a_faire`, `en_cours`, `fait`, `bloque`, `abandonne` ;
   - `seul` (faisable sans validation), ou bien `validation` avec la question posée.

   Chaque élément à valider part dans la boîte à l'édition qui le pose : les bloquants
   d'abord. L'agent attaque ensuite le premier élément faisable seul, sans attendre la réponse.
3. **Le pas.** L'agent travaille un élément, puis appelle
   `tour_fini { element, reussi, fait, reste }`. L'appel finit le pas :
   - réussi : l'élément passe à `fait` ;
   - raté : l'élément garde son statut, avec sa raison ;
   - dans les deux cas, une ligne part dans la boîte.

   Un pas est un tour de chat, donc un run du journal et un bilan du juge.
4. **Le budget du pas.** Au-delà de douze appels d'outil sans `tour_fini`, l'hôte refuse tout
   autre appel par le même message : « déclare d'abord tour_fini (réussi ou non, ce qui est fait,
   ce qui reste) ». Le refus est une réponse d'outil : l'agent le lit et s'y plie.
5. **La frontière.** Entre deux pas, l'hôte injecte dans la conversation, comme un message, ce
   qui est arrivé dans la boîte pendant le pas (réponses aux bloquants, consignes nouvelles) et le
   mini-plan courant. La conversation est la même pour ce lot. La compaction par les fiches est
   pour équipier-2.
6. **La fin.**
   - Plus d'élément faisable ni bloqué : l'agent rend compte (`rendre_compte`), et l'équipier
     s'arrête.
   - Seulement des éléments bloqués : il attend la boîte. Une réponse qui arrive le relance.

## Les messages

Il y a deux voies :
- **L'arrêt** passe tout de suite : `stop_run` sur le run du pas en cours, et l'équipier
  s'arrête là, sans pas suivant.
- **Tout le reste** attend la frontière. Un agent seul finit ce qu'il a commencé avant
  d'écouter.

## La boîte aux lettres

- **Un fichier par tâche**, `<dossier de la tâche>/boite.jsonl`, lisible sans IDE. Une ligne
  par message : `{instant, de, genre, element?, texte}`. Les genres sont `demande`, `consigne`,
  `reponse` (à un bloquant), `question` (un bloquant posé), `tour_fini`, `compte_rendu` et
  `arret`.
- **Un sujet du bus** par tâche, `boite.<tâche>`, où chaque ligne est aussi publiée.
- **Le superviseur écrit** par `POST /ide/equipier/boite {backend, tache, genre, element?,
  texte}`. La route ajoute la ligne au fichier et la publie. Un réacteur de l'hôte écoute le
  sujet :
  - une `demande` à une tâche absente l'ouvre ;
  - un message à un équipier qui attend le relance.

  Une ligne ajoutée au fichier à la main est lue à la frontière suivante, mais ne réveille pas
  un équipier qui attend (pas de surveillance de fichier dans ce lot).
- **L'agent écrit** par `rendre_compte`, et `tour_fini` y ajoute sa ligne.
- **L'IDE** montre la boîte et le mini-plan dans la vue de la tâche.

Le canal des sessions Claude n'est pas utilisé.

## Le mini-plan : libre, puis gardé en mémoire (Lucie)

**Pas de harnais sur le plan.** L'agent édite ses éléments quand il veut, comme il le sent. Il
peut juger son plan accompli ou obsolète, le clore avec sa raison et en ouvrir un autre. Les
seules contraintes dures sont le budget du pas (douze appels sans `tour_fini`) et l'arrêt (C4).

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
- `editer_le_plan` :
  - poser le plan entier ;
  - changer, ajouter ou retirer un élément (statut, raison, question) ;
  - ou clore ce plan (`accompli` ou `obsolete`, avec sa raison) et en ouvrir un autre.
- `tour_fini { element, reussi, fait, reste }` : il finit le pas. La boucle de l'agent apprend
  un **outil qui termine** : un nom déclaré terminal arrête la boucle après sa réponse
  (`StopReason::Terminal`). C'est générique : `pause_dialogue` en est le cas d'origine.
- `rendre_compte { texte }`.

## La montée

1. Une doc sur un dépôt de test (le témoin Gemini de ce lot).
2. Une doc de rag3db.
3. De petits lots de code.

La tendance des bilans de la tâche, lue par le juge, autorise la marche suivante. La décision
reste au superviseur.

## Les témoins

- **Sur faux modèle** : un plan à trois éléments, dont un à valider.
  - Le bloquant part dans la boîte, et les deux autres se font en deux pas : deux runs
    journalisés, deux `tour_fini`.
  - La réponse du superviseur écrite pendant le pas 1 est lue à la frontière.
  - Le budget force un `tour_fini` sur un pas qui s'étire.
  - L'arrêt passe tout de suite.
- **Sur Gemini (exigé)** : la tâche « doc sur un dépôt de test » de bout en bout. Le juge rend un
  bilan à chaque pas, et la doc est sur la branche de la tâche.

## Ce qui n'est pas décidé

- Le nombre de pas avant compaction (équipier-2).
- Le grain laissé à l'agent : combien un élément doit-il contenir ? On lira dix tâches jugées
  avant de régler.
- Le juge qui parle au pas suivant (équipier-2).
- La surveillance du fichier de la boîte (un ajout à la main qui réveille l'équipier).
