# Claude en agent — ce qu'il faut savoir

Mis à jour le 10 octobre 2026 à 11 h 30.

## Le protocole, relevé sur le flux réel (10 octobre 2026)

- Un événement = `event: <type>` puis `data: {json}` dont le `type` redit
  l'événement ; le JSON est **parfois rembourré d'espaces** avant son
  accolade finale (anti-tampon) — `serde_json` s'en moque.
- `message_start` porte `usage.input_tokens`, `cache_read_input_tokens`,
  `cache_creation_input_tokens` ; **le cache est compté à part** de
  `input_tokens` (notre `Usage` le compte dedans : on additionne).
- `message_delta` porte `delta.stop_reason`, `delta.stop_sequence`,
  `delta.stop_details` (présent, `null` hors refus) et un **usage complet**
  (entrée redite, `output_tokens` cumulatif,
  `output_tokens_details.thinking_tokens`).
- Un bloc `thinking` : `content_block_start` au texte vide, des
  `thinking_delta`, puis un `signature_delta` ; un bloc `tool_use` :
  `content_block_start` avec `id`, `name`, `input: {}`, puis des
  `input_json_delta.partial_json` à concaténer.
- Un flux complet se ferme par `message_stop` ; un EOF avant
  `message_delta` est une coupure.

## Ce que Claude Opus 5 refuse ou exige

- `temperature`, `top_p`, `top_k` : 400. Le levier est
  `output_config.effort` (`low` à `max`, défaut `high`).
- `thinking` : `{type: "adaptive"}` ; `budget_tokens` est refusé ; omettre
  le réglage = adaptatif quand même sur Opus 5 (pas sur Opus 4.8 / Sonnet 5).
  `display: "summarized"` rend un résumé ; `"omitted"` (le défaut du
  fournisseur) des blocs vides — même facturation.
- La réflexion compte dans `max_tokens` : un plafond de 512 tronque avant
  la réponse. Le chat envoie `max_output_tokens` (4 096 par défaut) et
  **ne règle pas l'effort** — Gemini a joué dans les mêmes conditions.
- Un refus de classifieur est un **HTTP 200** avec `stop_reason: "refusal"`
  et `stop_details.category` (`cyber`, `bio`, `reasoning_extraction`…).
  Demander au modèle sa chaîne de pensée brute peut être refusé
  (`reasoning_extraction`) : lire les blocs résumés à la place.
- Les blocs `thinking` d'un tour à outils doivent revenir **inchangés** au
  tour suivant (c'est pourquoi ils voyagent dans `provider_extra`). Pour
  les tours passés sans outil, le fournisseur les ignore — sauf Claude
  Fable 5.1, qui vérifie l'historique : `with_thinking_binding_drop()`.
- Claude Fable 5.1 et Opus 5.5 refusent `tool_choice` `any` et `tool`
  (nos `Required` et `Function`) ; Opus 5 les accepte.
- Un message `{"role": "system"}` au milieu de `messages` est accepté par
  Opus 5 et Fable 5 (après un `user`, en dernier ou suivi d'un
  `assistant`) ; pas par Sonnet 5.
- Un bloc `text` vide est refusé ; tous les `tool_result` d'une annonce vont
  dans **un** message `user`.

## Le harnais

- `scripts/passe_agent_code.py` lit le bloc `llm` du chat (`LlmProvider`,
  champ `provider`) ; `provider: "anthropic"` passe par `source()` →
  protocole `anthropic` → `model_source::connect_llm` (clé par
  `api_key_env`, `ANTHROPIC_API_KEY` par défaut ; une `address` sert de
  racine pour un mandataire).
- `connect_llm` rend `Box<dyn Llm>` depuis ce chantier : un seul type pour
  les deux clients ; `chat.rs` et le binaire suivent.
- Le backend de la passe doit porter `daemon,rag3db-native,code` ; depuis un
  worktree, lier la lib partagée de l'arbre principal (recette au rapport
  §6) plutôt que rebâtir le moteur.
- `RAG3WEAVER_SSE_DUMP=<fichier>` écrit le flux brut : c'est ainsi que les
  formes ci-dessus ont été relevées.

## La méthode de la passe 2 (à garder)

Demander au modèle de relire **notre** code de son propre protocole rend un
avis mêlé : trois points justes, un faux de bonne foi (sa connaissance de
son API est datée), un hors sujet. La règle qui tient : chaque point se
vérifie contre le flux réel (`SSE_DUMP`) avant d'être appliqué, et l'avis
se garde tel quel, avec le verdict point par point.
