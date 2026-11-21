# Ce que cet agent sait : ce que l'hôte expose de la mémoire, ce qui manque

21 novembre 2026, session mémoire (96), à la demande de c7 pour Lucie : l'IDE se range autour des agents, et
l'utilisateur veut « avoir une vue sur leur mémoire ». Recherche dessine la page d'un agent. Cette page dit ce qui
existe, la route qui le rend lisible, et ce qui manque pour une vue en lecture seule. Elle ne propose aucun code avant
que recherche et c7 l'aient lue.

## Ce que l'hôte expose déjà

| Route | Ce qu'elle rend | Limite |
|---|---|---|
| `GET /ide/chat?backend=<id>` | `memory_writes` : les apports que le juge a rangés pour les runs de la conversation (les 20 derniers runs du journal, puis `select_sources` par `session`, puis `select_apports` par `source`, 500 lignes au plus), du plus récent au plus ancien | seulement si l'hôte tient déjà la mémoire ouverte ; les écritures de l'agent lui-même n'ont pas encore de `session` (M7), donc n'y sont pas |
| `GET /ide/l1?backend&run` | le L1 d'un run : pour chaque tranche rangée, les apports qui la citent (`cited_by`), avec leur fiche, leur section et leurs tours | un run à la fois |
| `GET /ide/bilans?backend=<id>` | les fiches rangées par le juge (`via = juge`) et leurs apports | le juge seul |
| `GET /ide/etat` | pour chaque session de chat, `memory: {backend, rag}` : quelle mémoire, et si son index est prêt | aucun contenu |
| `POST /ide/call` | n'importe quel outil de n'importe quel manifeste, dans la session de l'IDE : `recall_fiches`, `get_fiche`, `recall_apports`, `whois`, `list_turns`, `read_turns`, `list_slices`… | **aucune garde de lecture seule** : la même route écrit |

Côté IDE, `Chat.tsx` montre déjà la liste « fiches écrites » et un récapitulatif, nourris par `memory_writes`. `Bilans.tsx`
montre les bilans du juge. Il n'y a ni vue des tours, ni vue des tranches et des L1, ni navigateur de fiches.

## Ce qui dit qui est l'agent

- **L'identité d'un agent** est une fiche dont le nom est sa clé stable (`agent:<clé>`). `whois` la lit par `agent`,
  `handle` ou `alias`, et rend ses apports d'alias, de poignée et d'identité. `compose_restart` écrit « Identité :
  agent:<clé> » dans le texte de reprise.
- **Ce que l'agent écrit ne porte pas sa clé.** Les schémas ont `author` et `via` sur la fiche, l'apport et la source,
  `run` sur l'apport, la source et le tour, et `session` sur la source. Seul le hook de Claude Code pose `author`, avec
  la clé stable. Les écritures de l'hôte pour un chat ou un équipier ne posent que `via` (`teammate`) et `run`
  (`chat:<id>`, ou `<tâche>:<parent>` pour un équipier). À la clôture d'un plan, l'équipier range une fiche, une source
  (`session: teammate-<tâche>`) et un apport `L1`.
- **L'équipier écrit, mais ne relit pas.** Aucun `recall` à une frontière de pas : sa mémoire longue est une sortie,
  pas encore une entrée.

## Ce qui manque pour une vue « ce que cet agent sait », en lecture seule

1. **L'attribution.** Sans la clé de l'agent sur ses lignes, une vue ne peut filtrer que par runs, ce qui donne une
   vue par conversation, pas par agent. Il faudrait que l'hôte pose `author = agent:<clé>` (ou un champ `agent`) sur
   `put_turn`, `put_source` et `put_apport`, pour les chats comme pour les équipiers. C'est la M7 de la mémoire longue,
   déjà nommée dans le code.
2. **Une route de lecture à part.** Par exemple `GET /ide/agent/memoire?backend&agent=<clé>` (le nom est à choisir avec
   recherche), qui rendrait, bornés et par les outils de lecture de la mémoire, jamais par son catalogue :
   - l'identité (`whois`) ;
   - les fiches que l'agent a écrites ou nourries (les apports dont `author` est sa clé), avec leurs sections ;
   - pour chaque run de l'agent, ses tranches et leur L1 (`l1_du_run`) ;
   - des comptes (fiches, apports, tours, tranches) et la date de la dernière écriture.

   `/ide/call` ne convient pas à une page de lecture, puisqu'il écrit aussi bien qu'il lit. Une voie en lecture seule,
   par la `policy` des outils, en serait l'autre forme.
3. **Une réponse de Lucie** sur ce qu'« un agent » désigne dans l'IDE : une tâche d'équipier, une session de chat, ou
   une clé stable qui traverse les sessions (une session Claude Code, l'équipier d'une tâche relancée). La troisième
   forme est celle que la fiche d'identité porte déjà ; c'est elle qui donne une mémoire qui suit l'agent.

## L'ordre proposé, chacun rouge d'abord

1. L'hôte pose la clé de l'agent sur ce qu'il écrit pour lui, chats et équipiers. Un témoin : un tour et un L1
   d'équipier portent `author = agent:<clé>`.
2. La route de lecture seule, bornée, qui dit sa coupe. Un témoin : un agent avec deux runs voit ses fiches et ses L1,
   et pas ceux d'un autre agent.
3. La page, par recherche, sur cette route.

Ce que cette page ne règle pas : le rappel par l'équipier de sa propre mémoire à une frontière (ce qu'il relit). C'est
le pendant d'écriture de cette vue, à ouvrir à part si Lucie le veut.
