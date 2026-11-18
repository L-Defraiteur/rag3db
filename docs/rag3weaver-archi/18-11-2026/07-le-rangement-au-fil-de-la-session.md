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
