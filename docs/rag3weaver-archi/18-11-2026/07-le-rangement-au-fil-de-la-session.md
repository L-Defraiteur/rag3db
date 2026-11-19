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
3. **La marque, un seul rédacteur** (forme validée par c7 le 18 novembre). La trace reste le
   `ChatFile`. La marque vit à côté, dans `chats/<backend>.rangement.json` :
   - `archived_through_turn` ;
   - `slices` : `{from_turn, to_turn, ok, error?}` ;
   - `last_error`.

   Son seul rédacteur est le fil de rangement. Le rangement de fond ne prend donc ni le verrou
   des sessions ni la réécriture du `ChatFile`. Un rangement raté garde la marque où elle était :
   la tranche repartira, avec la suivante.
3 bis. **La marque n'avance qu'après le commit** (contrat avec mémoire, 18 novembre). Le rangement
   rend `{report, archived_through_turn}`. L'hôte n'écrit `archived_through_turn` dans le
   `ChatFile` que si deux conditions sont réunies :
   - les écritures de la passe sont validées : la Source de tranche, les L1 et les fiches ;
   - le rapport ne porte aucune erreur.

   Un appel `archive` seulement terminé ne suffit pas. Une passe qui échoue, ou dont le rapport
   porte une erreur, laisse la marque où elle était, et la tranche se refait, avec la suivante.
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
9 quinquies. **La chaîne inverse** (Lucie) : d'un tour, remonter à sa tranche et à son run,
   puis aux L1 que l'archiviste en a tirés.
   - **La Source de tranche** porte `run`, son intervalle (`from_turn`, `to_turn`),
     `slice_summary` et le rapport de l'archiviste.
   - **La tranche d'un tour se calcule depuis l'intervalle** : c'est la Source de tranche du même
     `run` dont l'intervalle contient le tour. Les `Turn` ne reçoivent pas d'identifiant de
     tranche après coup. C'est mon choix, pour deux raisons :
     1. une entité en ajout seul le reste, sans exception ;
     2. une tranche ratée puis rangée avec la suivante, ou une tranche refaite, ne laisse aucun
        `Turn` pointer vers une tranche qui n'existe pas ou plus.

     Les intervalles des tranches réussies d'un run ne se chevauchent pas : l'hôte n'avance la
     marque qu'après un rangement réussi. La réponse est donc unique. Un tour sans tranche est
     « pas encore rangé ».
   - **Les L1 de la tranche** viennent de `cited_by` sur la Source de tranche (gabarit :
     mémoire). L'hôte offre `trace_turn(run, turn)` (un seul nom avec l'outil du gabarit, accord du 18 novembre ; à côté, `get_slice` et `list_slices` du gabarit) : la tranche, son run et ses L1, ou « pas
     encore rangé ». C'est en lecture seule, offert au chat, à l'archiviste et à l'IDE.

9 ter bis. **Les routes de l'IDE et du tty**, par run et non par tâche, pour servir le chat et
   l'équipier (noms définitifs, pris de tty le 18 novembre) :
   - `GET /ide/l1?backend=<id>&run=<run>` : les L1 d'un run dans l'ordre des tours (`apport`,
     `fiche`, `section`, `run`, `from_turn`, `to_turn`) ;
   - `POST /ide/turns/read`, avec `{apport}` ou `{run, from, to}` : `read_turns`, chaque tour avec
     son état, `indexed`, `from_chat_file` ou `gone`.

9 quater. **Les tours cherchables par le temps, et les appels d'outils filtrables** (Lucie :
   revoir « ce qui a mal marché », vérifier qu'un outil a bien été appelé la veille).
   - **Ce que l'hôte écrit pour chaque tour** : un `Turn` avec `at`, `run`, `turn` et `role`.
   - **Pour chaque appel d'outil** : `tool_name`, `params` (le JSON des arguments, borné),
     `result` (le texte rendu, borné) et `status` (`ok`, `error`, `refused`, `interrupted`, lus
     sur le retour : `ok: false`, `error`, un refus de la porte ou des bornes du pas, un appel
     orphelin refermé).
   - **La garde de l'exclusion** vaut aussi pour `params` et `result`.
   - **Le filtre des motifs de secrets** (clés, jetons, mots de passe) passe sur `text`,
     `params` et `result` avant l'écriture d'un `Turn`. Il tourne **dans le gabarit**, à
     l'écriture, comme la garde de l'exclusion. L'hôte écrit les `Turn` par l'outil `put_turn` du
     gabarit (run, turn, seq, at, role, text ; pour un appel d'outil : tool_name, params, result,
     status ; il rend la ligne écrite, avec ce qui a été masqué ou coupé ; mémoire, page du 18,
     section g), si bien qu'un seul endroit couvre tous les
     rédacteurs (l'hôte, les hooks Claude Code, l'archiviste). L'hôte ne garde pas de liste de
     motifs.
   - **`search_turns` passe les filtres** `since`, `until`, `role`, `tool_name` et `status` au
     rappel du memory (forme du gabarit : mémoire).
   - **Les bornes** sont celles du gabarit memory, qui fait foi (accord avec mémoire, 18
     novembre) : `params` à 4 000 caractères, `result` à 8 000. Au-delà, le texte finit par
     « … » suivi de sa taille d'origine. Le dense découpe en morceaux de 1 500 : ces bornes
     limitent la place et ce qu'on relit, pas l'embarqueur. L'hôte les lit dans le gabarit et ne
     les redéclare pas.

9 ter. **Suivre la chaîne à la main** (Lucie) : un L1 mène à sa tranche, puis à son intervalle
   de tours, puis au tour exact, tant que le TTL ne l'a pas effacé.
   - **`read_turns` prend `(run, from_turn, to_turn)`.** Il lit l'entité `Turn` du memory si elle
     est là, sinon le `ChatFile`. Il rend chaque tour avec son état :
     - « indexé » ;
     - « hors index, lu dans le fichier » ;
     - « disparu », si le fichier est parti aussi.
   - **Un L1 rendu par l'hôte porte sa plage en clair** (`run`, `turns a-b`), pour qu'on l'ouvre
     sans deviner.
   - **Le tty et l'IDE** montrent cette plage et ouvrent les tours par `read_turns`. Le tty le
     fera à son tour ; l'IDE, par archi, avec ce lot.

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
   4. **Ce que la trace contient** (tranché par Lucie, 18 novembre au soir). La trace porte le
      code lu et des sorties de commandes. `memory.index_turns` est **allumé par défaut**. La
      seule garde est l'exclusion des fichiers du dépôt : un contenu d'un fichier exclu par les
      règles d'exclusion est remplacé par « exclu du dépôt » avant l'écriture (forme décidée par
      mémoire).

10. **La preuve en plus.** Faux modèle : le pas de l'archiviste appelle `read_turns` sur des
   tours d'avant sa tranche, puis range. Ses bornes l'arrêtent proprement. Débrayé, le graphe
   seul tourne.

Cela agrandit le lot 1 :
- le pas et `read_turns` : une demi-journée ;
- l'indexation des tours au fil du run (plein texte par tour, dense par tranche, TTL) et `search_turns` : 1,5 jour.

Le lot 1 passe à 4 jours au lieu de 1,5, avec les champs des tours et les filtres (une demi-journée).

## Les signatures, accordées avec mémoire (18 novembre au soir)

- **`put_turn(run, turn, seq, at, role, text, tool_name?, params?, result?, status?)`** rend
  `{written, masked, cut}`.
  - `run` vaut `<chat>:<backend_id>` pour le chat, `<tâche>:<parent>` pour l'équipier.
  - `turn` est le numéro du tour utilisateur. `seq` est le rang du message dans le tour : 0 pour
    le message utilisateur, puis l'assistant, puis chaque retour d'outil.
- **`archive_slice(run, from_turn, to_turn, context, contenu?)`** rend `{report,
  archived_through_turn}`.
  - **Une seule forme d'entrée, écrite dans le gabarit** (accord du 18 au soir). Le script
    `render_turns` fait, des `Turn` de l'intervalle, le texte de la tranche : un `## turn <n>`
    par tour, un `### <role> · <heure>` par message, et pour un appel d'outil son nom, son
    statut, ses paramètres et son résultat, bornés et masqués. `archive_slice` range ce texte et
    l'écrit comme Source de tranche.
  - L'hôte n'écrit ni ne passe aucun texte : il appelle `put_turn` à chaque fin de tour, puis
    `archive_slice(run, from_turn, to_turn, context)`.
  - Avec `index_turns` éteint, l'hôte ne ferme aucune tranche et le dit une fois dans l'état du
    chat. Sans `Turn` qui couvre l'intervalle, `archive_slice` refuse en le disant.
  - Le banc passe la même forme dans `contenu`.
  - `archived_through_turn` vaut `to_turn` quand le commit est fait et le rapport sans erreur,
    sinon `from_turn - 1`.
  - La Source de tranche est `sources/<run en slug>-turns-<a>-<b>.md`.

## Lot 2 : la compaction (conception du 19 novembre)

1. **Quand.**
   - Au début d'un tour du chat, si le dernier appel au modèle a dépassé `memory.compact_at`
     (0,8) de la fenêtre du modèle, l'hôte compacte d'abord. La personne attend ce moment-là,
     comme dans Claude Code. Il n'y a pas de fil à part qui réécrirait la session.
   - Sur demande : `POST /ide/chat/compact {backend}` (la commande `/compact`).
   - L'équipier n'est pas concerné : chaque pas repart déjà d'un fil neuf.
2. **Les refus.**
   - Pendant un tour en vol (`busy`) : « un tour est en cours ».
   - Pendant un rangement de fond en vol : « un rangement est en cours ».
   - Le tour automatique qui trouve un refus passe sans compacter ; le suivant réessaie.
3. **Les étapes, dans l'ordre.**
   1. Les messages pas encore écrits partent par `put_turn` : c'est le ticket du put_turn raté,
      dans ce lot. Le fichier de rangement tient `turns_written_through` (turn, seq).
   2. La tranche ouverte (marque + 1 jusqu'au dernier tour) se range en attendant
      (`archive_slice`). Un échec arrête la compaction : `last_error`, et la session ne change
      pas.
   3. `compose_restart(sources, identity, task_fiche, budget_tokens)`, avec `sources` = la liste du
      fichier de rangement. Chaque `source` rendue par `archive_slice` n'y entre que si
      `archived_through_turn == to_turn`.
   4. La session repart sur trois éléments :
      - le message système, réécrit à chaque tour comme aujourd'hui ;
      - un tour « contexte de reprise » (le texte de `compose_restart`, marqué
        `metadata.compaction`) ;
      - la queue brute, soit les `memory.keep_turns` derniers tours (2 par défaut).
4. **La numérotation survit à la compaction.**
   - Le fichier de rangement tient `turn_offset`, le nombre de tours retirés de la session.
   - Le tour n d'une session compactée est le tour `turn_offset + n` du run, et le tour de reprise
     ne compte pas.
   - `put_turn` reste immuable par `(run, turn, seq)` : rien n'est réécrit.
5. **Le filet `ContextOverflow`** va au lot 3, avec les outils de lecture.

## Lot 3 : le filet `ContextOverflow` (conception du 19 novembre)

1. **Quand.** Un tour du chat rend `llm context overflow` : le fournisseur a refusé l'entrée,
   l'hôte l'a vue (`ContextOverflow`).
2. **Le tour qui a débordé reste vrai.** Ses messages, déjà dans la session, partent par
   `put_turn` sous leur numéro N. La compaction les range avec la tranche ouverte, puis les
   retire de la session comme les tours d'avant : `turn_offset` les compte.
3. **Le nouvel essai est un tour neuf, N+1**, avec le même message. Rien ne réécrit une ligne
   `(run, turn, seq)` déjà écrite (`put_turn` est immuable). L'historique du run dit ce qui s'est
   passé : le tour N a débordé, le tour N+1 l'a rejoué.
4. **Une fois seulement.** Un second débordement après la compaction est une panne nommée
   (« context overflow after a compaction »), rendue telle quelle. Une compaction refusée (un
   rangement en vol, aucun memory) rend le débordement d'origine, avec le refus.
5. **L'équipier n'en a pas besoin** : chaque pas repart d'un fil neuf. Son débordement reste un
   pas en échec, compté par `max_failed_steps`.

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
