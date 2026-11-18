# Le rangement au fil de la session, côté hôte — conception du 18 novembre 2026

C'est la demande de Lucie, transmise par c7 le 18 novembre au soir : « il faut le faire en même
temps qu'on débogue le reste, pas le garder de côté pour toujours ; vu que ce sont des fenêtres de
jetons, on peut le faire bien avant les 80 % : l'archiviste à chaque tranche de jetons, et basta ;
au moment de la compaction, tout est déjà rangé sauf la dernière tranche ».

C'est le « rangement au fil de la session » du 15 novembre, jamais codé côté hôte. La page de
mémoire fixe l'archiviste en deux étapes et les ingrédients déclarés : cette page-ci ne dit que la
part de l'hôte, et se relira quand elle sera posée.

## Ce qui existe sur master

- **Le gabarit `memory`** :
  - l'outil `archive` (`graphs/archive.mmd`) prend un document Markdown, garde le L0 et range
    chaque section en L1. Ses paramètres : `titre`, `contenu`, `chemin`, `origine`, `jour`,
    `context`, `allow_create`… ;
  - l'outil `compose_restart` compose le contexte de reprise dans un budget, à partir de
    `sources` (les L0 de la session), `identity`, `task_fiche` et `budget_tokens`.
- **L'hôte** (`daemon_host/chat.rs`) :
  - `ChatEntry` et `ChatFile` (le manifeste, la session, `last_call` : la jauge) ;
  - un tour à la fois (`busy`), et l'écriture du fichier du chat à la fin de chaque tour ;
  - aucun appel à `archive` ni à `compose_restart`, et aucun seuil sur la jauge.
- **Les hooks Claude Code** (`claude_code_hook.rs`) rangent la compaction de Claude Code
  (PostCompact). C'est un autre chemin, qui ne change pas ici.

## Conception

1. **La tranche.** L'hôte compte les jetons de la trace du run de la conversation (la session :
   les tours, leurs appels d'outils et leurs retours, estimés à ceil(caractères / 4) comme
   `compose_restart`), depuis la marque « rangé jusqu'au tour T ». Quand le compte passe
   `memory.slice_tokens` (la fenêtre de l'archiviste, 50 000 par défaut), la tranche T+1..T' se
   ferme à la fin du tour en cours : jamais au milieu d'un tour.
2. **L'archiviste en fond.** La tranche fermée part, en Markdown (un titre par tour), vers
   l'archiviste du gabarit `memory` de la mémoire déclarée par le backend (`bilans.memoire`) :
   l'étape 1 (extraction structurée), puis l'étape 2 (rangement par sujet avec rappel), selon la
   page de mémoire. Pour l'outil `archive` d'aujourd'hui :
   - `origine` vaut « tranche » ;
   - `chemin` vaut `sources/<jour>-<chat>-tours-<T+1>-<T'>.md` ;
   - `context` porte le résumé des tranches d'avant.

   Le tour suivant ne l'attend pas : un fil à part, un seul à la fois par conversation. Un nouveau
   tour pendant un rangement le laisse finir.
3. **La marque, un seul rédacteur.** Le `ChatFile` gagne `archived_through: Option<usize>` (le
   dernier tour rangé) et `slices: Vec<{from, to, source, ok}>`. Seul l'hôte les écrit, sous le
   verrou des chats, à la fin d'un rangement réussi. Un rangement raté garde la marque où elle
   était : la tranche repartira, avec la suivante.
4. **La compaction ne range que la dernière tranche.** À 80 % de la jauge (`last_call` sur la
   fenêtre du modèle, réglage `memory.compact_at`, 0,8) ou sur `/compact`, l'hôte range la
   tranche ouverte (T+1..maintenant), en attendant cette fois, puis appelle `compose_restart`.
   Il lui passe les `sources` de toutes les tranches et les ingrédients déclarés par la page de
   mémoire. La session repart sur ce contexte, plus la queue brute.
5. **Les refus et le filet.**
   - Une compaction demandée pendant un tour en vol est refusée en le disant (« un tour est en
     cours »), comme un second tour.
   - Un `ContextOverflow` du fournisseur déclenche la même compaction une fois, puis rejoue le
     tour. S'il revient, c'est une panne nommée.
6. **Pour le chat et l'équipier.**
   - Le chat suit la session du backend.
   - L'équipier repart déjà d'un fil neuf à chaque pas. Sa trace à ranger est la session de la
     tâche, qui garde chaque pas pour l'affichage. Ses tranches suivent les mêmes règles.
7. **Les réglages** sont des clés neuves en anglais, sous `memory` dans le manifeste du backend
   (pas dans le code) : `slice_tokens`, `compact_at`, `archive_in_background`, plus les
   ingrédients de la page de mémoire. Le banc les joue.

## L'archiviste qui lit ce qui lui chante (ajout de Lucie, 18 novembre au soir)

Si on le branche, l'archiviste peut relire :
- la trace de la session hors de sa tranche ;
- la mémoire (`recall_fiches`, `get_fiche`, `recall_apports`) ;
- le code (`search_code`, `read_file`).

La part gabarit est décrite par mémoire. La part hôte :

8. **Le rangement d'une tranche peut être un pas d'agent du socle**, au lieu d'un seul graphe.
   - L'étape déclarée par le gabarit dit `mode: "step"`, sa liste d'outils et ses bornes. Elle
     est débrayée par défaut ; débrayée, l'étape reste le graphe.
   - L'hôte la joue comme un pas de l'équipier : `Step` et ses bornes de S1
     (`step_context_tokens`, `max_tool_calls_per_step`, `max_model_calls_per_step`, propres à
     l'archiviste : `memory.archivist_*`), plus une durée bornée (`memory.archivist_step_s`, 600
     par défaut).
   - Seuls les outils déclarés sont offerts : lecture seule, sauf les écritures de la mémoire
     que l'étape 2 fait déjà.
   - Il y a toujours un rangement à la fois par conversation. Le pas finit par `step_done`, ou
     par sa borne, qui est un échec nommé : la marque reste où elle était.
9. **`read_turns`, un outil de l'hôte.** Il lit la trace de la conversation par intervalle de
   tours (`from`, `to`, et `chat` : la conversation de la tranche par défaut), rendue comme la
   tranche, bornée en caractères, en lecture seule. Il n'est offert qu'au pas de l'archiviste, et
   à aucun autre agent sans déclaration.
9 bis. **`search_turns`, chercher avant de lire : les tours eux-mêmes sont indexés au fil du run**
   (Lucie : « pas seulement relire, mais chercher » ; « les L0, les vrais tours, doivent quand
   même être indexés au fil de l'eau, avec un TTL »).
   - **La Source de la trace.** La trace du run entre dans le memory déclaré comme une Source,
     avec une section par tour (l'identité d'un Apport : la source et le tour). Son chemin est
     `runs/<chat>/<jour>.md`.
   - **Le plein texte, à chaque fin de tour** : la section du tour s'écrit et s'indexe en plein
     texte. C'est local, par le writer du memory.
   - **Le dense, par lot, à la fermeture de la tranche** : un appel au service d'embarquement de
     la vue pour toute la tranche, jamais un embarquement par tour.
   - **Le TTL** (`memory.l0_ttl_days`) purge les entrées d'index et les vecteurs des tours
     anciens. Le `ChatFile` reste sur disque, et les L1 gardent leur intervalle de tours en
     citation : `read_turns` marche toujours après la purge.
   - **Les tours ne sortent pas dans le rappel par défaut de la mémoire** (Lucie) : il faut les
     demander explicitement. `search_turns`, côté hôte, n'est que le rappel du memory avec ce
     paramètre posé (`include_turns` ou une portée `turns` : la forme est à mémoire), filtré sur
     la trace du run. Il rend des intervalles de tours avec un extrait. L'archiviste cherche, puis
     lit par `read_turns`.

   **Ce qui me gêne, et que la page de mémoire doit trancher.**
   1. **Le L0 du gabarit est immuable, écrit une fois** (`archive` : « immuable, écrite une fois
      les sections décidées »). Une Source qui gagne une section à chaque tour est donc une autre
      espèce de Source : ajout seul, jamais réécrite. Il faut la nommer dans le gabarit (une
      origine `run_trace` ?), sinon les deux contrats se contredisent.
   2. **Le writer.** Le memory n'a qu'un rédacteur. L'écriture du tour, le rangement en fond et
      l'archiviste en pas s'y succèdent : l'hôte les met en file sur le même writer, et une
      écriture de tour ne doit jamais attendre un rangement de plusieurs minutes. Les tours
      passent donc avant le rangement, qui se fait par petites transactions.
   3. **Le coût au fil du run.** Le plein texte local à chaque fin de tour ne me gêne pas : une
      section par tour, quelques Ko. Le dense par tranche, par le service de la vue, ne me gêne
      pas non plus : un lot toutes les 50 k jetons. Mais il doit attendre sa fenêtre comme
      l'équipier quand le service est saturé, sans bloquer le tour.
   4. **Ce que la trace contient.** Les retours d'outils portent le code lu et des sorties de
      commandes. Les indexer dans une mémoire durable, même sous TTL, doit rester débrayable
      (`memory.index_turns`, éteint tant que Lucie ne l'allume pas pour un dépôt). Un fichier
      exclu par les règles d'exclusion ne doit pas y entrer non plus.

10. **La preuve en plus.** Faux modèle : le pas de l'archiviste appelle `read_turns` sur des
   tours d'avant sa tranche, puis range. Ses bornes l'arrêtent proprement. Débrayé, le graphe
   seul tourne.

Cela agrandit le lot 1 :
- le pas et `read_turns` : une demi-journée ;
- l'indexation des tours au fil du run (plein texte par tour, dense par tranche, TTL) et `search_turns` : 1,5 jour.

Le lot 1 passe à 3,5 jours au lieu de 1,5.

## La preuve, rouge d'abord (demain)

- **Faux modèle, `slice_tokens` petit.** Après assez de tours, l'outil `archive` du memory du
  banc reçoit une tranche ; le `ChatFile` dit `archived_through`. Le tour suivant n'a pas attendu
  le rangement.
- **Compaction.** Seule la tranche ouverte part, puis `compose_restart` est appelé avec toutes les
  `sources`, et la session repart plus courte.
- **Refus et filet.** Une compaction pendant un tour est refusée. Un `ContextOverflow` simulé
  déclenche une compaction puis le rejeu.
- **Les e2e.** e2e_memoire_* et e2e_archiviste avec le vrai gabarit, et e2e_equipier sous
  Vertex.

## Taille

En jours d'agent, trois lots :
1. **La tranche, l'archiviste en fond et la marque** : 1,5 jour.
2. **La compaction** (seuil, `/compact`, `compose_restart`, ingrédients) : 1,5 jour, après la
   page de mémoire.
3. **Le refus, le filet `ContextOverflow` et l'équipier** : 1 jour.

**Questions ouvertes, qui ne bloquent rien.**
- Le memory visé, quand le backend n'en déclare pas : rien n'est rangé, et l'IDE le dit déjà
  (« ce dépôt ne déclare pas de mémoire »).
- Le fil de fond partage le modèle et son quota avec le chat : un rangement ralenti par un 429
  attend sa fenêtre, comme l'équipier.
