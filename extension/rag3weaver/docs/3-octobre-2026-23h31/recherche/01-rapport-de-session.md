# Session recherche — rapport

**Mis à jour : 4 octobre 2026, vers 12 h 00.** Ce fichier se met à jour sur
place après chaque lot fusionné.

## Fait aujourd'hui (3 octobre, soirée), tout sur master

Dans l'ordre, chaque lot avec batterie verte (lib + tuyauterie
`scripts/test_backend_code.py`) avant sa fusion :

1. **Défaut embeddings** (72fc2037b) : un backend en mots seuls démarre sans
   service d'embarquement ; `PreparedBackend::needs_embeddings()` décide,
   l'absence quand il en faut un refuse en nommant `embeddings.address` et
   `RAG3WEAVER_EMBED_SERVICE`.
2. **Faille de traversée `WaitOutputNode`** (bfbed7097) : `journal_borne()` —
   refus de `..` puis canonisation du maillon existant le plus profond.
   Tests rouges d'abord (traversée, lien symbolique sortant).
3. **Garde des liens symboliques dans `WorkingTree`** (56fb79610, relue par
   la session de l'arbre principal qui a attrapé le trou du lien MORT —
   fermé en strict) : un lien dont la cible canonisée sort de la racine
   n'est ni listé, ni lu, ni écrit à travers ; écarté avec sa raison.
4. **`index` + `wait` en base déclarative**, attachés aux deux manifestes
   (35ec4c43f, 706ca2c47) : le cloud suit l'indexation sans `run_commands`.
5. **Recherche auto-adaptative** (06a4f666e + adb49261d) : `SearchMode` auto
   par défaut, décision UNE fois dans `SearchSourceNode` (try_lock, occupé =
   balayage, jamais d'attente), `ScanFilesNode` (classé mots-puis-proximité,
   borné, secrets écartés, muet hors mode), graphe générique
   `search_workspace.mmd`, ligne d'état par la méta (balayage / partiel avec
   temps restant ; prêt = silence).
6. **Poids de la marque de test** : 0,5 uniforme (case/suite/support,
   `default` 1,0) **plus** `test_role` dans `return_fields` — sans
   l'enrichissement le poids est un vœu. Calibré à e2e_code (plafond 0,597),
   plateau au banc (0,4–0,7 identiques). Déclaré par l'arbre principal avec
   la pile codeparsers (359d61daf). Mesures : `../3-octobre-2026-22h40/01`.
   La ligne T du banc est commitée (rejouable).
7. **Passes d'agent** (docs `../3-octobre-2026-22h40/02` à `05`) : scénario
   rejouable 5 tâches (`scripts/passe_agent_code.py`, `--llm-json`,
   `--taches`), passe faible (qwen2.5-7b servi par embarquements), passe
   Gemini (provider `vertex` ajouté à `LlmProvider`, 21 appels, 81 198
   jetons), reprises ciblées. Chaque passe a trouvé un trou qu'aucun test
   scripté ne voyait — voir le knowledge dump §6.
8. **Faille `cat ../` fermée** (30ebbdb2e) : la lecture libre confinée au
   domaine (`argument_hors_domaine` : texte puis disque canonisé), AVANT les
   acquis, oui humain qui prime, refus en standard ; ligne « pas par une
   autre formulation » au 2e refus de la même intention (1db21b324). Chantier
   bac à sable (Landlock) nommé au journal §3.
9. **Outils `usages` et `impact` attachés** aux deux manifestes et au chat
   (+ `estimate` et `index` qui manquaient à `allowed_tools`).
10. **État d'index par entité, bouclé** : le journal de conversation du chat
    polluait l'état global (trouvé par la reprise T1, vérifié par repro
    isolée) ; `index_state_for` livré par embarquements (92ecaa656), ma
    décision auto basculée, `read_catalog` basculé par codeparsers
    (42df4136f), test chat-réel dans la tuyauterie (ca1d64f26 : le journal
    s'écrit D'ABORD, les trois outils disent encore la vérité).
11. **Crochet après outil, lot 1** (760c15f22) : `after { graph, title,
    max_lines, threshold, policy }`, section au rendu, silence par défaut,
    compteurs stderr (déclenché / tu / en erreur), `hook_nodes` = base moins
    écrire/bloquer/lancer, refus actionnables au chargement. Extension
    `result_uuids` + `after.results_port` (854af5a60) pour la section
    « Liens » de codeparsers : le crochet reçoit les identités que l'outil a
    résolues, jamais de re-calcul.

## En cours (4 octobre 12h45, les deux expériences de l'optimiseur)

- **La réponse préalable sur lucivy est rendue à 9f, lue dans les
  sources (4.3.0, filtre identique à 4.0.1)** : tout champ texte passe
  par RAW_TOKENIZER câblé en dur (`lucivy-core/src/handle.rs`) —
  SimpleTokenizer (coupe sur non-alphanumérique, donc `_`) +
  CamelCaseSplitFilter + LowerCaser. Le filtre coupe camelCase et
  lettre↔chiffre puis FUSIONNE les pièces < 4 caractères (max 2 chunks) :
  `getElementById` → `getelement`, `byid`. **L'identifiant entier n'est
  jamais émis** (les pièces remplacent le jeton) et rien n'est
  déclarable par entité : la variante demande du code dans ld-lucivy.
- **Le patch local d'essai est écrit et prouvé** (9f d'accord, trois
  variantes à sa demande) : copie de ld-lucivy 4.3.0 sous
  `~/.cache/rag3weaver-build/lucivy-variantes/ld-lucivy`, pilotée par
  `LUCIVY_SPLIT_VARIANTE` — `entier` (l'identifiant d'origine émis en
  plus, même position, `_` gardé par le tokenizer pour que le snake
  entier survive), `fin` (sans la fusion-à-4 : get/element/by/id),
  `entier-fin`. Sans variable : comportement publié à l'identique (19
  tests du crate verts) ; chaque variante vérifiée par
  `test_variante_selon_env`. Branché par `[patch.crates-io]` dans le
  Cargo.toml du worktree, fusionné dans la section existante des forks
  burn (deux sections = « duplicate key ») — NON COMMITÉ, à retirer
  après l'expérience.
- **Section A au banc étagé** : les 7 combinaisons à poids égaux
  (chaque voie seule, chaque paire, le trio), phrases et identifiants
  séparés, sur la base granite+creux bge du bloc avec_creux ; jouable
  aussi sans creux (les passes « b » sautent les combinaisons SPARSE).
- **Première passe (a) tombée à 268 s — et la cause était chez moi** :
  « aucun service ne sert bge-m3, 7980 ne répond pas » parce que ma
  variable était la forme ABRÉGÉE `127.0.0.1:7979,7980,7981`, reprise du
  résumé de compaction alors que ma propre note mémoire disait déjà
  « adresses complètes, jamais 7979,7980,7981 ». Diagnostic rendu par eb
  (rien n'avait redémarré, 7980 sert bge dense+creux). Relance au
  « fini » de son créneau (cinq passes du dépôt, mesure sensible :
  aucune compile de ma part entre-temps, consigne 9f) avec
  `RAG3WEAVER_EMBED_SERVICE=127.0.0.1:7979,127.0.0.1:7980,127.0.0.1:7981`.
- **c0 a fusionné impact_fichier (9425feb05)** : branchement du crochet
  « Avant d'éditer » APRÈS les passes du banc (le rebase invaliderait le
  binaire précompilé).

## Ancien (12 h, après la coupure de quota de la nuit)

- **Le creux est une option, pas le défaut** (10bdda04d) : position de
  Lucie (gadget bien fait, parité Qdrant, expériences avant de
  préconiser) — le manifeste d'exemple ne l'active plus, README porte
  les clés et la mesure, default_weights inchangé. J'attends la liste
  d'expériences de l'optimiseur pour le banc.
- **Correctifs de reprise** (8a022da9e) : e2e_mesure_sync_source lit
  RAG3DB_ROOT (seule fautive des sept migrées, audit fait, prouvée
  depuis le worktree) ; les « reopen DB » s'expliquent ; path_in_source
  fourni aux crochets (le chemin tel que la base l'indexe — file_path
  est ABSOLU en base, mesuré).
- **« Avant d'éditer » cadrée de bout en bout** : c0 a son gabarit
  impact_fichier (égalité sur file_path via path_in_source, 60-121 ms
  sur 6 000 scopes, summary_group=case) — j'attache à sa fusion.

## Ancien (07 h)

**T5 auto + bac à sable, l'épreuve finale du lot 3 : 7 itérations,
19 448 jetons, UN refus lu, zéro fuite** — contre 15 / ~80 000 sous la
garde seule. L'agent conclut juste et le dit proprement. Le mode sans
humain est tenable.

**Pièce 2 livrée (9ebfeb8bf)** : la ligne de statut de l'index par
l'application — op index_state du backend (try_lock, busy plutôt
qu'attente), événement index_status du chat après chaque tour,
affichage « [index] mots · vecteurs N % · creux N % » hors du texte de
l'agent.

**Pièce 3 en attente de codeparsers** : la section « avant d'éditer »
(crochet after de read_file, tirée d'impact). L'obstacle nommé : impact
est par NOM, read_file par CHEMIN — demandé un mode par-fichier de
NeighborhoodNode (l'impact agrégé du fichier) plutôt que dupliquer sa
logique de traversée en gabarit. Mon côté prêt : attachement, budget,
silence par la porte read_catalog (text+relations), latence à mesurer.

## Ancien (06 h)

**Les trois lots de l'orchestration sont FERMÉS.** Lot 3 livré
(0d4c6b5e6 + 4a313daae) : le bac à sable Landlock — BacASable dans
commande.rs (ruleset construit avant le fork, le fils n'exécute que la
restriction ; une erreur refuse la commande, jamais de repli
silencieux), la clé workspace.sandbox (mode/network/extra_read/
extra_write, ~ étendu), le service posé sur chaque run, commands "auto"
qui REFUSE de s'armer sans bac, le noyau prouvé au chargement ET par le
test (dedans tout marche ; dehors lecture et écriture échouent par
EACCES ; le HOME invisible). bubblewrap : dit « pas encore branché »,
pas promis. Et la ligne d'état porte le niveau creux d'embarquements
(« creux N % » ; prêt silencieux si les trois niveaux le sont).

Reste ouvert chez moi : la T5 du scénario d'agent en mode auto AVEC bac
(l'épreuve finale du lot 3 — demande un manifeste de passe en commands
auto) ; la ligne de statut d'application du chat (journal §3, après) ;
l'instrument des quatre issues de la session mémoire (il me le rend).

## Ancien (05 h)

**Le lot 3 — bac à sable de run_command** : la proposition est au dépôt
(03-bac-a-sable-proposition.md), le code commence (crate landlock,
variante executer_confine d'Atelier, clé sandbox au manifeste, le mode
auto qui refuse de s'armer sans).

**Fermé depuis 04 h — le lot 2 en entier** (1afd8bfbe) : la pièce de
câblage workspace.index_signals (clé explicite, générique, validée dans
les deux sens — y compris models.sparse que rien ne lit) ; le manifeste
du poste déclare le creux et la tuyauterie indexe les trois signaux en
vrai ; README : coût, dette d'une base existante, cloud sans creux tant
que son service n'existe pas. Demande à embarquements : le creux dans
index_state_for (niveau séparé recommandé).

## Ancien (04 h)

**La pièce de câblage du creux** (cadrée par l'orchestration) : une clé
EXPLICITE du manifeste monte le signal creux sur l'entité d'un index
nommé quand models.sparse est déclaré — générique (toute entité d'un
schéma nommé), validée au chargement dans les deux sens. À livrer avec :
le coût du creux à l'indexation (mesuré), la conduite d'une base déjà
indexée (la dette rattrape, pas de zéro pour un signal absent — à voir
avec embarquements et index_state_for), le creux déclaré au manifeste du
poste et la recommandation cloud. Puis le lot 3 (bac à sable — la
proposition est écrite, 03-bac-a-sable-proposition.md).

**Fermés depuis la dernière mise à jour** :
- Lot 1 rendus (0f75fa79b) : ligne d'état EN TÊTE (verdict des rejeux :
  les modèles ne la relaient pas mais agissent mieux — mesuré ; la ligne
  de statut d'application du chat est au journal pour après) ; marque de
  test lisible par value_labels déclaratif ; correspondance exacte : non.
- Lot 2 fusion (91c44a96c) : la demande du 1er octobre FERMÉE par la
  mesure — voir le journal §4.2 et le message de commit ; les chiffres
  clés : creux 0,4 porte H à 0,369/11/23 et I à 0,900 sur granite+bge ;
  default_weights='bm25:0.45,vector:0.55,sparse:0.4', étage choix→défaut.
- Le banc : RAG3WEAVER_BANC_CREUX (creux indépendant du dense, dual gardé
  au même-modèle), lignes HS/IS, branche bge-m3.

## Ancien en-cours

**L'épreuve T5 Gemini du lot 6** (en fond) : la tâche 5 du scénario par
le chemin vertex DÉCLARÉ (llm → alias → source() → connect_llm) — elle
éprouve à la fois la pose du lot 6 et, en re-passant, la garde confinée.

**Fusionnés depuis la dernière mise à jour** :
- Le seuil du motif CALIBRÉ (bfdbe1d90) : la section M du banc étagé —
  126 requêtes sur src/, granite-278m, le meilleur voisin hors fichier
  d'un scope quelconque vit à 0,87 de médiane (p90 0,81 : en code, tout
  se ressemble), les vrais clones à 0,975-1,000 (embed/name/dim des impl
  Node). Le 0,72 provisoire aurait parlé à chaque édition ; le seuil est
  à 0,97 (presque-identique), la tuyauterie prouve le silence sous le
  seuil, la preuve positive vit au banc M nommée.
- La pose du lot 6 (3645e0799) : models.llm au chat, section llm en
  alias, les chat.json existants intacts — le diff d'embarquements posé
  par moi (mon fichier), vérifié lib avec et sans openai-llm.

Ancien en-cours : **lot 6 d'embarquements (models.llm au chat)** : accepté — la session
embarquements apporte connect_llm (openai + vertex) dans model_source,
la fenêtre de contexte dans ModelSource (comme dimensions), et me passe
le diff de chat.rs (mon fichier, section llm → alias de models.llm) que
je pose moi-même ; s'il met un Auth qui rafraîchit par requête, la
limite d'une heure du jeton vertex tombe. Reste ensuite : la
**calibration du seuil du crochet au banc** (premières données réelles,
granite-278m : 0,84 pour le scope édité contre son propre ancien texte,
0,76 pour son appelant — le seuil 0,72 est en dessous des deux).

**Livré depuis le premier jet** — le client « motif ailleurs »
(e87b64ba0 + a49a48422) : FilterResultsNode (seuil + exclusion à
frontière de séparateur — file_path en base est absolu, l'argument est
relatif, l'égalité stricte laissait le fichier édité dans sa propre
section), gabarit vecteur seul (le seuil est une similarité, jamais un
rang de fusion), rendu Jinja vide-si-rien, porte
silent_unless_vectors_ready (try_lock + index_state_for), attaché à
edit_file du poste. La tuyauterie porte la PREUVE SÉMANTIQUE : après
l'index, l'édition rend la section avec le vrai voisin (l'appel dans
main.rs) et jamais le fichier édité.

Ancien en-cours, pour mémoire : **le client « motif ailleurs » (lot 2 du crochet)** : après `edit_file`,
chercher par similarité le code qui ressemble à l'ANCIEN texte, hors du
fichier édité, au-dessus d'un seuil. État : `FilterResultsNode` écrit
(générique : seuil + exclusion par champ, `src/dataflow/filter_results_node.rs`,
enregistré, compte 43+13), gabarit de rendu
`templates/render/motif-ailleurs.md.jinja` écrit (vide si zéro résultat —
c'est lui qui porte le droit de se taire). Reste : le champ
`silent_unless_vectors_ready` du crochet (try_lock + `index_state_for`,
vecteurs pas prêts = silence compté), le gabarit `motif-ailleurs.mmd`,
l'attachement au manifeste du poste, la tuyauterie, et la **calibration du
seuil au banc** (le défaut sera provisoire et dit tel).

## Ce qui attend quelqu'un

- **Lucie** : la ligne d'état en tête de fiche (proposition chez
  l'orchestration — changement de rendu) ; le couple de fusion 0,45/0,55
  (recommandation du 3 octobre, références refaites avec la pile) ; la règle
  « correspondance exacte en tête » (non implémentée, posée par
  l'orchestration) ; le rendu `· test_role=case` (signalé par l'arbre
  principal).
- **Codeparsers** : sa section « Liens » (LinksNode + links.mmd) sur mon
  extension `result_uuids` ; il demandera le feu vert pour un boost de
  cohésion C1 au banc de l'étage (accordé sur le principe, régime doux,
  annoncé).
- **Embarquements** : rien — le petit modèle se relance à la demande pour
  une passe (3 secondes).

## Comment reprendre

- **Worktree** : `/home/lucied/git_workspaces/rag3db-pas-c`, branche
  `surface-agent-code`, rebasée puis poussée sur master à chaque lot
  (`git push origin surface-agent-code:master`). Submodule codeparsers : le
  remettre au pointeur après chaque rebase (`git submodule update
  extension/rag3weaver/codeparsers`).
- **Environnement des passes** : `CARGO_TARGET_DIR=<worktree>/target`,
  `-j6`, `RAG3DB_ROOT=/home/lucied/git_workspaces/rag3db`,
  `LUCIVY_SCHEDULER_THREADS=8`, `RAG3WEAVER_EMBED_CHAR_BUDGET=4096`,
  `RAG3WEAVER_GPU_DUTY=70`,
  `RAG3WEAVER_EMBED_SERVICE=127.0.0.1:7979,7980,7981`.
- **Batterie d'un lot** : lib (`cargo test --lib --features
  rag3db-native,burn-embedder,burn-ocr,code,daemon`), build backend
  (`--features daemon,rag3db-native,code`) et chat (`--features
  openai-llm`), puis `scripts/test_backend_code.py` (venv MTG :
  `/home/lucied/git_workspaces/rag3db/experiments/mtga/.venv/bin/python`) —
  le script REFUSE un binaire plus vieux que les sources.
- **Passe d'agent** : `scripts/passe_agent_code.py <out> --llm-json '…'`
  (`--taches 1,5` pour une reprise) ; faible =
  `{"base_url": "http://127.0.0.1:7983/v1", "model": "qwen2.5-7b-instruct",
  "context_tokens": 32768}` (demander le serveur à embarquements) ; Gemini =
  `{"provider": "vertex", "model": "google/gemini-3.5-flash",
  "context_tokens": 1000000}` avec `GOOGLE_APPLICATION_CREDENTIALS=
  …/.vault/vertex-sa.json` et `GOOGLE_CLOUD_PROJECT=lr-hub-472010`.
  Artefacts sous `~/.cache/rag3weaver-passes/`.
- **Pièges** : lib à rejouer après un rebase qui amène du code dans ce que
  les tests traversent ; jamais deux e2e en parallèle (target partagé) ;
  commit par chemins explicites ; les docs par le worktree détaché
  (`rag3db-docs-banc`, checkout `--detach origin/master`, push
  `HEAD:master`) ; `/tmp` est en mémoire vive (artefacts durables sous
  `~/.cache`).
