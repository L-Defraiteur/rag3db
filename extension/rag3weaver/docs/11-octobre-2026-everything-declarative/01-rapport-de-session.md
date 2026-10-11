# Everything declarative — rapport de session du 11 octobre 2026

Chantier ouvert à midi par l'orchestration, sur la demande de Lucie : « un
agent est un DAG, un DAG est un outil quand on en a besoin (dont les agents),
et les agents se parlent ». La conception est dans `03-les-agents-se-parlent.md`,
acceptée par l'orchestration. Ce qu'on sait du code est dans `02-knowledge-dump.md`.

## État

| Lot | Ce qu'il pose | État |
|---|---|---|
| 0 | les flux dans le moteur de graphes | **session recherche** : le saut (`522fa6780`) et les déclarations de flux (`feeb1932b` : `PortKind`, `Posture`, sorties requises respectées) sont sur master, relus ici ; suit l'exécuteur concurrent |
| 1 | la boucle comme graphe à ports : les points d'accroche, la parité | **sur master** (`d3aa6f281`), relu par la recherche (accord, deux corrections faites) ; lib 1388/1388, toutes les cibles compilent |
| 2 | `decide` et `route`, posés sur un port | **sur master** (`7b5cda799`), relu par la recherche (trois défauts corrigés) ; lib 1413+, les quatre jeux de features |
| 3 | `send` et l'annuaire | **sur master** (`8918bd072`), relu par la recherche (trois points corrigés) |
| 4 | `AgentNode`, le parent, un agent appelé en attendant | — |
| 5 | `spawn`, rester à l'écoute, les bornes | — |
| 6 | la boîte de session dans l'atelier ; l'interface proposée au chat et au MCP | — |
| 7 | l'expérience à vrai modèle | — |
| W | surveiller les fichiers (demande de Lucie, entre les lots 3 et 4) | page `04-surveiller-les-fichiers.md` (`00618a043`) ; **W1 et W2 sur master** (`a73a7941d`, correction du natif seul `dd45c9d50`) ; **W3 sur master** (`c1b6ceca1`) ; W4 (crochet Claude Code, recette) à faire |

## Lot 1 — les ports d'agent

`src/agent_ports.rs` (nouveau) et des points d'accroche dans `src/agent.rs`.

- **Sept ports** : `user_in`, `peer_in`, `model_out`, `tool_call_out`,
  `tool_result_in`, `answer_out`, `turn_end`. Ce qui est branché sur les ports
  (`AgentPorts`, posé par `Agent::with_ports`) rend, pour chaque élément :
  l'élément (tel quel ou réécrit), rien (il est **retenu**), ou une panne
  (dite sur le bus ; l'élément passe inchangé).
- **Retenir** veut dire, selon le port :
  - `user_in`, `model_out`, `answer_out`, `turn_end` : le run s'arrête, sur
    `StopReason::Held { port }` ;
  - `peer_in` : le message est écarté ;
  - `tool_call_out` : l'appel reste annoncé, il ne s'exécute pas, et son
    résultat dit « retenu » ;
  - `tool_result_in` : le contenu dit « retenu ».
- **Les résultats d'outils en fond** arrivent par la boîte, mais passent par
  `tool_result_in`, pas par `peer_in` : ils sont reconnus à leur émetteur
  (`outil #<outil>-<n>`).
- **Ce que la trace dit** : `model_out` passe après le compteur, donc la
  trace dit ce qui a été payé. Un appel réécrit par `tool_call_out` garde son
  identifiant, et un avertissement dit ce qui s'est réellement exécuté.
  Quand `model_out` retire ou ajoute des appels, la raison de fin suit ; un
  identifiant vide ou en double est refusé en le disant.
- **`GraphPorts`** tient un graphe-outil par port (`%% param: element json!`).
  Un graphe qui ne pose pas son résultat, ou dont la branche n'est pas prise
  (`NoResult`, `NotTaken`), retient l'élément.
- **La parité** : sans port attaché, l'élément n'est pas même construit. Le
  témoin rejoue le même scénario sans ports et avec des ports transparents
  (un message de pair, un tour d'outil, une réponse) : même historique,
  mêmes jetons, même run.
- **Pas encore fait** : le premier flux (`model_out` en jetons) attend
  l'exécuteur concurrent du lot 0 ; aujourd'hui, `model_out` reçoit la sortie
  entière, un flux de un.

Témoins (`src/agent_ports/tests.rs`, 16) : la parité ; une panne qui laisse
passer ; retenir et réécrire à chaque port ; un appel réécrit dit sur le bus ;
des identifiants en double refusés ; un résultat en fond par `tool_result_in` ;
un graphe sur `peer_in` qui retient et réécrit ; une branche non prise qui
retient.

## Lot 2 — `Decide` et `Route`

`src/dataflow/decide_nodes.rs` (nouveau), le refus `Complete` ← `Open` dans
`graph.rs`, `NodeFactory::schema_for`.

- **`Decide`** pose sur un élément une étiquette choisie parmi N : par le
  `Decider` s'il est servi (l'étiquette la plus probable, et sa probabilité
  sous `<as>_probability`), sinon par le modèle de génération sous une sortie
  structurée (schéma à `enum`). Une étiquette nue est lue ; une étiquette hors
  liste, ou un decider qui ne rend pas une probabilité par étiquette, fait une
  erreur qui le dit.
- **`Route`** transmet chaque élément sur le port de sa clé (`key`, `label`
  par défaut) et ne pose pas les autres : leur aval est sauté par le moteur.
  Une clé inconnue part sur `other`. Les routes s'écrivent en tableau ou en
  `a|b` (le mermaid n'a pas de tableaux), comme des noms de port.
- **Les sorties d'un `Route` sont ses routes** : `NodeFactory::schema_for(config)`
  (défaut : le schéma fixe), lu par la sonde des sous-graphes, la vérification
  des sorties de route du backend et le test de dérive schéma/nœud.
- **`Complete` ← `Open` est refusé au montage** (`DataflowGraph::connect`, par
  où passent toutes les arêtes, définitions comprises).
- **Le témoin de la page** : sur `peer_in`, un graphe `RhaiNode → Decide →
  Route` laisse passer les questions et retient le bavardage, le modèle étant
  faux.
- **Piège du banc** : l'aval d'une branche non prise n'est sauté que si son
  entrée est **requise** ; `RhaiNode` (entrée `value` optionnelle) tourne
  sans valeur et échoue. La recherche l'écrit dans sa page 04.

## Lot 3 — `send` et l'annuaire

`src/directory.rs` (nouveau) et `send` dans `src/agent.rs`.

- **L'annuaire de la session** : nom → run, fil → participants. `send`
  adresse un nom, un run, ou tout un fil (sans `to`, chacun sauf
  l'expéditeur) ; l'expéditeur et le destinataire nommé entrent au fil.
  `enroll` (l'hôte) et `claim_and_open` (l'agent) **ouvrent la boîte** de
  l'inscrit : un message écrit avant son run est lu à son run.
- **Un nom est tenu par son agent** (`claim`, un identifiant par agent posé
  par `with_messaging`) : deux agents du même nom — le nom par défaut — ne se
  volent pas leur boîte ; le second ne peut pas écrire sous ce nom, et le
  refus est dit sur le bus.
- **`send`** est annoncé par `with_messaging` (qui refuse un outillage ayant
  déjà un `send`) et traité dans la chaîne ordinaire du résultat : trace,
  compte des erreurs, ports. Les refus disent quoi faire (un nom inconnu liste
  les inscrits ; des arguments illisibles le disent).
- **Un tour reçu dit son fil** quand il est nommé : « [message de X, fil F] ».
- **Le témoin de la page** : alice écrit à bob par son nom ; c'est le modèle de
  bob qui choisit de répondre par `send` ; alice lit la réponse à son tour
  suivant. Le test lance les runs, il ne relaie rien.
- Ticket ouvert : `2026-10-11-la-pause-n-adresse-pas-par-l-annuaire` —
  `pause_dialogue` prend encore son `avec` pour un run.

## `Decide` : un gabarit (pour la mémoire)

`prompt`, un gabarit à trous `{instruction}`, `{labels}`, `{text}`, rempli en une
passe : tout le texte envoyé au decider (le banc de la mémoire y met le ChatML
de Qwen3), ou le tour `user` du llm. Vide : le texte composé d'avant. Refusé
sans `{text}` ou avec un trou inconnu. Sur master `a6bfb9b9a`.

## W1-W2 — l'index suit le disque

- **La montre** (`src/watch.rs`) : `watches` au manifeste ; balaie la source de
  l'index (`FileSource::list`), compare date et taille, attend `settle_ms` de
  stabilité, hache le contenu (blake3 : un `touch` ne dit rien), publie un
  `FileChanged` par fichier sur le sujet `files`. Un fil par montre, réveillé à
  l'arrêt ; montée au début de `mount_reactions` et arrêtée au début de
  `stop_reactions` — les mêmes chemins (ouverture, rechargement, réouverture
  par l'hôte, arrêt). Une source `snapshot` est refusée.
- **La réaction** `reindex_on_change` du gabarit code (`%% on: files`, `batch
  300`) et `ReindexFileNode`, capacité réactive nommée (la liste blanche
  réactive passe à base + 3, garde-fou mis à jour avec sa raison) : **le disque
  décide**, l'événement ne désigne que le chemin ; reprise pendant une
  indexation entière.
- **Témoin de bout en bout**, vraie base : une édition à la main puis `usages`
  voit le nouvel appelant sans `index` ; un `removed` forgé pour un fichier
  présent ne le retire pas ; un fichier supprimé perd ses usages. Vus rouges
  (montre débranchée ; ancien nœud).
- **Conséquence** : tout backend ouvert du gabarit code — le MCP du code compris
  — surveille son workspace, période 1 s.
- **Mesure du balayage** (Python, cache chaud) : 2,6 ms pour les 1 653 fichiers
  de `src/` ; 13 ms pour 6 605 fichiers.

## W3 — le retard se dit

- `src/catalog/retard.rs` (sans feature, son propre verrou) : par cellule,
  le premier et le dernier changement de chaque chemin. La montre note ; ce
  qui **relit vraiment** le fichier efface — la réaction (réindexé, retiré),
  la synchronisation (après l'écriture réussie de chaque paquet, et à la
  reprise) — et seulement si rien n'a changé après l'instant de la lecture.
  Une mise en reprise n'efface rien.
- `index_states` porte `lag` ; `usages` et le voisinage (par
  `read_catalog_in`, la cellule de l'appel) disent « N fichier(s) changé(s)
  depuis N s, pas encore réindexé(s) : ce résultat peut dater ».
- Relu par l'arbre principal (catalog.rs, code_sync : deux courses fermées) et
  par la recherche (dataflow : trois points).
- **Un rouge de master à ma charge**, corrigé (`dd45c9d50`) : W1 laissait des
  imports et un `mut` qui ne servent qu'avec la feature `code`, refusés sous
  `rag3db-native` seul avec `-D warnings`. Ce jeu est désormais dans mes
  contrôles avant tout hash (et devient le cinquième de `check_features.sh`).

## Relectures faites pour la recherche

- La page de l'exécuteur de flux (`../11-octobre-2026-recherche/04-l-executeur-de-flux.md`).
- Le pas 2 du lot 0 : d'accord, avec trois remarques, toutes reprises (le saut
  en une seule méthode, le prix du saut écrit dans la page, le témoin
  `NotTaken` contre `NoResult`).
- La page de l'exécuteur concurrent (05) : cinq points, tous repris (un même
  code de nœud pour une valeur et un flux, `PortKind::Follows` ; l'arrêt de
  l'extérieur, `cancel_handle` et un jeton qui réveille une source ;
  `token_sink` qui ne coupe que sur annulation ; le détecteur et les attentes
  hors registre ; la longueur d'un `Open` repris).
- Les déclarations de flux (`flux-declarations`) : d'accord ; la recherche
  corrige ensuite un oubli trouvé par ma relecture du lot 2 (un sous-graphe
  republiait ses ports sans leur genre ni leur posture).

## Contrôles de la soirée

- Toute passe avant un hash : `complet.sh` sur luciepc — le sous-module
  `codeparsers` au pointeur de l'arbre, un fichier modifié touché et
  « Compiling rag3weaver » exigé, toute la lib, puis
  `scripts/check_features.sh` (quatre jeux de features).
- Un rouge de la soirée n'était pas du code : après un `reset` du worktree,
  `codeparsers` était resté sur l'ancien pointeur (`reexport` inconnu).

## Rouges vus en passant

- `daemon::embeddings::tests::un_gros_envoi_est_decoupe_en_lots` : « le démon
  n'a pas répondu » une fois, pendant la passe complète, sous charge. Le test
  attend 2 s qu'un démon lancé dans un fil réponde sur un `port_libre()`
  (port pris puis lâché). C'est la même famille que la course de `daemon::db`
  vue le matin. Ce code n'est pas le mien. Le test est passé aux deux passes
  complètes suivantes : il est intermittent, pas lié au lot 1. Le ticket
  `2026-10-10-le-demon-de-test-n-a-pas-repondu-sous-charge` a été fermé le
  même soir par l'arbre principal (`2d3009efc`, le démon lié au port 0).

Le chantier précédent (le proto « tout déclaratif », six lots) est rapporté
dans `../10-octobre-2026-18h20/everything-declarative/`.
