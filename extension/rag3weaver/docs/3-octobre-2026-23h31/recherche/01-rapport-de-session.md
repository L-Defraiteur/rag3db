# Session recherche — rapport

**Mis à jour : 4 octobre 2026, vers 01 h 15.** Ce fichier se met à jour sur
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

## En cours

**Lot 6 d'embarquements (models.llm au chat)** : accepté — la session
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
