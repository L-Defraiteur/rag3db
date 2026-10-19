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

## Le matin du 12 octobre — la fraîcheur sur la base entière, et les jetons

La priorité de Lucie est la fraîcheur. Codeparsers a rejoué cinq fois le même témoin sur la base
entière (102 469 morceaux) : un renommage sur deux fichiers, vu par `usages`. On est passé de
7,26 s à 5,21 s, puis à ce que dira le cinquième rejeu. Chaque lot a été poussé sous le verrou,
après la lib entière et `check_features` rejoués.

- **Le découpage** (`2a3f57f0d`, `5c97f4c5a`) : la ligne `[montre]` attribue chaque étape à sa
  phase, entre crochets, avec ce qui n'y est pas détaillé. Le tiers non nommé d'« ingérer »
  était la résolution des symboles et le drain.
- **`LlmNode.tokens`** (`7627fa92d`) : une sortie `tokens` en flux, un puits double, et
  `stop_when_unread`. La recherche l'a éprouvée sur qwen3-1.7b : premier jeton lu à 210 ms pour
  une génération de 14,1 s, 822 ns de canal par jeton.
- **La réindexation par lot** (`c391e8387`) :
  - `reingest_files`, un passage pour les fichiers vus ensemble ;
  - `ReindexFileNode` par lots, dont la taille suit un budget de tenue (400 ms) pour laisser
    passer les lecteurs ;
  - la fin d'un fichier qui va du `File` à ses scopes par l'arête `DEFINED_IN`
    (`SnapshotConfig.fineScopeVia`), au lieu de balayer `Scope`.
  Quatrième rejeu : 5,21 s, une réaction pour deux fichiers.
- **La période par défaut** (`2b14edd88`) : 250 ms de période et 150 ms de pose. Coût au repos :
  0,8 % d'un cœur sur 1 654 fichiers, 3,5 % sur 6 929 (page 04, §8).
- **Les symboles** (`8bfdd74d3`, `25bbca30b`) : la résolution d'un lot ne relit plus toutes les
  mentions des noms courants, mais seulement celles du lot et les réexportations. Sur le témoin,
  30 mentions avant, au plus 3 après, et les arêtes sont identiques (e2e des arêtes verts).
- **`TokenEvents` et `Spot`** (`3db578589`) : les jetons sur le bus (sujet `tokens`), et le
  classement au fil de l'eau avec arrêt du modèle.
- **Corrigé en chemin** : j'avais annoncé trois suites e2e vertes pour `c391e8387`, alors que
  `run_e2e.sh` n'en avait joué qu'une (il ne garde que le dernier filtre). Les deux autres ont été
  jouées depuis, vertes ; le piège est en mémoire.
- **Tickets** : la suppression d'une ligne indexée HNSW (pour hotfixs, que le banc a reproduite,
  88 % dans `keepNodeReachable`) ; le témoin du réacteur instable sous charge (pour la recherche).
- **Ménage** : le worktree et le target de luciepc sont effacés.
- **Ensuite** : la boîte par nom de l'annuaire (`Directory::deliver`), qui branche le lancement en
  fond de la recherche ; la réaction sur événement du noyau (notify, avec le balayage en filet) ;
  « planifier » avec la jointure par clé du cœur ; l'AgentNode.

## L'après-midi du 12 octobre — une porte, des boîtes, le noyau

- **Une seule porte de l'index** (`838ed1f0b`, `03c2b065e`) : `code::admission` décide pour
  l'index entier, la montre, la réconciliation et la réingestion. La montre n'intègre plus les
  722 fichiers que l'index écarte.
- **La boîte par nom** (`bbaf4481d`) : `Directory::deliver`, qui branche le lancement en fond de
  la recherche ; une boîte qui déborde le dit (`dab191e43`).
- **Le contournement de la suppression HNSW retiré** (`c0679ed33`) : 5,4 ms par morceau sur la
  lib corrigée par hotfixs.
- **Les paquets de jetons** (`c5cd38152`, `batch_ms`).
- **Le noyau réveille la montre** (`fb23b4f6d`, crate notify) : la période n'est plus qu'un filet.
  Au cinquième rejeu de codeparsers, un renommage se voit en 2,29 s (7,26 s le matin).
- **L'embarquement hors du verrou**, préparé le soir : interrompu par la pause de Lucie (12 au
  18 octobre), commité en local.

## Le 18 octobre — la reprise, puis l'IDE

- **L'embarquement hors du verrou** (`113e388fd`) :
  - la réaction réindexe en plein texte sous le verrou, puis calcule les vecteurs sans lui ;
  - le chemin dual pose enfin `_sparse_hash` (BGE-M3 recalculait les mêmes morceaux à chaque
    passe) ;
  - le rejeu a trouvé un rouge que la pause avait laissé : un morceau muet nommé sans sa base,
    corrigé.

  Au septième rejeu de codeparsers : la tenue passe de 1 636 à 1 238 ms, et 2,11 s de bout en
  bout.
- **`labels_from` dans Decide** (`4c3e8179a`) : les candidats de l'élément d'abord, les fixes
  ensuite, refus nommé d'un champ absent. La tentative 4 des fiches de la mémoire donne 36/39 et
  39 appels au lieu de 117.
- **Vertex 3.8 par défaut** (`f1d575b50`) :
  - le témoin du slug était rouge d'abord : le compteur rangeait `llm.google/…` ;
  - qualité comparable à 3.5, environ 50 % de durée en plus.
- **Le rouge sous charge départagé** (`a81e398c4`) :
  - la reprise après arrêt rougit 2/10 avec 113e388fd et 1/10 sans : c'est une course du test,
    corrigée ;
  - trois autres témoins instables sous charge ont un ticket, dont le lot vu à moitié, donné à
    l'arbre principal pour son L6 (un instantané par appel d'outil).
- **L'IDE**, la priorité de Lucie :
  - la page de conception (`f5d50fe16`, page 05) ;
  - A1, le JSON des outils de lecture (`b665d94c3`) ;
  - A2, le socle sur l'hôte : entrée par jeton en cookie, `/ide/call`, `/ide/flux` numéroté et
    rejouable (`6695017d6`) ;
  - A3, la page de lecture : arbre, fichier, symboles cliquables, recherche, une URL par vue
    (`903aa05e1`). Le rendu attend la relecture de Lucie dans un navigateur.
- **Les passes tournent sur luciepc**, détachées par `systemd-run` ; le push part de là-bas dans
  un `if VERT` et refuse tout code arrivé sur master depuis sa base. Trois fois, ce code était
  hors du crate ou une cible de test autonome : rebase sans rejeu, par un script à liste fermée.
- **Ma faute** : mes passes du matin embarquaient sur la carte de ce poste
  (`RAG3WEAVER_EMBED_SERVICE` non posé), vues par Lucie. Corrigé dans mes scripts.
- **Ensuite** : A5, le voisinage en graphe ; puis `LlmError::Unavailable`, une erreur nommée quand
  Vertex rend un 429 de quota ou un 503, que les suites cloud comptent comme un saut nommé, jamais
  comme un vert ni un rouge. A4 (`cited_by`) est chez la mémoire.

## La nuit du 18 au 19 octobre — l'IDE vivant

Onze commits sur master, tous joués sur luciepc (la lib deux fois, `check_features`, le
navigateur exigé quand le lot touche une page), poussés dans un `if VERT` après trois contrôles.

| Commit | Ce qu'il pose |
|---|---|
| `e89bde792` | la montre ne perd plus un fichier créé pendant sa liste (« racy git » : une date de dossier trop récente fait relire) ; 3 rouges sur 20 sous charge avant, 0 après |
| `79be52abe` | lot 3 des composants : le graphe des modules d'un composant, servi à une page d'aperçu par un registre CommonJS |
| `f32d97383` | lot 4 : un composant modifié se recharge dans l'aperçu **en gardant son état** (React Refresh), constaté dans Chromium avec le vrai React 19.2 |
| `d18951c61` | le témoin de `tail` en fond juge les lignes du bloc, plus des sous-chaînes (le dossier `.tmpYl4dqn` contenait `l4`) |
| `cd164cb7d` | C1 : le graphe d'un outil, d'une réaction ou d'un composant en JSON (`/ide/graphe`, outil `graph`) |
| `5f9cf0983` | un run dit ce qu'il exécute (`RunStarted.target`) ; les appels d'outil publient sur `dataflow`, les réactions sur `reactions`, qu'aucune réaction ne peut écouter |
| `178ff1cd4` | C2 : le DAG vivant d'un outil dans l'IDE, allumé nœud par nœud par ses runs |
| `f6ba5aafc` | `cancel_after` annule après le démarrage du premier nœud (six témoins d'annulation avaient la course) |
| `863ba4a83` | (a) : le graphe de clic d'un composant s'exécute enfin dans l'aperçu, dessiné à côté du rendu et allumé à chaque clic |
| `fadc999df` | le témoin de l'arrêt juge l'ordre : l'arrêt attend le lot en cours, comme conçu ; la borne de 3 s était fausse |
| `456f50ae6` | (b) : la vue des réactions et de leurs runs ; la montre visible |

**Les défauts que les témoins ont trouvés en route** (tous corrigés, chacun avec son rouge d'abord) :

- le vrai `react-dom` importe `react` en `peerDependencies` : les pairs se résolvent vers la copie
  du backend ;
- un éditeur qui écrit en place laisse voir un fichier vide : un rechargement entier attend une
  seconde ;
- aucun appel d'outil ni aucune réaction ne publiait ses runs (`build_services` ne posait pas le
  bus) : décision de c7, `dataflow` et `reactions` ;
- la trace enregistrait un run sans dire de quel outil ;
- le graphe de clic était validé mais jamais exécuté par l'aperçu ;
- React Refresh rejouait l'effet du dernier tour après un rechargement, ce qui aurait refait l'appel
  d'outil d'un `Effet` : le dernier tour traité vit dans une ref ;
- un graphe à cycle laissait des couches vides, et la hauteur du dessin valait `NaN`, dans
  l'aperçu comme dans le DAG de l'IDE.

**Ma faute** : `5f9cf0983` prenait `catalog.lock()` à chaque appel de graphe pour obtenir le bus.
Les lectures attendaient de nouveau l'écrivain (`search_code` à 2 015 ms) et `e2e_lectures` a
rougi master. La lib et `check_features` ne le voient pas. L'arbre principal l'a corrigé (le bus
vient de la vue). Désormais, un lot qui touche `Backend::run`, l'hôte ou le bus joue
`e2e_lectures` avant son push.

**Témoins départagés** : `tail` en fond (le test), l'annulation (le test), l'arrêt d'une réaction
longue (la borne ; ticket fermé, mesures dedans). Restent ouverts au ticket des témoins sous
charge : le lot vu à moitié (chez l'arbre principal, L6) et `the_watch_does_not_index…`.

**Accords avec les autres sessions** :

- avec la mémoire (96) : l'enveloppe commune des événements est celle du flux
  (`{id, sujet, backend, at_ms, evenement}`), et elle écrit ses événements du catalogue sur cette
  forme ;
- avec G (90) : son témoin navigateur porte mes vues, CodeMirror est passé par mon magasin de libs
  (semver, `module_graph_of`, `compile_module`), et son magasin réduit (`prune_store`) sert aux
  deux témoins d'aperçu.

**Comment reprendre** :

- (c) attend la pièce 1 de la mémoire : rejouer son témoin anti-boucle (une réaction qui écrit
  publie sur `catalog` et ne se relance qu'une fois) et `e2e_reaction_montee` sur la publication
  des runs ;
- la marche suivante de l'IDE (c7, page 09 de codeparsers) : les propositions — `proposer(files,
  reason)` validé sur un recouvrement en mémoire, tenu par l'hôte et publié sur le flux ; la vue
  (diff textuel, diff de graphe nœud à nœud, aperçu du module régénéré) ; `appliquer(id)` et
  `rejeter(id, pourquoi)`, en mode approbation par défaut. Dépend du lot tsx de codeparsers
  (`KINDS`, `declare_set`) ;
- les scripts de passe sont dans `~/.cache/rag3weaver-build/I-*.sh` sur luciepc (voir le relevé).

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
