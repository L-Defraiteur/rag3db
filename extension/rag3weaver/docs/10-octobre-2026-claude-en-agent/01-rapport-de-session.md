# Claude en agent — rapport de session

Chantier E du plan de reprise (`../8-octobre-2026-16h29/orchestration/01`).
Mis à jour le 7 novembre 2026 à 14 h 10. Ce fichier se met à jour sur place.

## 1. Le rôle

Donner au trait `Llm` une implémentation de l'API Messages d'Anthropic, en
HTTP brut, puis jouer avec Claude le même jeu de tâches qu'avec Gemini et
rendre le tableau — et l'avis du modèle sur nos outils, gardé comme artefact.

Worktree `/home/lucied/git_workspaces/rag3db-anthropic`, branche
`anthropic-llm`, target `~/.cache/rag3weaver-build/anthropic`.

## 2. Fait

1. **Le client** : `src/anthropic_llm.rs` (commit `5b65f30ca`, puis un
   second avec les corrections de la passe 2). HTTP brut sur `ureq`,
   `/v1/messages` en flux SSE ; blocs `tool_use` / `tool_result` ;
   `thinking: {type: "adaptive", display: "summarized"}` par défaut, le
   résumé de réflexion poussé dans `on_reasoning` ; les blocs `thinking`
   d'un tour à outils rejoués à l'identique par `ToolCall::provider_extra`
   du premier appel (le même rail que la `thought_signature` de Gemini) ;
   `stop_reason` lu — `end_turn`, `tool_use`, `max_tokens`, `stop_sequence`,
   `refusal` rendu en **erreur nommée** (« refus du fournisseur (… catégorie :
   X) »), `model_context_window_exceeded` en `ContextOverflow` ; la clé par
   `ANTHROPIC_API_KEY`, jamais écrite ni affichée. Feature `anthropic-llm`
   (elle tire `openai-llm` pour `Auth`, `RetryPolicy`, `Clock`).
2. **Les tests** : 26 sur transcriptions enregistrées, sans réseau — texte,
   appel d'outil avec réflexion, refus, annulation, séquences d'arrêt à
   cheval sur deux événements, `max_tokens` qui garde l'appel annoncé,
   coupure de flux, usage final, forme du corps (system hoissé, résultats
   d'outils fondus en un message, réflexion rejouée en tête, rien
   d'échantillonnage envoyé), secrets absents du `Debug`.
3. **L'exemple** : `examples/anthropic_llm_stream.rs`
   (`cargo run --features anthropic-llm --example anthropic_llm_stream`).
4. **Deux passes réelles** (clé chargée dans la seule commande) :
   - prompt par défaut : 49 jetons d'entrée, 152 de sortie dont 21 de
     réflexion, 2,7 s ;
   - une tâche de ce dépôt — relire `read_sse` et dire ce qui manque face au
     flux réel : 3 322 entrée, 2 202 sortie, 32 s. **L'avis du modèle, tel
     quel** : cinq points ; trois justes et appliqués (une fin de flux sans
     `message_delta` est une coupure, pas une fin ; `message_delta` porte
     l'usage complet ; un `text_delta` ne vaut que sur un bloc texte), un
     faux (il affirme que `stop_details` n'existe pas — le flux réel le porte,
     à `null`), un hors de notre usage (outils serveur, `pause_turn`).
5. **Le branchement du harnais** : protocole `anthropic` dans
   `model_source::connect_llm`, qui rend désormais `Box<dyn Llm>` ; deux
   signatures de `chat.rs` et une ligne du binaire du chat. Lignes annoncées
   à la session recherche avant d'être écrites (chantier C touche le
   constructeur de l'agent, pas ces lignes).

## 3. Décisions prises (réversibles, dites)

- `temperature` / `top_p` ne partent jamais : Claude Opus 5 les refuse.
- `fallbacks: "default"` existe mais reste **éteint** : un remplacement
  silencieux de modèle fausserait la comparaison (accord de l'orchestration).
- `ReasoningEffort::Minimal` devient `low` ; `JsonObject` n'envoie rien (pas
  de forme sans schéma chez Anthropic).
- Un tour `system` au milieu de la conversation (le bloc d'attente du
  harnais) devient un message `{"role": "system"}` — connu d'Opus 5 et
  Fable 5, pas de Sonnet 5.
- Les séquences d'arrêt sont détectées chez nous (`first_stop` /
  `holdback`), comme pour OpenAI, et ne sont pas envoyées.

## 4. L'atterrissage du 7 novembre 2026

La branche est restée un mois sans compiler son dernier commit (le poste mis en
pause le 10 octobre). Le 7 novembre, règle de Lucie : « tout ce qui est sans
bloquant réel doit être mergé ». Worktree neuf
`~/.cache/rag3weaver-build/wt-anthropic`, branche `anthropic-llm-2` (rebase
propre sur master `f5ed39b0b`), target `~/.cache/rag3weaver-build/anthropic`.

Ce que le rebase a demandé, et ce qui a été choisi :

- **`connect_llm` rend `Arc<dyn Llm>` pour tous les protocoles** (plus un
  `Box` ; master avait gagné trois appelants qui font `Arc::new` sur le type
  concret : `backend.rs`, le binaire du backend, `daemon_host/chat.rs` — ils
  prennent l'`Arc` tel quel). Un seul type pour le catalogue, l'hôte et le
  chat ; rien qui nomme anthropic chez les appelants.
- **Le défaut de raisonnement déclaré** (`models.llm.reasoning`, arrivé sur
  master le 25 octobre) est honoré par le client Anthropic comme par OpenAI :
  `with_reasoning_default`, l'appel l'emporte ; un test de plus (27).
- **Le test d'`openai_llm` qui inspectait le corps à travers `connect_llm`**
  ne peut plus (le type est abstrait) : il vérifie que la déclaration est lue
  et que la connexion aboutit ; l'effet est prouvé par le test du dessus.
- La feature `anthropic-llm` entre dans le jeu large de
  `scripts/check_features.sh` et de `docs/procedures/push-d-un-lot.md`.

La passe (`~/.cache/rag3weaver-build/wt-anthropic-passe.sh`, sous `poste
lourd`, lib épinglée `04063ee06`, `RAG3WEAVER_MOTEUR_ANCIEN=1`, aucun appel
réel à l'API) : `check_features` vert sur les cinq jeux ; lib entière sous le
jeu large : **1 904 tests, 0 échec** ; `e2e_lectures` (une ligne de l'hôte
touchée) : 20 tests, 0 échec ; l'étape navigateur ensuite. Un faux rouge du
script au passage — `grep -i FAILED` attrapait « 0 failed » — corrigé : un
vert est l'absence de rouge, jugée sur « N failed » non nul et `SAUTÉ`.

**Ce qui n'est pas fait** : la passe d'agent Gemini / Claude (page 03) n'a
jamais été jouée ; elle attend le mot de Lucie pour la clé, et le jeu de
tâches est inchangé. Le chantier suivant de cette session est le §1 du plan du
week-end (les hooks de Claude Code,
`../07-novembre-2026-hooks-claude-code/01`).

## 5. Ce qui attend Lucie

Rien pour l'instant. À sa décision plus tard : allumer le repli serveur en
produit (hors comparaison), et le modèle de la seconde passe
(`claude-fable-5-1` : il refuse `tool_choice` `any`/`tool` et vérifie
l'historique des blocs de réflexion — `with_thinking_binding_drop`).

## 6. Comment reprendre

1. `cd ~/git_workspaces/rag3db-anthropic/extension/rag3weaver ;
   export CARGO_TARGET_DIR=~/.cache/rag3weaver-build/anthropic` ;
   `poste lourd cargo test --lib --features anthropic-llm anthropic_llm model_source`.
2. Le backend et le chat pour la passe : features
   `daemon,rag3db-native,code,anthropic-llm`, lib partagée de l'arbre
   principal (`RAG3DB_SHARED=1`, `RAG3DB_LIBRARY_DIR` et
   `RAG3DB_INCLUDE_DIR` = `~/git_workspaces/rag3db/build/lecteurs-csv/src`,
   `RAG3DB_ROOT=~/git_workspaces/rag3db`) ; liens `extension/vector/build`
   et `extension/rag3weaver/target` posés dans le worktree (non commités).
3. La passe : `passe.sh claude-1` (bloc llm `{"provider":"anthropic",
   "model":"claude-opus-5","context_tokens":1000000}`, `--commands auto`,
   régime doux, service d'embarquement distant). Référence Gemini :
   `../3-octobre-2026-22h40/04` et `~/.cache/rag3weaver-passes/gemini-1/`,
   T5 sous bac à sable : `gemini-5-auto-bac/`.
