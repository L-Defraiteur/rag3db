# La commande en fond — page courte, avant le code

*Arbre principal, 10 octobre 2026. Chantier A du plan de reprise
(`8-octobre-2026-16h29/orchestration/01-plan-de-reprise.md`), vision du produit code §0.
Rien n'est codé : la page attend l'accord de l'orchestration.*

## Ce que l'agent doit pouvoir faire

Lancer une commande longue (une compilation, des tests, un serveur), **reprendre la main
tout de suite**, lire ce qu'elle écrit pendant qu'elle tourne, et apprendre sa fin avec
son code de sortie, sans attendre ni deviner. C'est ce que Claude Code fait avec un
fichier temporaire par commande en fond.

## Ce qui existe déjà (lu dans le code le 10 octobre)

- `run` (`templates/tools/run.mmd`, nœud `RunCommandNode`, `dataflow/run_nodes.rs`) :
  une commande jugée par la garde (`commande.rs`, sans shell, `&&` s'arrête au premier
  échec). Les flux complets vont dans des fichiers, et le résultat rend des aperçus et le
  chemin des journaux. Délai par défaut de 60 s, **plafond de 1 800 s**, au-delà duquel
  l'enfant direct est tué.
- `run_bg` (`templates/tools/run_bg.mmd`, `%% async: true`, 600 s, même nœud) : l'agent
  reçoit tout de suite `{handle: "#run_bg-n", statut: "en cours"}`, et le résultat final
  arrive comme message dans sa boîte (`agent.rs`, `spawn_async_tool`).
- `wait` (`WaitOutputNode`) : attend la première ligne d'un journal qui correspond à un
  motif (« Trouvé » ou « Pas encore »). `journal_borne` le limite au dossier des
  journaux.

## Ce qui manque, et les défauts trouvés en lisant

1. **Le chemin du journal n'est pas rendu tout de suite.** L'accusé de `run_bg` est fait
   par la boucle (`agent.rs`), sans rien du nœud. `wait` ne peut donc pas viser le
   journal pendant que la commande tourne.
2. **Pas de « fin du journal »** : aucun verbe ne rend les N dernières lignes, ni ce qui
   est nouveau depuis le dernier regard.
3. **Pas de mort propre.** Il n'y a pas de groupe de processus : seul l'enfant direct est
   tué, et seulement au délai. Les petits-enfants (`cargo` → `rustc`, un serveur)
   survivent. Les fils asynchrones sont **joints** à la fin du run : un run qui a lancé
   une commande en fond **attend** sa fin, jusqu'à 1 800 s.
4. **Deux lancements du même programme s'écrasent** : les journaux sont nommés
   `<programme>-<pid du parent>.out/.err`.
5. **Les journaux vivent en mémoire vive et sans borne.** Ils vont dans
   `temp_dir()/rag3weaver-commandes/<run>/`, soit `/tmp`, un tmpfs sur ce poste. Il n'y
   a ni borne de taille ni ménage.
6. Le seul constructeur d'agent du produit (`chat.rs`) ne branche ni `with_events` ni
   `with_inbox` : `run_bg` y tourne en **synchrone**.

## La proposition

- **`run` prend `background: true`** (`run_bg` devient un alias, gardé pour les cartes
  existantes). La commande part dans **son propre groupe de processus** (`setsid` en
  `pre_exec`), et `run` rend tout de suite, comme résultat :
  - la poignée, par exemple `#cmd-3` ;
  - les deux chemins de journal ;
  - l'argv jugé.
  Les journaux sont nommés par la poignée (`cmd-3.out` et `cmd-3.err`), donc uniques.
- **Un verbe `tail`**, à côté de `wait`. Il prend la poignée (ou le chemin), puis soit
  `lines: N` (50 par défaut), soit `since_last: true`, qui rend ce qui s'est ajouté
  depuis le dernier `tail` de cette poignée (le décalage est gardé par poignée).
  Il rend aussi l'état de la commande : en cours, ou finie avec son code. Il est borné au
  dossier des journaux, comme `wait`.
- **La fin livrée comme message** dans la boîte : la poignée, le code de sortie (ou le
  signal), la durée, les tailles des journaux et leurs dernières lignes. Une commande
  en fond **n'a plus le plafond de 1 800 s**. Elle vit au plus le temps du run, ou le
  `timeout_s` qu'on lui donne.
- **La mort propre à la fin du run.** Toute commande en fond encore vivante reçoit
  SIGTERM sur son groupe, puis SIGKILL après un délai de grâce (5 s), et la boîte reçoit
  « tuée à la fin du run ». Le run ne l'attend plus.
- **Les journaux** :
  - ils restent bornés en accès (`journal_borne`) ;
  - ils sortent de la mémoire vive, vers `$XDG_CACHE_HOME/rag3weaver/commandes/<run>/`,
    comme les points de reprise ;
  - chaque flux est plafonné (64 Mio, puis on garde la fin en anneau, et l'aperçu le
    dit) ;
  - les dossiers de run de plus de 7 jours sont effacés au démarrage.
  Les chiffres sont à confirmer.

## Ce qu'il me faut de la boucle (session recherche, `agent.rs`)

Je ne touche ni à `agent.rs` ni au runtime. La livraison dans la boîte existe déjà par
`is_async`. Il me faut trois choses :
1. **La poignée dans le contexte du nœud** (par exemple `ctx.async_handle()`), pour
   nommer les journaux et les rendre tout de suite.
2. **Un accusé que le nœud peut remplir.** Soit l'accusé reprend un premier rendu du
   nœud (poignée et chemins), soit le nœud rend seul en mode fond, et la boucle livre
   plus tard un message qu'il poste lui-même (`bus.send_message` dans le contexte).
3. **Un crochet de fin de run** (ou la fin d'un objet de portée), pour tuer les commandes
   en fond du run au lieu de les joindre.
Plus, pour le produit : `chat.rs` doit brancher les événements et la boîte. Sinon le
fond n'existe pas pour l'utilisateur.

## Les témoins, avant le code

- `run` avec `background` rend en moins d'une seconde, pour une commande de 10 s, avec
  la poignée et des chemins qui existent.
- `tail` avec `lines: 3` rend les trois dernières lignes. `since_last` rend seulement le
  nouveau, puis rien s'il n'y a rien de neuf.
- La fin arrive dans la boîte avec le code de sortie (0, puis 3, puis tuée par signal).
- Un run fini tue la commande et ses petits-enfants : un `sh -c 'sleep 600 & wait'`
  laisse zéro processus derrière lui.
- Deux lancements du même programme ont deux journaux.
- `tail` refuse un chemin hors du dossier des journaux.
- Un flux de plus de 64 Mio est plafonné, et l'aperçu le dit.
