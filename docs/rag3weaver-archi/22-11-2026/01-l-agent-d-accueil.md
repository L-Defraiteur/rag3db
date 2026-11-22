# L'agent d'accueil

*rag3weaver archi, 22 novembre 2026, à la demande de Lucie relayée par c7. Une page de conception commune avec recherche,
qui fait la maquette de l'écran, avant tout code. Ce qui est dit « aujourd'hui » a été relevé dans le code de master, sans
être joué.*

## Ce que Lucie veut

Un IDE qui est d'abord un chat, avec une barre des agents à gauche, et un **agent d'accueil toujours là**, même quand rien
n'est ouvert. Son exemple :

> « j'aimerais qu'on regarde ce repo, tu peux le cloner puis me l'ouvrir »
>
> — « je n'ai pas d'outils de code, veux-tu que je te prépare un agent pour t'aider là-dessus ? »

L'agent d'accueil n'est donc pas un agent de code. Il sait où en est la machine, il prépare le terrain (un dossier, un
clone, un agent de code), et il passe la main.

**Le cadre, posé par Lucie le 22 novembre vers 22 h 15.** On travaille désormais l'UX de l'IDE, jugée dans son Chrome,
avec deux règles :
1. simple à utiliser mais très avancé, et qui met le produit en valeur ;
2. un novice arrive, parle à un agent, et ça commence pour lui, sans adresse de dossier tapée ni réglage.

L'utilisateur passe son temps à parler à un agent. Les détails (fiches mémoire, tâches, chats d'agents) sont des liens
que l'agent lui donne, et l'agent d'accueil **génère à la volée des affichages** dans le fil : tableaux, cartes, liens
(section 7).

## 1. Ce qu'il est

- **Toujours là.** L'hôte l'ouvre à son démarrage, sans dossier, sans dépôt. Il est la première entrée de la barre des
  agents, et l'écran d'arrivée de l'IDE est son chat.
- **Sans outils de code.** Il ne lit pas, n'écrit pas et n'exécute rien dans un dépôt : ni `read_file`, ni `edit_file`, ni
  `run_command`, ni la recherche de code. C'est ce qui le distingue de l'atelier (page 02 du 21 novembre), qui a les outils
  standards de fichiers sur ce qu'il a attaché.
- **Un agent comme les autres pour l'hôte** : une déclaration, une clé d'agent stable, une boîte, ses appels de modèle au
  registre des dépenses d'ED. Il passe par la même porte, et ses cartes sont celles du mode auto.

## 2. Sa déclaration

Un gabarit livré, `templates/backends/welcome/backend.json`, posé chez l'utilisatrice au premier démarrage comme l'atelier,
et ouvert par l'hôte sans manifeste de dépôt :

```json
{
  "name": "welcome",
  "kind": "welcome",
  "agent": { "name": "Accueil", "key": "agent:welcome" },
  "tools": ["machine_state", "open_folder", "clone_repository", "create_code_agent"],
  "workspace": null
}
```

- **Le modèle** : celui du régime de l'hôte, comme l'équipier ; rien à déclarer.
- **Pas de `workspace`** : il n'a aucune racine. Le bac à sable n'a donc rien à monter pour lui, et `judge_all` ne le
  concerne pas : il n'exécute pas de commande.
- **Les identifiants sont en anglais** (`welcome`, ses outils) ; son nom à l'écran, « Accueil », est un libellé.

## 3. Ses outils

| Outil | Ce qu'il fait | Porte |
|---|---|---|
| `machine_state` | lit l'état de la machine : les dépôts attachés (nom, chemin, fraîcheur de l'index), les agents et leurs tâches (état, dernière ligne de boîte), les dépenses du mois (le registre d'ED : total, plafond), les hôtes déclarés | lecture seule, sans carte |
| `open_folder(path)` | attache un dossier, comme `rag3weaver-backend ouvrir` (`code_tree::open_folder`), **sans index** : l'index reste une proposition (page 02 du 21) | un chemin hors de la racine de création passe par la carte `open_root` de `judge_all` |
| `clone_repository(url, into?)` | clone dans `~/git_workspaces/<nom du dépôt>` (la convention de Lucie), puis l'attache comme `open_folder`. **Un dossier qui existe déjà est refusé**, avec la proposition de l'ouvrir | **toujours une carte** : réseau et écriture, classe `outbound`. La carte montre l'URL et le dossier. Une URL qui porte un jeton est refusée net |
| `create_code_agent(repository, request?)` | ouvre un agent de code (une tâche de l'équipier) sur un dépôt attaché et rend son lien dans l'IDE. Il n'écrit la première demande que si l'utilisatrice l'a dictée | sans carte : rien ne sort, l'agent créé a ses propres portes |

**Ce qu'il n'a pas, exprès** : pas de shell, pas de lecture de fichiers, pas de mémoire longue en écriture (il la **lit**,
pour dire ce qui a été fait et décidé : `recall_fiches`, `get_fiche`), pas de push, pas de création de dépôt distant. Ces gestes appartiennent aux agents de
code et passent par leurs cartes.

## 4. Ses limites, dites honnêtement

L'invite lui fait dire ce qu'il ne sait pas faire, au lieu de le tenter ou de le promettre :
- une demande de code (lire, corriger, expliquer un fichier, lancer des tests) → « je n'ai pas d'outils de code » et la
  proposition d'un agent de code sur le bon dépôt ;
- une demande qui sort de la machine (pousser, publier, créer un dépôt GitHub) → il le dit, et renvoie à l'agent de code,
  qui aura sa carte ;
- un état qu'il ne peut pas lire (un hôte injoignable, un registre de dépenses absent) → il le dit, sans deviner.

## 5. L'invite, qui part neutre

Comme l'agent de la page 02 du 21 : il ne propose ni index ni agent sans raison, il demande avant de préparer, et il ne
cache jamais une carte. En anglais, comme l'invite de l'équipier :

```
You are the welcome agent of this IDE. You have no code tools: you cannot read, edit or run anything in a repository.
You know the machine's state (machine_state), and you prepare the ground: open a folder (open_folder), clone a repository
(clone_repository: always a card the user answers), and create a code agent on an attached repository (create_code_agent),
then give its link.
- When the user asks for code work, say plainly that you have no code tools, and offer to prepare a code agent on the
  right repository. Do it only on their yes.
- Propose nothing the request does not need: no index, no agent, no clone unless asked or clearly useful, and then ask.
- Never claim something is done while a card waits: say it waits for their answer.
- Something that leaves the machine (a push, a publication, a remote repository) is not yours: say so.
- Answer in the user's language, briefly.
```

## 6. Le parcours de l'exemple

1. Lucie : « j'aimerais qu'on regarde ce repo, tu peux le cloner puis me l'ouvrir » (avec l'URL).
2. Accueil : « Je n'ai pas d'outils de code. Je peux le cloner dans `~/git_workspaces/projets/<nom>` (une carte te le
   demandera), l'ouvrir, puis te préparer un agent de code dessus. Je le fais ? »
3. Lucie : « oui ».
4. `clone_repository` → **la carte** (classe `outbound`, l'URL et le dossier). Lucie dit oui → le clone, puis l'attache.
5. `create_code_agent` → « Ton agent de code sur `<nom>` est prêt : [ouvrir] ». Il apparaît dans la barre, sous l'accueil.
6. Lucie passe à lui, et c'est lui qui « regarde le repo », avec ses outils et ses propres cartes.

## 7. Les composants du chat, en gabarits

**Précision de Lucie (22 novembre, 22 h 2x)** : « l'interface principale, c'est des agents ; il peut arriver dans le chat
que l'agent nous génère des composants à la volée ou des menus préfaits qu'il fait surgir ; mais il faut des gabarits
tout le temps, quoi qu'il arrive, pour les menus intra-chat ».

**Le principe** : l'agent ne dessine rien. Il **choisit un gabarit** dans un catalogue déclaré, et le **remplit** par
un outil typé, `show(component, data)`. L'hôte vérifie `data` contre le schéma du gabarit ; l'IDE ne rend que des gabarits
qu'il connaît. Jamais de HTML ni de JS écrit par le modèle.

**Le catalogue** est une famille versionnée de gabarits livrés (`templates/chat-components/<nom>.json`, chacun : son nom,
sa version, son schéma JSON, sa description pour le modèle). Le rendu est à recherche, un composant React par gabarit.

| Gabarit | Ce qu'il montre | Ses données |
|---|---|---|
| `links` | une liste de liens | `items: [{text, link}]` |
| `table` | un tableau | `columns`, `rows` (une cellule : texte ou `{text, link}`) |
| `action_card` | une carte avec ses actions | `title`, `subtitle?`, `status?`, `link?`, `actions?` |
| `choice_menu` | un menu à choix, qui surgit | `question`, `options: [{label, says}]` |
| `folder_picker` | un sélecteur de dossier, sans chemin tapé | `candidates: [{name, path, why?}]`, `allow_new` |
| `machine_state` | l'état de la machine | ce que rend l'outil `machine_state` (dépôts, agents, tâches, accords en attente, index) |
| `spending` | les dépenses | `items: [{label, value, unit?, hint?}]`, le plafond |
| `memory_cards` | des fiches de la mémoire | `items: [{title, summary, link}]` |

Les états d'un `status` sont `running`, `waiting` (« attend ton accord »), `done`, `failed` et `idle` (un agent arrêté
sans erreur). Un `hint` est une phrase courte sous un chiffre.

**Les règles** :
- **rien d'inventé** : `data` ne porte que ce qu'un outil vient de rendre dans le même tour. Un `link` est une route de
  l'IDE (`/ide/…`) ou une fiche de la mémoire ; l'hôte retire le reste, en le disant ;
- **une action n'agit jamais d'elle-même** : un bouton (`actions`, `options`) envoie à l'agent sa phrase `says`, comme
  un message de l'utilisatrice, signé humain ; aucun bouton n'appelle un outil en direct. Le `folder_picker` envoie
  « ce dossier : <name> » ;
- **un gabarit inconnu ou une `data` hors schéma est refusé**, et l'outil dit lequel et pourquoi : l'agent corrige ou
  répond en texte ;
- **un gabarit neuf est un lot** : son schéma, sa description, son rendu et son témoin navigateur. Le catalogue ne
  grandit pas au fil d'une conversation.

## 8. L'agent d'accueil est la page d'accueil

Tout ce qu'une barre latérale d'accueil offrirait à un utilisateur avancé passe par lui, en composants : les projets
(`folder_picker`, `table`), les agents et leurs tâches (`action_card`), les accords en attente (`action_card` en
`waiting`), les dépenses (`spending`), l'état des index et des hôtes (`machine_state`), la mémoire (`memory_cards`). La
barre gauche des agents reste pour la navigation : passer d'un agent à l'autre.

À l'arrivée, sans un mot de l'utilisatrice, il ouvre par un `machine_state` court et un `choice_menu` : « Tes projets
récents », « Un projet neuf », « Cloner un dépôt », « Ce qui attend ton accord » (s'il y en a).

## 9. Un novice, sans dossier tapé

Le novice ne connaît ni chemin ni réglage. L'agent d'accueil s'en charge :
- **à l'arrivée**, un premier message court, puis un `choice_menu` : « Tes projets récents » (ce que `machine_state`
  trouve sous `~/git_workspaces`, par date, dans un `folder_picker`), « Commencer un projet neuf », « Cloner un dépôt » ;
- **un projet neuf** : il propose un nom, crée le dossier sous `~/git_workspaces/<nom>` (`open_folder`, la racine de
  création ne demande pas de carte), puis un agent de code dessus ;
- **un projet existant** : il le reconnaît à son nom (« mon jeu », « le site ») parmi les dépôts de la machine, et demande
  « c'est celui-ci ? » avec sa carte, au lieu d'un chemin ;
- **jamais un réglage à faire à la main** : le modèle, le mode auto, l'index sont des défauts ; l'agent les dit s'ils
  comptent pour la demande (« je n'ai pas encore indexé ce projet ; je le fais ? »).

## 10. La maquette de l'écran

*Section de recherche.* Un seul écran : le chat au centre, toujours ; la barre des agents à gauche, l'accueil toujours
en tête. Il n'y a plus de page de formulaires ; la plomberie est derrière « Avancé ».

### L'arrivée, sans rien d'ouvert

```
┌─────────────────────┬────────────────────────────────────────────────────────────┐
│ rag3weaver          │  Accueil                                    ◐   ⚙ Avancé   │
│ ┌─────────────────┐ │────────────────────────────────────────────────────────────│
│ │ 🔍 un agent…    │ │                                                            │
│ └─────────────────┘ │  Bonjour. Sur cette machine : aucun dépôt ouvert, aucun    │
│                     │  agent en cours. Je n'ai pas d'outils de code : je peux    │
│ ● Accueil           │  ouvrir un dossier, cloner un dépôt, et te préparer un     │
│                     │  agent de code dessus. Que veux-tu faire ?                 │
│ (aucun agent)       │                                                            │
│                     │  [Ouvrir un dossier…]  [Cloner un dépôt…]                  │
│ + Nouvel agent      │                                                            │
│                     │ ┌────────────────────────────────────────────────────────┐ │
│                     │ │ Écris…                                            ➤   │ │
│                     │ └────────────────────────────────────────────────────────┘ │
└─────────────────────┴────────────────────────────────────────────────────────────┘
```

- La saisie a le focus. Le premier message est celui de l'agent d'accueil, tiré de `machine_state` (le nombre de
  dépôts, d'agents en cours, d'accords qui attendent ; zéro se dit « aucun »).
- Les deux boutons sous le message sont les mêmes gestes que les phrases : « Ouvrir un dossier… » montre le sélecteur
  dans le fil (§ 7.3), « Cloner un dépôt… » demande l'adresse puis pose la carte (§ 7.2).

### La barre des agents

- **Accueil**, toujours en tête, avec un point s'il attend une réponse.
- **Actifs** : un tour en cours (●), un accord qui attend (◐, « attend ton accord »). **Autres** (○), du plus récent au
  plus ancien, avec « il y a 3 j ». Chaque ligne : le projet et la tâche (`rag3db · lot B`), puis l'état en une ligne.
- Un clic ouvre le chat de l'agent au centre ; l'adresse suit (`/ide/code/<id>/chat`), le retour en arrière ramène.
- La recherche filtre par projet ou tâche. « + Nouvel agent » écrit à l'agent d'accueil « prépare-moi un agent ».
- La liste vient de l'état de l'hôte (`/ide/etat`, les dépôts et leurs tâches) : rien de neuf à calculer côté page.

### 7.2 La carte du clone, dans le chat de l'accueil

```
   tu : j'aimerais qu'on regarde github.com/x/y

   Je n'ai pas d'outils de code. Je peux le préparer :
   ┌────────────────────────────────────────────────────────────┐
   │ Cloner un dépôt                                   attend ton accord │
   │ https://github.com/x/y                                             │
   │ → dans ~/git_workspaces/y   (le dossier n'existe pas : il sera créé) │
   │ puis l'ouvrir, et créer un agent de code dessus                    │
   │                                       [Refuser]   [Accorder]       │
   └────────────────────────────────────────────────────────────┘
   ✓ Cloné dans ~/git_workspaces/y, ouvert.
   ✓ Ton agent de code sur y est prêt : [Ouvrir son chat]
```

- C'est la carte d'accord de l'IDE (le même composant que celle d'une commande), de classe `clone` : l'adresse en clair,
  la destination avec `~`, ce qui se passera ensuite. Une adresse qui porte un jeton est refusée avant la carte.
- Les deux lignes suivantes sont les résultats de `open_folder` et de `create_code_agent` ; le bouton « Ouvrir son
  chat » est le lien que rend `create_code_agent`, et l'agent apparaît aussitôt dans la barre, parmi les actifs.

### 7.3 Le sélecteur de dossiers, dans le fil

- Le sélecteur s'ouvre en carte dans le fil : un fil d'Ariane depuis `~`, en tête les dépôts git trouvés sous
  `~/git_workspaces`, puis les sous-dossiers. Seul ce que l'hôte refuse vraiment est marqué d'avance, avec sa raison.
- « Ouvrir ce dossier » appelle `open_folder`, puis l'accueil propose « Créer un agent de code dessus ? ».
- Le même sélecteur est dans « Avancé », avec le champ de chemin (qui comprend `~`).

### Ce qu'il me faut des outils (pour archi)

- `machine_state` rend, par agent : son id d'IDE (pour le lien et la barre), le projet, la tâche, l'état (`running`,
  `waiting_approval`, `idle`), l'heure du dernier tour, et le nombre d'accords qui attendent en tout.
- `create_code_agent` rend l'id de l'IDE de l'agent créé et son lien (`/ide/code/<id>/chat`).
- `clone_repository` rend la destination retenue avant la carte (pour l'y écrire), et après l'accord, le chemin cloné.

## Les lots, si la page est retenue

1. La déclaration, l'ouverture par l'hôte au démarrage, `machine_state` en lecture seule, la mémoire en lecture, et son
   témoin : l'agent répond « pas d'outils de code » à une demande de code, sur un faux modèle.
1 bis. Le catalogue des gabarits (`templates/chat-components/`), l'outil `show(component, data)` qui les remplit sous
   leur schéma, le contrôle des liens, et ses témoins (archi) ; leur rendu et leurs témoins navigateur (recherche).
2. `open_folder` et `create_code_agent` (rend le lien), avec leurs témoins.
3. `clone_repository`, toujours sous une carte, avec un témoin qui refuse une URL à jeton.
4. L'IDE : l'accueil en tête de la barre, l'écran d'arrivée (recherche) ; le terminal (tty).

## Tranché par c7, par défaut (22 novembre, Lucie en essai ; elle corrige ensuite)

- **Son nom à l'écran** : « Accueil ».
- **La mémoire longue** : en lecture seule. Il doit pouvoir dire ce qui a été fait et décidé.
- **La destination d'un clone** : `~/git_workspaces/<nom du dépôt>`, refusé s'il existe déjà, avec la proposition de
  l'ouvrir.
