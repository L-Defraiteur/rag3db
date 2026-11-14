# Les fils de lucivy et le profil léger (mesures, 14 novembre 2026)

Deux lots demandés par c7, mesure d'abord, pas de code tant qu'elle n'a pas parlé :
- **les fils** : Lucie a vu des rouges sous partage et a évoqué une variante tokio async de lucivy, pour qu'il
  prenne moins de fils hors wasm ;
- **le profil léger** des index de couche (budget de 30 Go par dépôt) : `positions: false` et le dictionnaire
  partagé.

La première partie est établie. La seconde est un protocole : ses chiffres viendront quand une place de mesure
s'ouvrira à luciepc.

## 1. Les fils

### Ce que lucivy démarre

| pool | taille par défaut | borne | où |
|---|---|---|---|
| ordonnanceur luciole (`scheduler-#`) | `available_parallelism` | `LUCIVY_SCHEDULER_THREADS` | `luciole/src/scheduler.rs:205` |
| écrivains par index (shard) | `min(available_parallelism, 16)`, 1 sous wasm | `LUCIVY_WRITER_THREADS` | `lucivy_core/src/handle.rs:67`, `src/indexer/indexer_actor.rs:47` |
| recherche multi-fils (`lucivy-search-`) et préchauffe (`lucivy-warm-`) | seulement à la demande (`Index::set_multithread_executor`, warming) | le nombre passé | `src/index/index.rs:320`, `src/reader/warming.rs:182` |

- **`RAYON_NUM_THREADS` ne borne rien dans lucivy.** Lucivy ne se sert pas du pool rayon global : ses pools rayon
  sont construits avec un nombre explicite (`src/core/executor.rs:23`).
- **`available_parallelism` suit le quota CPU du cgroup** (`cpu.max`). Vérifié au portable avec un programme
  d'une ligne compilé par rustc 1.98.1, lancé par `systemd-run --user --scope -p CPUQuota=…` :

  | quota | `available_parallelism` |
  |---|---|
  | aucun | 32 |
  | 500 % | 5 |
  | 300 % | 3 |
  | 150 % | 1 (division entière) |

  Conséquence : **sous `poste lourd`, l'enveloppe du poste (3a654c4d9) borne déjà lucivy.** Un lourd de luciepc
  reçoit 500 %, donc l'ordonnanceur et les écrivains partent à 5 fils. Ce n'est pas `RAYON_NUM_THREADS`
  qu'exporte l'enveloppe qui borne lucivy, c'est le `CPUQuota` de sa portée.

### L'hôte du portable au repos

Relevé du 14 novembre sur `rag3weaver-daemon.service` (MainPID 2943666) : **359 fils**, dont **32
`scheduler-#`** pour lucivy. Les autres appartiennent à rag3weaver et au moteur :

| famille | fils | d'où |
|---|---|---|
| `demon-#` | 168 | 8 fils du serveur de l'hôte, et 160 du moteur : cinq bases ouvertes × 32 fils, `max_num_threads` = 0 (identifiés par G, `docs/optimiseur/14-11-2026/03-une-enveloppe-pour-l-hote.md`) |
| `rag3weaver-graphes` | 34 | les graphes de l'hôte |
| `teammate-journa…` | 33 | le journal de l'équipier |
| `scheduler-#` | 32 | lucivy |
| le reste | ~92 | relais de l'IDE, journaux, montres |

Le lot de G, `RAG3WEAVER_DB_THREADS`, borne les pools du moteur.

### L'hôte n'a pas de quota

`rag3weaver-daemon.service` a `cpu.max = max`. Sans quota, `available_parallelism` répond 32, et les pools de
lucivy comme ceux du moteur partent à la taille du poste. C'est là que les 32 fils de lucivy et les 160 du moteur
se disputent les cœurs pendant une indexation réelle. **Le remède est l'unité, pas lucivy** : la page 03 de
l'optimiseur propose un drop-in (`CPUQuota`, `CPUWeight`, `Nice`), que Lucie pose à sa main, et c'est le
domaine de G. Avec ce quota, lucivy se borne seul, sans variable.

### Conclusion provisoire

**La variante async n'a pas lieu d'être** si le quota de l'unité et la borne des pools du moteur suffisent. Lucivy
dimensionne déjà ses pools sur le CPU que le cgroup lui laisse. Changer de modèle d'exécution n'enlèverait pas
de charge : il n'enlèverait que des fils endormis, qui ne coûtent qu'un peu de mémoire de pile. Si une borne plus
basse que le quota était voulue, la plus petite coupe existe déjà : `LUCIVY_SCHEDULER_THREADS` et
`LUCIVY_WRITER_THREADS` dans l'environnement de l'unité.

À confirmer sous indexation réelle. La passe du profil léger (partie 2) relève les fils de son processus toutes
les deux secondes pendant qu'elle réindexe, et imprime la taille annoncée par l'ordonnanceur
(`[scheduler] starting with N threads (source)`, sous `LUCIVY_VERBOSE`).

## 2. Le profil léger : le protocole

### Ce que disent les options

- **`positions: false`** : les postings gardent documents et fréquences, pas les positions, et les fichiers
  annexes de positions ne sont pas écrits. Chaque correspondance est retrouvée comme documents candidats, puis
  vérifiée sur le texte stocké. **Les réponses sont les mêmes ; le coût porte sur le temps** : un span, ou une
  correspondance à travers plusieurs tokens, demande de relire les candidats. Réglé à la création, comme le
  champ `fts_positions` du manifeste de rag3weaver (défaut `true`). Aucun backend livré ne le pose à `false`.
- **Le dictionnaire partagé** (`shared_dictionary`, sfx v4) est **déjà le défaut** : rag3weaver ne pose pas
  l'option, et chaque shard tient un dictionnaire commun à ses segments. Le partage **entre versions** (entre les
  couches des worktrees) est une question de rag3weaver, pas une option de lucivy. La mesure compare le
  dictionnaire par shard au dictionnaire par segment pour dire ce que le partage rapporte.

### Ce que `search_code` exige : les spans

Toutes les recherches plein texte de rag3weaver passent par `fts_handle::search_hits_with_text`, qui pose
**toujours** un `HighlightSink`. Les spans ne servent pas qu'au surlignage : ils rattachent chaque hit à son
morceau (chunk) et donnent le bonus de titre. En mode `auto`, celui de tous les outils livrés, une requête
part sous deux formes :
- un `contains` de la requête **entière** sur `name`, `content` et `docstring`, à distance 1, ou à distance 0
  au-delà de 900 caractères ;
- un `parse` (OU de `contains` par mot) dès qu'elle a deux mots ; une phrase entre guillemets y passe par le vrai
  QueryParser.

Les cas lourds sans positions seront donc les requêtes longues de plusieurs mots : la section entière que
l'archiviste envoie à `Lucivy_Fiche`, et le prompt que le hook envoie à `recall_fiches`.

### La passe

- **L'outil** : `lucivy_core/tests/test_light_profile.rs`, branche lucivy `mesure-profil-leger` (1e5eff5), test
  ignoré. Il relit les documents stockés vivants d'une **copie** de `Lucivy_Scope`
  (`mesure-fts/scope-20261114-130846`, 4,3 Go), et les réécrit dans un index neuf par profil :
  - `positions` : le profil actuel ;
  - `no_positions` ;
  - `segment_dictionary` : positions, dictionnaire par segment.

  Les écritures sont committées tous les 20 000 documents. Ensuite, la passe attend la fin des fusions et des
  replis du dictionnaire.
- **Par profil** : le temps d'indexation, `index_report` (octets par famille : positions, texte stocké,
  dictionnaire, postings ; et le rapport index / texte), puis le banc de requêtes. Chaque requête tourne cinq
  fois, avec un `HighlightSink`. On note la première course, la médiane, le nombre de hits et de spans.
  **Les réponses doivent être les mêmes d'un profil à l'autre** ; un écart s'imprime `DIFFERENT ANSWERS`.
- **Le banc**, calqué sur `build_bm25_query` :
  - un mot long, un mot court (`commit`), un mot en distance 0 ;
  - deux mots en `contains` et en `parse` ;
  - quatre mots avec ponctuation (`fn open_fts_handle_or_reason(`) et leur `parse` ;
  - une phrase entre guillemets ;
  - une requête longue de 300 caractères en distance 1 (archiviste) ;
  - une de plus de 900 caractères en distance 0.
- **Où** : luciepc, sous `poste mesure`, dans une fenêtre que c7 ouvre. La passe du 14 novembre à 16 h 26 a été
  retirée de la file avant d'avoir démarré : elle bloquait les rejeux de la pile.

### Les chiffres

À venir, sur cette page.
