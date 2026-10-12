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

## Le soir — la montre en service, et Route et Decide en flux

Sur master `94354595c` (quatre commits, relus par l'arbre principal et la
recherche ; sept tickets fermés dedans) :

- **La réconciliation au démarrage** : le disque contre l'empreinte que l'index
  garde par fichier (`File.content_hash`) ; ce qui a bougé pendant que la
  montre ne tournait pas est repris (trouvé sur la base de Lucie : `callees`
  rendait « aucune définition »).
- **Le coût au repos** : un tour ne lit plus que les dates ; la liste de la
  source n'est relue que quand un dossier bouge. Compteurs par phase dans
  `index_state` (`watches`). Le « 0,7 cœur » mesuré sur l'hôte était
  probablement 7 % (des tops au centième de seconde) ; remesure demandée.
- **Un fichier créé est indexé**, de bout en bout.
- **Trois défauts vus sur luciepc** : la racine disparue met la montre en pause
  (rien n'est retiré) ; une lecture attend un catalogue tenu brièvement au lieu
  de le dire occupé ; l'arrêt coupe une réaction longue entre deux fichiers.
- **Un quatrième, trouvé par les témoins** : le registre « indexation en
  cours » de `code_sync` était clé par `source_id` (« local » pour toutes les
  copies de travail) ; il l'est par la racine.
- **Route et Decide en flux** : l'entrée `element` se lit élément par élément,
  les sorties la suivent (`follows`) ; un lecteur qui lâche ne fait pas
  échouer Route ; l'appel au modèle de Decide s'arrête au jeton suivant si le
  run est annulé. Le témoin réel passe par l'exécuteur concurrent de la
  recherche. **La correction NoReader est partie avant la relecture** de la
  recherche (la file de la montre pressait) ; relue après coup, d'accord.
- **Une page de plus** : `04-surveiller-les-fichiers.md` §5-6 (ce que la mise en
  service a appris ; la base copiée d'un autre arbre et sa condition, B4).
- **Un ticket ouvert** : un index entier qui se fige pendant un checkout
  (recette de diagnostic ; suspect principal, la lib partagée réécrite en
  place sous l'hôte).

**La suite, dans l'ordre fixé par l'orchestration** : le lot suivant (la base
copiée en relatif, les remarques de relecture, `max_tokens` de Decide, la
reprise dite) ; la mesure phase par phase des 6 s de réindexation d'un fichier
et son correctif ; la file à deux priorités côté embarquement et le délai
borné ; le retrait par ensemble ; puis le lot 4 des agents (`AgentNode`), que
le pont vivant de la recherche rend possible.

## La nuit du 12 octobre — les « 6 s », et le lot de la montre sur master

Luciepc pris jusqu'au matin : l'orchestration a fait bâtir ici, sous `poste lourd`, et pousser
sous le verrou de push (la fenêtre de push se prend par le verrou, le rejeu se fait dessous).

- **`30f1f7949`, la suite des relectures de la montre.**
  - Une lecture passe pendant une réaction qui enchaîne les fichiers : la réaction cède aux
    lecteurs, et les lectures attendent un verrou court (`lock_for_read`).
  - Le reçu « occupé » d'`index_states` porte montres et reprise. Le correctif des montres est
    celui de l'arbre principal ; mon témoin déterministe a été rouge avant le rebase, vert après.
    Ticket fermé.
  - La ligne `[montre]` par fichier dans le journal de l'hôte, demandée par codeparsers.
  - L'instrumentation de la mesure par phase ; `max_tokens` de Decide (2 048).
- **`df1184d32`, la cause des « 6 s ».** La mesure par phase (page 04, §7) sur un index de
  `src/` (15 631 morceaux) l'a montré : la suppression des morceaux du scope changé coûtait
  2,1 à 2,9 s **par ligne qui porte un vecteur sous HNSW**, contre 2 à 8 ms pour les lire.
  - Le contournement : les vecteurs sont remis à NULL juste avant le `DETACH DELETE`, dans le
    dialecte rag3db seulement.
  - Réindexer un fichier : de 2,5–3,9 s à 0,56–0,68 s. Retirer un fichier de 32 scopes : de
    96–114 s à 0,40 s.
  - Ticket moteur ouvert pour hotfixs. Le banc mesure 20 à 50 ms par ligne indexée (0,7 ms par
    NULL d'abord), et prépare une variante fidèle à ma forme.
- **`5fc38460d`, la relecture de la recherche.**
  - « Embarquer » sort dans la ligne `[montre]` sans la variable du profil (témoin rouge, puis
    vert).
  - Le vidage vide aussi le marqueur dense : une suppression qui échoue laisse une dette
    honnête.
  - Le vidage sauté se dit ; `record()` est sûr face à une panique.
- **Le P1 (morceaux sans texte) n'était pas chez moi** : c'était le balayage filtré des chaînes
  du moteur, corrigé par le cœur (`f6179ade0`). Mes trois suspects sont tombés, et le témoin de
  réingestion a été retiré du lot.
- **Contrôles** :
  - chaque push : la lib entière rejouée sous le verrou (1 543 à 1 545 passés, 0 échec),
    `check_features` 5/5, le natif seul sans avertissement ;
  - `e2e_invariant_des_vecteurs` 8 sur 8 sur la lib courante du magasin ;
  - trois contrôles lus à chaque hash.
- **Fait en passant** : le worktree `rag3db-agents` de luciepc est effacé (codeparsers n'y
  indexait rien), et le worktree local aussi, après chaque fusion. Le target `target-I-ici`
  (6,7 Go) est gardé.
- **Ensuite** : `LlmNode.tokens` (le premier flux de jetons, partage convenu avec la recherche :
  flux non branchés jetés chez elle, `Spot` et `TokenEvents` sur le sujet `tokens` chez moi).
  Codeparsers rejoue son renommage sur la base entière, à df1184d32.

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
