# Session recherche — ce que je sais

**Mis à jour : 4 octobre 2026, vers 02 h 10.** Le rapport (01) dit l'état ;
ici, l'architecture et les leçons, pour qu'une reprise n'ait pas à les
redécouvrir.

## 1. Le backend déclaratif et ses clés

`rag3weaver-backend <backend.json>` (JSONL stdin : `describe`, `call`,
`journal`, `journal_read`, `shutdown` ; réponses `{"ok":…, "result":…}`).
`PreparedBackend::load` charge et VALIDE tout (refus actionnables — la
leçon constante : nommer le paramètre à changer, pas la règle) ; puis
`open(conn, Option<Box<dyn Embedder>>)` → `Backend { prepared, catalog,
file_source, garde }`. Clés du manifeste qui sont à moi :

- `workspace { source: snapshot|working_tree, root, read_only, commands:
  off|standard|approval|auto, index: Option<String> }` — `"code"` seul
  connu, le schéma se NOMME (jamais copié : un schéma de payload ne sait
  pas dire `Text`).
- `ToolAttachment` : `description` (surcharge le `%% description:` du
  gabarit), `policy { read_files, write_files, run_commands }` (tout faux
  par défaut), `after` (le crochet, §5).
- `BackendEntity.description` : reprise par `describe()` partout où
  l'entité apparaît.
- `models.embed` remplace `embeddings` (l'un ou l'autre) ; nos deux
  manifestes restent sur `embeddings`.

`src/backend_code.rs` : `BASE_NODES` (la base déclarative — dont
`IndexNode` et `WaitOutputNode` depuis qu'ils sont sûrs), `READ_NODES`
(read/grep/list/**Scan**), `WRITE_NODES` (edit), `RUN_NODES` (run seul),
`allowed_nodes(policy)`, `hook_nodes(policy)` (§5),
`validate_tool_policy`, `build_source`, `build_garde`.

Manifestes : `templates/backends/code/backend.json` (poste : working_tree,
commands approval) et `snapshot.json` (cloud : snapshot, PAS de
run_command — test `la_surface_cloud_ne_lance_aucun_processus`). Outils
des deux : search_code, read_file, grep_files, list_files, schema,
edit_file, estimate, index, wait_output (cloud compris), usages, impact ;
le poste ajoute run_command. Chat : `templates/apps/code/chat.json`.

## 2. Les gardes (trois, indépendantes, et elles se recouvrent)

1. **Chemins des outils de fichiers** : `check_relative` (vide, absolu,
   `..`) puis `full_path_within` (le maillon existant le plus profond,
   canonisé, sous la racine canonisée ; lien MORT refusé en strict — une
   écriture le suivrait en créant la cible dehors). `code_tools.rs`.
2. **La porte des commandes** (`commande.rs`, code de Lucie) :
   `decomposer` (codeparsers) refuse ce qu'il ne réduit pas ; familles
   lecture seule libres SEULEMENT si chaque argument se prouve dans le
   domaine (`argument_hors_domaine` : `$`, `~`, `../`, `=/`, absolu ou
   lien canonisé hors domaine → « demande ») ; le confinement passe AVANT
   les acquis (un oui sur `cat x.rs` retient la famille — il ne doit pas
   couvrir `cat ../secret`) ; seul `accorde_par_l_utilisateur` prime ; au
   2e refus de la même intention, le motif ferme la porte aux variantes.
   **Limite écrite** : analyser du shell n'est pas une frontière de
   sécurité — bac à sable (Landlock) au journal des chantiers §3.
3. **`WaitOutputNode`** : `journal_borne()` — mêmes deux temps.

## 3. La recherche adaptative (mode auto)

La décision se prend UNE fois, dans `SearchSourceNode`, et descend par la
requête (`QueryPayload.scan`) — deux lecteurs d'état divergeraient pendant
une course. `try_lock` sur le catalogue : occupé = balayage, jamais
d'attente ni d'erreur. L'état est celui de la CIBLE
(`index_state_for(target_name)`, voir §6-c). Décision pure et testée :
`adaptive_search_decision`. En scan : `options.signals = NONE` (les nœuds
de signal passent leur tour par `SignalInput::Scan`), cible résolue si le
verrou est libre, sinon `None` (resolve/paginate tolèrent le vide).
`ScanFilesNode` : liste de la source (exclusions) + `probable_secret`
re-appliqué, mots ≥ 2 lettres, classement (mots distincts, meilleure
ligne, occurrences), budget `options.limit`, fichiers > 512 Ko sautés et
comptés, MÊME forme de résultat (`UnifiedResult`, uuid `scan:<path>`),
méta « combien d'autres ». La ligne d'état rend sous `⚠` et ne dit que ce
qui n'est pas évident (balayage / partiel avec `vectors_seconds_left` ;
prêt = silence). Graphe : `templates/tools/search_workspace.mmd`
(générique — la cible est un binding du manifeste) ; `search_structured`
intact pour le notebook.

## 4. Les poids (pas C) — ce qui est mesuré

Échelle de préséance : appelant > choix du graphe > entité > défaut du
gabarit > moteur ; le PRODUIT des poids par champ s'applique
(`FieldWeightNode`). **Un champ pondéré doit être dans `return_fields`,
sinon le poids est un vœu** — le nœud l'avoue en méta (« champ absent des
données — pondération neutre ») ; c'est cet aveu qui a trahi la première
ligne T plate du banc. Déclaré dans `Scope` : `scope_type` 1,0/0,85 et
`test_role` 0,5 (case/suite/support, défaut 1,0). Références du banc
étagé (granite-278m, service distant, pile codeparsers) : sans poids
0,361/11/24 à 7 372 scopes ; avec la déclaration 0,405/13/26 (avant-pile :
0,406-0,407/13/27). La ligne T est un plateau 0,4–0,7 : la valeur se
décide à e2e_code (plafond 0,597 — scores 0,0159 contre 0,0095), 0,5 pour
la marge. En attente de Lucie : couple de fusion 0,45/0,55.

## 5. Le crochet après outil

`after { graph, title ("À voir aussi"), max_lines (12), threshold,
policy, results_port }` sur un outil du manifeste. Après l'exécution :
le graphe du crochet tourne avec les SEULS arguments que son gabarit
déclare (`instantiate` refuse l'inconnu, à raison) + `threshold` +
`result_uuids` (les uuids des résultats de l'outil, capturés par un port
de métadonnées — `results_port` — jamais re-calculés ; les `scan:`
écartés ; le paramètre se déclare REQUIS, `json!`). La section s'ajoute à
`response["result"]` (string) ou en `response["after"]` (structuré),
tronquée ET avouée. Silence par défaut ; erreur journalisée jamais
bloquante ; compteurs stderr `[crochet <outil>] déclenché|tu|en erreur` —
sans eux le défaut sûr serait invisible (session mémoire). `hook_nodes` :
jamais écrire/bloquer/lancer (Run et Edit interdits même déclarés).
Refus au chargement : nœud interdit, `result_uuids` sans `results_port`
et l'inverse, `max_lines` 0. Contrat avec la session mémoire : la portée
`person` ne s'affiche JAMAIS tant que le protocole n'a pas d'identité
d'appelant (dette nommée chez elle). Client en cours : « motif
ailleurs » (rapport §En cours) ; client prévu : mémoires de `read_file` ;
client de codeparsers : section « Liens » (LinksNode).

## 6. Les passes d'agent — ce qu'elles ont appris

Scénario : docs `../3-octobre-2026-22h40/02` ; grilles : 03 (faible),
04 (Gemini), 05 (reprises). Chaque passe a trouvé un trou qu'aucun test
scripté ne voyait :

- (a) **Un outil qui rend un vide sans dire d'où il vient** fait conclure
  faux (« la fonction n'existe pas ») — corrigé : description + porte
  d'état dans `usages`/`impact` (codeparsers, `catalog_read.rs`).
- (b) **Un modèle fort cherche un autre chemin après un refus** :
  `read_file(../)` refusé → `cat ../` par la liste libre (fermé) ; puis
  15 itérations de variantes — d'où la ligne « pas par une autre
  formulation ».
- (c) **L'état d'index global ment en chat réel** : le journal de
  conversation écrit dans la même base dès le premier message → plus
  jamais `Never`. Vérité : par entité (`index_state_for`). Le test
  chat-réel de la tuyauterie écrit le journal D'ABORD.
- Le harnais du chat est blanchi pour le multi-tour : `tools` renvoyé à
  chaque tour (fiches relues fraîches), historique avec `tool_call_id`
  mot pour mot. Le décrochage du 7B (appels écrits en texte dès le 2e
  tour) est la limite du modèle sur un vrai outillage ; proposition d'un
  rappel de protocole borné au doc 03, non codée.
- Vertex : `LlmProvider.provider = "vertex"` (`project`, `location`,
  jeton par `TokenSource`/`GOOGLE_APPLICATION_CREDENTIALS`, une heure —
  limite avouée). `.vault/vertex-sa.json`, projet `lr-hub-472010`.

### Le harnais, pour les autres sessions (vérifié à la source)

- Un résultat d'outil n'est une ERREUR que si son JSON a un champ
  "error" (chaîne) au premier niveau (agent.rs, error_detail) ;
  {"ok": true, …} n'est jamais compté, quel que soit le contenu. La
  forme « demande de complément » = ok:true, executed:false, charge
  needs — le précédent est la validation d'entrée du backend.
- stop_on_repeated_error : DEUX erreurs consécutives à clé identique
  (outil + texte exact), reset au premier résultat sain.
- La coupure « pas par une autre formulation » (Garde) ne voit que les
  commandes shell hors domaine, jamais les outils.
- Le chat renvoie tools à CHAQUE tour (fiches relues fraîches) et
  l'historique garde tool_calls/tool_call_id mot pour mot.
- Mesure des passes : accepted:false AVEC marche à suivre n'a jamais
  fait boucler ni renoncer un agent ; le refus sec sans chemin, si. Et
  la reprise déviée la plus grave CHANGE d'outil (T5 : read_file refusé
  → cat) — tout compteur par même-outil est un minorant.

## 7. Défauts connus, essais sans succès

- La ligne d'état sous `⚠` n'est relayée par AUCUN modèle (faible et
  Gemini) — proposition « en tête de fiche » chez Lucie ; ne pas coder
  sans son mot (changement de rendu).
- Le seuil du motif-ailleurs est CALIBRÉ : 0,97. La section M du banc
  (126 requêtes sur src/, granite-278m) : meilleur voisin hors fichier
  max 1,000, p10 0,975, p25 0,923, p50 0,868, p90 0,811 — en code tout
  se ressemble, le discriminant est le presque-identique. Le micro-corpus
  de la tuyauterie (0,76-0,84) était TROMPEUR : ses paires vivent sous le
  bruit de fond de src/. Leçon : jamais calibrer un seuil de similarité
  code sur un corpus jouet. La section M se rejoue avec le banc
  (RAG3WEAVER_BANC_MODELE=granite-278m, ./run_e2e.sh --test
  e2e_banc_etage).
- L'exclusion d'un champ chemin se compare à frontière de séparateur
  (file_path en base est ABSOLU, l'argument d'outil est relatif) —
  l'égalité stricte a laissé le fichier édité dans sa propre section au
  premier run d'attache.
- Première ligne T du banc : plate par pondération neutre (champ non
  enrichi), pas par neutralité — un « zéro effet » se vérifie en
  branchant le levier avant de conclure.
- `estimate.rs` cassait la compilation sans features (corrigé par
  embarquements) ; `e2e_generic_search` casse `code,rag3db-native` sans
  `burn-embedder` (corrigé : gardes sur les helpers).
- Les comptes de nœuds vivent dans `BUILTIN_NODE_COUNT`
  (`node_factories.rs`) — une seule constante, suivie par 5 tests.
- Le banc et e2e_code ont un corpus VIVANT (src/ et port.rs) : toute
  édition change leurs résultats ; références à refaire après chaque
  changement de codeparsers.
