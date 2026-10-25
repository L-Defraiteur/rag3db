# Le journal des runs, borné par poids et par âge — et des runs épinglés

Décidé par Lucie le 25 octobre 2026 au soir, relayé par c7. Le lot suit le journal des runs (page
04) et le juge des outils (page 05).

Lucie : « par poids plutôt que par nombre, au moins 5 Go, les gens ne sont pas sur des PC
préhistoriques ». Son dimensionnement : 3 000 sessions par jour, gardées trois mois, soit environ
270 000 runs ; à 10 Kio le run, 2,7 Gio.

## Les bornes

- **Le poids de tout le journal** : `RAG3WEAVER_RUNS_MAX_OCTETS`, 5 Gio par défaut, compté sur
  les fichiers compressés, tous backends confondus.
- **L'âge d'un run** : `RAG3WEAVER_RUNS_MAX_JOURS`, 90 jours par défaut.
- Un run tombe à la première borne atteinte. Les plus vieux tombent d'abord, et l'hôte dit
  combien, pour quelle borne, et ce qui reste.
- `RAG3WEAVER_RUNS_MAX` (200 fichiers par backend) disparaît.
- **La borne d'un run** reste : `RAG3WEAVER_RUN_MAX_OCTETS`, 2 Mio d'événements. Au-delà, le
  milieu tombe, et le fichier le dit (`evenements_tronques`). C'est un plafond, pas une taille :
  un appel d'outil ordinaire pèse quelques Kio (les mesures plus bas).
- **Ce que l'écran a montré ne se coupe jamais** (c7, le soir) : une borne coupe les événements
  intermédiaires, jamais le rendu final d'un appel ni ses arguments. La borne de 64 Kio qui vivait
  sur le `rendu` d'un `/call` journalisait deux recherches à 4 et 12 résultats sur 30
  (`{tronque, debut}`) : elle est retirée. Témoin : une lecture de 200 Kio, journalisée entière.

L'hôte relève les journaux du disque à la première écriture, puis tient le compte à chaque
écriture et à chaque effacement. Relever le disque à chaque run coûterait un parcours de
centaines de milliers de fichiers.

## Compressés

Chaque journal s'écrit en `runs_root/<backend>/<run>.json.zst` (zstd, niveau 3). Le lecteur lit
les deux formes : le juge, la vue et les témoins passent par `JournalDesRuns::lire`. Un `.json`
d'avant ce lot se lit tel quel. Un ajout le réécrit compressé, et l'ancien fichier part.

## Les runs épinglés

Un run marqué `epingle` dans son fichier (`{raison, instant}`) ne tombe jamais, ni par le poids
ni par l'âge. Ce sont les runs que la graine de vision (§7) veut retrouver longtemps après :
qui a déclaré un outil, quand, et dans quel contexte. Trois choses épinglent :

- **La déclaration d'un outil.** C'est un appel `declare`, `declare_set` ou `propose` dont un
  fichier est le graphe d'un outil (`%% tool:`) ou un manifeste qui en nomme (`tools`). Il est
  reconnu à deux endroits :
  - dans les événements du run (l'appel d'un agent) ;
  - dans le journal d'un appel `/call` (un Claude Code, par MCP).
- **Le juge**, quand il range un défaut bloquant : le run en est la preuve.
- **Un appel explicite** : `POST /ide/runs/epingler {backend, run, raison}`, ou
  `DaemonHost::epingler_un_run`. Un run sans journal est un refus.

Un run épinglé compte dans le poids. Si le journal ne tient plus que des runs épinglés au-delà de
la borne, l'hôte le dit et n'efface rien.

Pour savoir si un run est épinglé, l'élagage lit son fichier la première fois qu'il serait
tombé, puis garde la réponse.

## Ce que pèse un run

Mesuré le 25 octobre 2026. « JSON » est le journal compact (ce lot n'écrit plus de JSON indenté),
« zstd » est le fichier sur le disque.

| run | JSON | zstd | taux |
|---|---|---|---|
| appels MCP réels sur l'hôte de luciepc (3 événements) | 0,7 Kio | 0,25 Kio | ×2,8 |
| idem (7 événements) | 1,3 Kio | 0,32 Kio | ×3,9 |
| idem, `search_code` (12 événements) | 2,7 Kio | 0,85 Kio | ×3,2 |
| un appel d'outil par /call (`chat_qui_code`) | 0,9 Kio | 0,4 Kio | ×2,3 |
| un tour qui code (`chat_qui_code` : edit_file, tours, tâche) | 2,9 Kio | 1,0 Kio | ×3,1 |
| un tour de chat (`e2e_juge`, cinq runs) | 2,7 à 4,2 Kio | 0,8 à 1,0 Kio | ×3,1 à ×4,3 |

Ce qu'on en lit :
- Un run ordinaire pèse un à quatre Kio en JSON, moins d'un Kio compressé.
- Le taux est de ×2 à ×4, pas ×5 à ×10 : un petit fichier se compresse moins, faute de
  répétitions. Un long tour de chat (des dizaines d'appels, des retours de plusieurs Kio) devrait
  se compresser mieux ; il reste à mesurer sur une vraie session.
- À un Kio par run, les 270 000 runs de trois mois pèsent environ 270 Mio, vingt fois sous la
  borne. Les 5 Gio ne seront atteints que par de longues sessions. C'est alors la borne d'âge qui
  tranche le plus souvent.

## Les témoins

- `journal_des_runs::tests` :
  - trois runs sur deux backends sous une borne minuscule, le premier épinglé. Les deux autres
    tombent dans l'ordre, l'épinglé reste. Le compte des octets est celui des fichiers sur le
    disque, et un relevé à neuf (un hôte relancé) donne le même compte ;
  - un run plus vieux que la borne d'âge tombe, l'épinglé jamais. Un `.json` d'avant se lit, et
    un ajout le réécrit compressé avec son épingle ;
  - la reconnaissance d'une déclaration d'outil ;
  - la borne d'un run (le milieu tombe).
- Les mesures : `chat_qui_code` (un tour qui code, un appel d'outil) et `e2e_juge` (un tour de
  chat) disent ce que pèse leur journal.
