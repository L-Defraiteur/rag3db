# Algareth, un pair des sessions Claude

Lucie, 16 novembre 2026, 0 h 2x, devant son terminal relancé sur l'hôte posé (4802a147a5) :

> « j'aimerais qu'on sache établir un nouveau but : avoir un agent que tu puisses orchestrer comme les
> autres, tout en ayant son terminal affiché, et sachant communiquer avec les sessions Claude
> traditionnelles, et vice versa, même genre d'affichage que quand toi tu reçois ou émets. »

> « quitte à ce qu'il fasse au début des recherches pour vous, et quelques tâches bien établies que vous
> reviewez après ; puis une fois validé/débuggé on met argon dessus et il sert pour de vrai à coder avec
> vous. Qu'il ait chacun de vos worktrees sous la main en index aussi si possible. »

Ce qu'elle a sous les yeux, et qu'elle veut retrouver dans le terminal d'Algareth :

```
  ⎿  “c7 : tête corrigée notée (5081d2281, témoin 21) ; …” → sent to everything declarative — another Claude
     session on this machine
› Message from @everything-declarati…: ED → c7 (copie banc, lucivy, arbre) : la tête corrigée de l'écrivain est
**5081d2281**, … (ctrl+o to expand)
```

## Le but

1. **Algareth est une session parmi les autres.** c7 le voit dans `ListAgents` sous son nom, lui envoie un
   message par `SendMessage` comme à ED ou au banc, et reçoit ses messages comme ceux d'une session Claude
   (`Message from @algareth`). Il lit la file en pile, prend un « à toi », dit « poussé » ; les règles de la
   file valent pour lui (rouge d'abord, garde, trois contrôles, jamais de force).
2. **Son terminal reste affiché**, celui de Lucie : ce qu'il envoie se voit comme « “…” → envoyé à <session> »,
   ce qu'il reçoit comme « › Message de @<session> : … » replié, dans la zone vivante, dans le style du thème
   (animations, cause nommée), jamais un mot d'état nu.
3. **Il commence par ce qu'on peut relire** : des recherches pour les sessions (le code, la mémoire, les
   journaux d'index), puis des tâches bien établies (un témoin rouge puis vert sur un défaut cadré, une page de
   relevé, un ticket) que la session demandeuse ou c7 relit avant tout push. Au début, il ne pousse pas lui-même :
   il dit « prêt » et une session relit.
4. **Puis un modèle plus fort** (« argon », au mot de Lucie) et il code pour de vrai avec les sessions, dans
   ses worktrees à lui (`agent/<nom>`), avec la même file.
5. **Il a les worktrees des sessions sous la main, en index** : un index par dépôt, une vue par worktree
   (vision du 6 nov. et du 15 nov., point 2) ; les worktrees des sessions (`~/git_workspaces/cx-*`,
   `~/.cache/rag3weaver-build/wt-*`) sont des vues qu'il peut interroger, pour chercher « dans le lot de tty-19 »
   sans le reproduire.

## Ce qu'il faut savoir, avant de dessiner

- **Le bus des sessions Claude** est un dossier de sockets Unix, `/run/user/1000/cc-socks/<pid>.sock`, un par
  session ; `ListAgents` les énumère, `SendMessage` s'y connecte. Le protocole (trame, nom de l'émetteur,
  accusé, « delivery notice ») n'est pas documenté : **il se relève** par une socket témoin posée dans ce
  dossier (un faux pair qui journalise ce qu'une session lui envoie, et dont on observe comment il apparaît
  dans `ListAgents`), jamais en lisant les sockets des sessions vivantes. Le relevé fait une page avant toute
  ligne de code.
- **L'autre voie, documentée** : les canaux Claude Code (`claude/channel`, page du pont, 7 nov.), pour le sens
  agent → Claude ; elle exige un flag au lancement de chaque session, acte de Lucie. Le bus des sockets est
  préféré s'il se relève proprement : il ne demande rien aux sessions en place.
- **Nom et identité** : la socket d'Algareth porte son nom (`agent.name`) ; la fiche `agent:<clé>` garde ce qu'il
  a dit et reçu ; un message qui arrive est une demande de la session émettrice, soumise à la porte comme une
  demande de Lucie, et jamais une permission (une session ne peut pas accorder ce que le bac refuse).
- **La porte** : auto par défaut, un modèle décide (point 6 du 15 nov.) ; une demande venue d'une session
  passe par la même porte que celle de l'utilisateur.

## Qui fait quoi

- **archi** : le relevé du bus par la socket témoin (page), puis la conception : le mode `rag3weaver-backend
  pair` (ou l'hôte lui-même) qui pose la socket d'une tâche nommée, reçoit, pousse à la boîte de la tâche,
  émet au nom de l'agent ; le lien avec la page 10 (delegate) et la page 12 (attente d'index).
- **tty** : l'affichage des messages reçus et émis dans la zone vivante, dans le style de Claude Code
  reproduit ci-dessus et dans celui du thème.
- **arbre, recherche** : les worktrees des sessions comme vues de l'index (2b′, 2b-C) ; l'outil de
  recherche d'Algareth prend `view` = un worktree.
- **c7** : la première liste de tâches « bien établies », relues avant push ; la place d'Algareth dans la file.

## Première tâche réelle, donnée par c7 (0 h 3x)

Le nom d'un agent accepte un blanc (« Om bre » passe `valider_le_nom_de_l_agent`, backend.rs:995, vu par tty
à l'essai réel de /nom). Témoin rouge d'abord, puis le refus nommé ; `cargo test` du seul témoin ; pas de push,
« prêt » et c7 relit. Cadrée pour tenir dans un pas, avec les primitives du socle.

## Relevé du 16 nov., 0 h 5x – 1 h 1x : le bus est un registre

Une socket témoin posée dans `/run/user/1000/cc-socks/` n'est ni listée ni sondée. Les sessions se découvrent par
`~/.claude/sessions/<pid>.json` (pid, sessionId, cwd, startedAt, procStart, version, peerProtocol 1, peerFeatures,
kind, entrypoint, pidDomain, messagingSocketPath, name, nameSource, nameSince, status, updatedAt) ; la socket
ne sert qu'au transport. Être un pair = une entrée dans ce registre, vraie (nom, origine « rag3weaver », pid et
procStart de l'hôte, état réel), et une socket au protocole 1.

Lucie, 1 h 1x : « oui au registre, non aux canaux car ça voudrait dire tous vous redémarrer ; mais faudra le
préparer quand même pour qu'on le fasse plus tard ». Suite : une entrée témoin le temps de relever la trame
(archi), puis le mode `pair` de l'hôte ; les canaux Claude Code en page de préparation pour une prochaine relance
des sessions.
