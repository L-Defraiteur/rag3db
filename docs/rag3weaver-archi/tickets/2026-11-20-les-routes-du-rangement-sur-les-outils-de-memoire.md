# Les routes du rangement sur les outils de mémoire

- **État** : ouvert, le 20 novembre 2026 ; pour archi, après le raisonnement borné
- **Famille** : produit
- **Pour** : archi
- **Gravité** : fonction à finir : `POST /ide/turns/read` rend encore « not yet », le tri de `GET /ide/l1` relit `from_turn` dans le texte de la section, et une tranche rangée ne purge pas ses tours.
- **Atteignable en service** : oui, l'IDE et le terminal de tty appellent ces routes
- **Touche rag3weaver** : oui, `src/ide.rs`, `src/daemon_host/chat.rs`, `src/daemon_host/rangement.rs`
- **Signalé par** : mémoire, le 20 novembre 2026, à l'atterrissage de ses lots

## Ce qui est sur master

- `memoire-turns-read` (42ed89c89) :
  - `list_turns(run, role, tool_name, status, since, until, limit)` rend `{turns, total}`, du plus récent au plus ancien ;
  - `search_turns(query, mêmes filtres, options)` cherche en plein texte sur Turn seul ;
  - `read_turns(run, from_turn, to_turn)` rend `{run, turns: [{turn, state: indexed|gone, messages}]}`.
- `memoire-turns-expire` (11291afa9) : `purge_turns(older_than_days = -1)`. -1 vaut le TTL de Turn (30 jours). Il rend `{expired, older_than_days, before}`.
- `memoire-chain` (3c2000eda) :
  - `get_slice(chemin | run+turn)`, `list_slices(run)`, `trace_turn(run, turn)` ;
  - le L1 porte `run`, `from_turn` et `to_turn`, et `recall_apports` les rend.

## Ce qu'il faut faire

1. `POST /ide/turns/read` appelle `read_turns`. Un tour `gone` se complète depuis le fichier du chat (`from_chat_file`).
2. Le rangeur appelle `purge_turns` à la fermeture d'une tranche, et le dit dans l'état du rangement.
3. `GET /ide/l1` trie par le `from_turn` du L1, au lieu de le relire dans le texte de la section.

## Comment le prouver

Un témoin rouge par point :
- un tour `gone` est rendu depuis le fichier ;
- une tranche close appelle `purge_turns` ;
- un L1 dont la section ne dit pas ses tours se trie quand même.

## Taille et blocage

Une demi-journée. Il vient après le raisonnement borné.
