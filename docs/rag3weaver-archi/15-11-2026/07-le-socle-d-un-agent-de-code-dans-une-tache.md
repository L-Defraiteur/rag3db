# Le socle d'un agent de code dans une tâche : ce qui existe, ce qui manque

15 novembre 2026, 16 h. Lucie, devant son terminal, sur sa tâche `essai-tty` : « il n'a toujours
pas d'outils pour éditer du code comme un agent standard ; j'aimerais vraiment retrouver
l'expérience d'un agent de code standard tout en gardant toutes ces fonctionnalités, là ça parle un
peu trop de gabarits » (c7). La direction est la sienne : le 7 novembre, « se comporte comme un
Claude Code normal » ; le 8 novembre, « capable de tout, porte en approbation, 200 lignes, trois
modes ». La page de référence est la [vision du 15 novembre](../../orchestration/visions/15-11-2026-l-agent-est-un-agent-de-code-qui-gere-ses-worktrees.md) (« l'agent est un agent de code, sauf qu'il gère builtin des worktrees et ça les indexe automatiquement »). Cette page est un relevé, pas une conception. Le lot vient après la garde de l'équipier
(forme B).

## Ce qui existe

- **Les outils d'un pas sont ceux du manifeste du backend**, rien d'autre.
  `run_teammate_turn` (`src/daemon_host/teammate_tools.rs`) prend `OutilsDuChat::de(b, …, true)` :
  les outils que le manifeste déclare, plus les outils de déclaration (`declare`, `declare_set`,
  `propose`). Le manifeste de Lucie
  (`~/.config/rag3weaver/code-rag3db/templates/backends/code/rag3db.json`) ne déclare ni
  `edit_file` ni `run_command` : son équipier lit, cherche et se décrit, mais n'édite pas.
- **Le gabarit livré les a.** `templates/backends/code/backend.json` déclare `edit_file`
  (`write_files`) et `run_command` (`run_commands`), et `workspace.commands: "approval"`.
- **Le repli sans index, `FileTools`**, a `read_file`, `list_files`, `grep_files`, `edit_file`
  (un passage unique `old → new`, ou le fichier entier par `content`) et `search_code`, sur le
  worktree de la tâche, chemins confinés. Mais il ne sert **que si le backend ne s'ouvre pas**.
- **La porte des commandes existe.** Dans une tâche, `run_command` passe par
  `workspace.commands` (`Off`, `approval`…). Une commande hors de la liste sûre attend l'accord :
  une ligne `approval` part dans la boîte (`StepTools::after`), et tty l'affiche avec la commande
  (`o` accorde, `n` refuse, 13 novembre).
- **L'invite d'un pas est celle de l'équipier** (`SYSTEM_PROMPT` dans `teammate.rs`) : les plans,
  les pas, `step_done`, la boîte. Celle d'un agent de code (`templates/apps/code/chat.json`,
  450 caractères : `search_code`, `read_file` avant toute édition, `edit_file`…) ne sert qu'au chat
  de l'IDE.

## Ce qui manque

1. **Un socle d'agent de code dans toute tâche, quel que soit le manifeste** :
   - `read`, `edit` (un texte unique), `write` (un fichier neuf), `grep`, `glob`, `list` ;
   - `run_command` derrière la porte : une liste sûre, sinon l'approbation.

   Aujourd'hui, ce socle n'existe qu'à moitié, et seulement en repli. Il faut le proposer toujours
   dans une tâche ; le backend n'y ajoute que ses outils d'index et de mémoire (`search_code`,
   `outline`, `usages`, `impact`, les fiches). Sous le même nom, l'outil du backend gagne s'il
   existe (l'index se met à jour au retour), sinon c'est le socle qui sert.
2. **`write` et `glob`** n'existent pas en propre : `edit_file` écrit un fichier entier par
   `content`, et aucune recherche de chemins par motif n'existe.
3. **Les outils déclaratifs** (`declare`, `declare_set`, `propose`) restent aux backends
   déclaratifs, ou passent en bas de la liste d'un backend de code. C'est le « ça parle un peu trop
   de gabarits ».
4. **L'invite** : un pas d'équipier dans une tâche de code prend l'invite d'un agent de code (celle
   de `apps/code/chat.json`), et la boucle de l'équipier (plans, pas, boîte) s'y ajoute en
   seconde partie, plus courte.
5. **La porte dans tty** : elle est en place pour `run_command`. Il manque la même porte pour les
   écritures, si Lucie veut un mode « éditions approuvées » parmi ses trois modes. Ces trois modes
   sont à préciser avec elle avant le lot.

En attendant, c7 déclare `edit_file` et `run_command` dans le manifeste de Lucie, pour qu'elle
travaille aujourd'hui.

## Ce que le second essai de Lucie ajoute (c7, 16 h 3x)

Lucie a refait l'essai sur son terminal. L'équipier a bien rendu son `report`, mais l'a signé
« Réponse apportée au superviseur ». Viennent ensuite deux lignes de plomberie : « étape faite :
fait : … », écrite par l'équipier, puis « fait sans plan », écrite par l'hôte. Le modèle croit parler
à un chef. Deux points de plus pour le socle :

6. **L'invite dit à qui il parle.** Dans une tâche ouverte par le terminal ou le chat, le
   « superviseur » est l'utilisateur. L'équipier lui répond directement, une fois par tour, en
   texte, sans se citer ni dire « rapport fait ». Le mot « superviseur » reste pour une tâche qu'un
   autre agent a ouverte (une demande par la boîte, depuis Claude Code par exemple).
7. **Le résumé de `step_done` est une ligne d'état** pour l'hôte et le journal, jamais rendue comme
   un message de l'équipier. Elle reste dans la boîte avec son genre (`step_done`), pour que le juge
   et la garde de la demande la lisent.

Pour tty : « étape faite » et « fait sans plan » se replient en une ligne d'état discrète, ou passent
derrière un réglage ; la prose à l'écran est la seule réponse.
