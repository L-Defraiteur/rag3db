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

Luciepc sous `poste lourd` (5 cœurs, `CPUQuota` 500 %), branche lucivy `mesure-profil-leger` (f89e6df). On
réindexe les **89 894 documents vivants** de la copie, soit 932 Mo de texte. La source telle quelle pèse
**4,59 Go** (×4,9 le texte) : 27 segments, 11,9 % de documents supprimés pas encore purgés.

| profil | sur disque | ×texte | positions | postings | dictionnaire | texte stocké | indexé en |
|---|---|---|---|---|---|---|---|
| `positions` (l'actuel) | 4,39 Go | ×4,7 | 1,65 Go | 1,82 Go | 463 Mo | 461 Mo | 73 à 82 s |
| `segment_dictionary` (sfx v3) | 4,49 Go | ×4,8 | 1,64 Go | 2,39 Go (dictionnaires compris) | — | 461 Mo | 75 à 95 s |
| `no_positions` | **1,44 Go** | **×1,5** | 0 | 516 Mo | 464 Mo | 461 Mo | 47 à 57 s |

Requêtes du banc, médiane de cinq, avec un `HighlightSink` :

| requête | `positions` | `segment_dictionary` | `no_positions` |
|---|---|---|---|
| 1 mot, contains d1 | 146 à 168 ms | 337 ms | 967 à 975 ms |
| 1 mot court (`commit`), d1 | 308 à 313 ms | 324 ms | 1 680 à 1 702 ms |
| 1 mot, d0 | 9 à 10 ms | 13 ms | 612 à 636 ms |
| 2 mots, contains d1 | 71 à 82 ms | 82 ms | 1 066 à 1 117 ms |
| 2 mots, parse | 57 à 61 ms | 141 ms | 1 336 à 1 470 ms |
| 4 mots, contains d1 | 35 ms | 86 ms | **4 398 à 5 478 ms** |
| 4 mots, parse | 340 à 358 ms | 1 387 ms | 2 663 à 2 818 ms |
| phrase entre guillemets | 5 à 6 ms | 16 ms | 110 à 117 ms |
| longue d1 (archiviste) | 176 à 180 ms | 262 ms | **60,1 à 62,9 s** |
| longue d0 (> 900 caractères) | 340 à 427 ms | 301 ms | 1 561 à 1 675 ms |

Les deux colonnes `positions` et `no_positions` viennent de deux passes ; les fourchettes sont leurs deux
médianes.

**Les réponses.** Le nombre de hits et de spans est le même dans les trois profils. Deux requêtes rendent 19
documents communs sur 20 : à la coupe du top 20, deux documents ont **le même score** (12.5362625 pour le mot
court, 21.706505 pour les 2 mots en `parse`), et l'ordre entre égaux dépend de la disposition des segments. Ce
n'est pas un défaut. En revanche, **les scores des requêtes `parse` diffèrent d'un ulp** d'un profil à l'autre,
pour un même document : au plus 1,9e-6, soit 7e-8 en relatif, sur 2 à 4 documents du top 20. La doc de lucivy dit
pourtant les scores identiques. Ticket : `docs/lucivy/tickets/2026-11-14-un-ulp-d-ecart-de-score-entre-profils-en-parse.md`.

**Les fils sous quota.** L'ordonnanceur part à 5 (`[scheduler] starting with 5 threads
(available_parallelism)`) : le quota de la portée le borne, comme prévu. Le relevé en voit jusqu'à 9 par
instants. Le processus a eu au plus 26 fils. Ce que le quota ne compte pas : **jusqu'à 20 fils
`docstore-compressor-thread`** (`src/store/store_compressor.rs:179`), un par segment en cours d'écriture. Ils sont
brefs et leur CPU est borné par le quota. On le dit, sans rien corriger.

### Verdict

- **On garde les positions** (décision de c7, 14 nov.). `no_positions` tient le budget (−67 %), mais pas
  `search_code` : ×5 à ×25 sur les requêtes d’un ou deux mots (×65 en distance 0), ×150 sur quatre mots, 63 s pour la requête de l’archiviste.
  Lucie acceptait « un poil lent », et ce n'en est pas un. Sans positions, chaque requête relit le texte de ses
  candidats pour produire les spans que rag3weaver demande toujours. La vérification d'une requête longue et floue
  sur le texte stocké de tous les candidats est pathologique. Ticket :
  `docs/lucivy/tickets/2026-11-14-le-chemin-sans-positions-relit-tout-pour-une-requete-longue.md`, pour un jour.
- **Le dictionnaire partagé ne rapporte que 2 %** sur ce corpus (4,39 contre 4,49 Go), et le dictionnaire par
  segment ralentit `parse` (×2 à ×4). Le défaut actuel est le bon.
- **Le budget des 30 Go par dépôt vient d'ailleurs**, et c'est déjà mesuré : les versions partagées de 1b
  (`Lucivy_ScopeContent` 1,2 Gio contre 4,0 pour `Lucivy_Scope`) et les règles d'exclusion de recherche (les
  fichiers minifiés portent 92 % des octets, page 01).

## 3. La vue filtrée : ce que coûte `allowed_ids` dans lucivy

Demande de c7, après la mesure d'ED sur la base à versions : le nœud lexical de `search_code` y passe de 874 à
6 928 ms (×8), et le masque des versions passe à lucivy un `allowed_ids` de 72 652 ids à chaque requête. Question
de Lucie : lucivy ne pourrait-elle pas tenir ces vues filtrées elle-même ?

### Ce que fait `allowed_ids` à chaque requête

Le code est dans `lucivy_core/src/sharded_handle.rs`.
- Les ids sont partagés par shard au travers du routeur : un lookup par id, sous verrou.
- Pour chaque segment des shards concernés, un bitset est reconstruit en parcourant toute la colonne `_node_id`.
  Cela se fait deux fois par requête : au préscan, puis au collecteur.
- Le BM25 est calculé sur N = la taille de l'ensemble.
- Il n'y a aucune résolution par le dictionnaire : les ids sont des u64 de colonne rapide.

### Les chiffres

Banc `lucivy_core/tests/test_filter_cost.rs` (branche lucivy `mesure-profil-leger`, dd8ce19), à luciepc sous
`poste lourd` (5 cœurs). Il tourne sur une copie de `1b-copie/migree-20261114-1641/…/Lucivy_ScopeContent`, avec
72 652 ids tirés parmi les documents vivants. La copie en compte justement **72 652** : le masque sans filtre
admet tout l'index.

Les parties du filtre :
- construire l'ensemble : 1,0 ms ;
- le partager par shard : 4,5 à 6,9 ms ;
- les bitsets des 14 segments : 1,2 à 1,3 ms par passe.

| requête (banc de la partie 2) | sans filtre (ms) | avec `allowed_ids` (ms) | écart (ms) |
|---|---|---|---|
| 1 mot, contains d1 | 26,5 | 33,9 | 7,4 |
| 1 mot court, contains d1 | 73,0 | 79,5 | 6,4 |
| 1 mot, contains d0 | 4,1 | 12,9 | 8,8 |
| 2 mots, contains d1 | 26,2 | 36,0 | 9,9 |
| 2 mots, parse | 21,5 | 29,7 | 8,2 |
| 4 mots, contains d1 | 10,5 | 20,1 | 9,6 |
| 4 mots, parse | 59,6 | 67,6 | 8,0 |
| phrase entre guillemets | 2,9 | 12,1 | 9,2 |
| longue d1 (archiviste) | 84,4 | 93,2 | 8,8 |
| longue d0 (> 900 caractères) | 362,4 | 371,0 | 8,6 |

Médianes de cinq courses. Les nombres de hits sont les mêmes avec et sans filtre.

### Avec les vrais ensembles d'ED

ED a donné les offsets que `allowed_ids_for` passe à lucivy (`ed-mesure-2a2/offsets/`, copiés avant usage).
Même banc, f89e6df :

| requête | sans filtre | masque sans filtre (72 652, tout l'index) | src/include/ (8 568) |
|---|---|---|---|
| 1 mot, contains d1 | 27 à 28 ms | 40,3 ms (+12,1) | 25,9 ms (0 hit) |
| 1 mot court, d1 | 79 à 80 ms | 90,6 ms (+10,8) | 17,7 ms (−61) |
| 1 mot, d0 | 4 à 5 ms | 15,4 ms (+10,2) | 6,9 ms (0 hit) |
| 2 mots, contains d1 | 34 à 38 ms | 45,8 ms (+12,2) | 28,5 ms (0 hit) |
| 2 mots, parse | 24 à 27 ms | 42,4 ms (+15,5) | 21,0 ms |
| 4 mots, contains d1 | 12 à 14 ms | 27,0 ms (+13,5) | 14,5 ms (0 hit) |
| 4 mots, parse | 73 à 80 ms | 77,6 ms | 51,3 ms (−22) |
| phrase entre guillemets | 4 ms | 13,8 ms (+10,0) | 6,7 ms (0 hit) |
| longue d1 | 91 à 101 ms | 101,2 ms (+10,5) | 89,1 ms |
| longue d0 | 384 à 406 ms | 395,0 ms (+11,3) | 381,1 ms |

- Le vrai ensemble complet coûte **10 à 15 ms** par requête. Le partage par shard monte à 8 ms : les offsets sont
  triés, donc moins bien répartis que le tirage au hasard.
- L'ensemble de src/include/ coûte moins de 3 ms, et rend la requête **plus rapide** qu'une recherche sans filtre
  dès qu'il exclut beaucoup de candidats (−61 ms pour le mot court) : le filtre est aussi un préfiltre.

### Verdict

**Dans lucivy, le filtre coûte 6 à 15 ms par requête.** Le ×8 vient d'ailleurs. La sonde d'ED dans le nœud
lexical (copie migrée, sans filtre) le place ainsi :
- les racines : 20 ms ;
- le masque `derived_references` : 902 ms ;
- **la conversion de `_uuid IN [72 652]` en offsets par rag3db (`resolve_filter_to_ids`) : 5 001 ms**.

Cela fait 5 923 des 6 928 ms. Le reste comprend les ~9 ms de lucivy et les deux passes du mode `auto`.

- **Le remède est de poser les offsets une fois par (couche, rang) côté rag3weaver**, au lieu de résoudre un `IN`
  d'uuid à chaque appel. Arbre principal prend le court-circuit sans filtre.
- **`define_view` n'est pas écrite.** La forme est convenue avec ED : une vue nommée en mémoire, un bitset par
  segment recalculé pour les seuls segments neufs, `update_view(add, remove)`, l'intersection avec un
  `allowed_ids`, et un score égal au bit près à celui d'`allowed_ids`. Elle ne gagnerait que les 6 à 10 ms par
  appel. On y revient si ces millisecondes comptent une fois les 5,9 s retirées.

## 4. Le dictionnaire qui garde les textes des documents effacés

Ticket : `docs/lucivy/tickets/2026-11-14-le-dictionnaire-garde-les-textes-des-documents-effaces.md`. Sur
`Lucivy_Scope`, après la purge de 8 809 documents et la compaction, le dictionnaire reste à 513,8 Mo, contre
420,1 Mo pour un index neuf des survivants.

### La cause : deux défauts

- **Le `.gmap` d'un segment fusionné était l'union des `.gmap` d'entrée.** L'id d'un terme dont tous les documents
  sont supprimés y restait, avec des postings vides, et rien ne savait qu'il était mort.
- **La compaction ne fusionnait que les groupes d'au moins deux segments.** Un shard réduit à un segment gardait ses
  supprimés pour toujours. Le test à 4 shards de `maintain` ne le voyait pas, parce que chaque shard y avait
  plusieurs segments.

### Pièce 1 : la fusion lâche les ids morts

PR lucivy #37. La fusion ne garde que les ids qui ont encore un document vivant, et un segment seul qui porte des
supprimés est réécrit.
- Témoin : une fixture à 1 shard, 400 documents, 200 purgés. Sur main, le segment garde ses 200 supprimés et son
  `.gmap` nomme 1 231 ids. Sur la branche, il en nomme 631, comme un index neuf, avec les mêmes hits et les mêmes
  scores au bit près.
- **Le coût ne touche que les fusions de segments qui portent des supprimés** : +12 % sur une compaction de
  maintenance (55 s contre 49), et l'indexation ne bouge pas (+0,6 à +1,8 %, sous le bruit). Le souhait d'une
  réduction est un ticket : `2026-11-14-le-pre-passage-des-ids-vivants-relit-les-postings.md`.

### Ce que pèse un dictionnaire

Mesuré avec `test_dictionary_anatomy` sur l'index neuf de recherche : 771 Mo, dont 52 % de dictionnaire.
- **Il ne pèse pas par le nombre de segments** : il y en a un par shard, et la compaction de 27 à 4 segments l'a
  laissé à 513,8 Mo.
- **Il pèse par le vocabulaire.** Le champ `content` compte 5,63 M entrées distinctes : 3,47 M fragments de 8
  octets au plus, avec le débord sur le token suivant, et 2,16 M mots. Cela fait 45,7 Mo de texte d'entrées pour
  73 Mo de texte stocké.
- **Chaque octet d'entrée devient un suffixe** : environ 46 M clés FST avec leurs parents, à 6 octets chacune
  (`.sfx` 282 Mo), plus environ 14 octets par entrée en `.termtexts` (80 Mo). Les mots longs ne pèsent rien : moins
  de 4 % des suffixes.
- Les doublons de la base pleine abaissaient le rapport (×4,9 contre ×10,6 à neuf), sans rien économiser.
- Les leviers sont la longueur minimale de suffixe, la taille du débord et `DEFAULT_MAX_TOKEN` = 8. Ils changent ce
  que `contains` trouve : rien sans décision.

### Pièce 2 : la compaction filtrée du dictionnaire

Elle fusionne toutes les générations en une, en ne gardant que les ids que l'union des `.gmap` vivants cite. Aucun
id n'est renuméroté et aucun segment n'est réécrit. Elle tourne sous le verrou des écrivains, et refuse s'il reste
des documents non validés ou des paires en attente. En cours.
