# Le juge de la porte appelle son modèle hors du trait `Llm`, et la garde du registre ne voit pas un appel HTTP direct

- **État** : ouvert (22 novembre 2026, matin)
- **Famille** : produit (le registre des dépenses, la porte du mode auto)
- **Gravité** : une lacune de couverture, pas une dépense perdue aujourd'hui : le juge est local et gratuit. Elle le devient le jour où un juge payant est appelé de la même façon.
- **Atteignable en service** : oui, dès que la v1 de la porte atterrit : ses appels ne passent pas par `MeteredLlm`.
- **Touche rag3weaver** : oui (`llm.rs`, `openai_llm.rs`, `usage_ledger`, `daemon_host/gate.rs`).
- **Pour** : everything declarative.
- **Signalé par** : archi et c7 (22 novembre 2026, matin), à la lecture du lot `ed-billing-ledger`.

## Ce qui se passe

Le juge de la porte (`daemon_host/gate.rs`, `served_judge`, un Qwen local à `127.0.0.1:8096`) lit les `top_logprobs` de
la réponse et passe `chat_template_kwargs` à llama-server. Le trait `Llm` ne rend ni l'un ni l'autre : le juge appelle
donc la route `/chat/completions` lui-même, sans `connect_llm`, et sans `MeteredLlm`.

La garde `no_paid_client_is_built_outside_the_metered_paths` (`usage_ledger/tests.rs`) cherche les constructions de
client (`OpenAiLlm::new`, `AnthropicLlm::new`…) hors des chemins comptés : un appel HTTP direct vers une route de modèle
lui échappe.

En attendant, archi écrit les appels du juge au registre à la main (`usage_ledger::user_ledger().reserve`, puis
`settle`) : chaque appel laisse une ligne, à prix zéro par `free_private_hosts`.

## Ce qu'il faut

1. **Le trait `Llm` aux logprobs** : une option de `GenOptions` (`top_logprobs: Option<u8>`) et un champ de `Finish`
   qui les rend, remplis par `OpenAiLlm` quand le service les donne ; et un passage des paramètres propres au gabarit de
   conversation (`chat_template_kwargs`) dans `GenOptions`. Le juge se construit alors par `connect_llm`, et passe par
   `MeteredLlm` comme tout modèle.
2. **La garde de source élargie** : outre les constructions de client, un appel HTTP vers une route de modèle
   (`/chat/completions`, `/v1/messages`, `:generateContent`) hors de `openai_llm.rs` et `anthropic_llm.rs` rougit, sauf
   dans un fichier qui écrit au registre (`usage_ledger::`).

## Le témoin

- le juge de la porte, construit par `connect_llm`, rend ses logprobs, et son appel laisse une ligne `free` au
  registre ;
- un fichier de `src/` qui poste vers `/chat/completions` sans passer par le registre fait rougir la garde.

## La recherche de prix passe aussi par `web_search` (23 novembre 2026)

La recherche du prix d'un modèle qui n'en a pas (`/ide/prices/lookup`, puis le prix provisoire que l'hôte pose seul,
lots `ed-price-card` et `ed-provisional-price`) appelle `web_search`, donc Gemini sur Vertex. Tant que le lot
`ed-llm-logprobs`, qui fait passer `web_search` par le registre, n'a pas atterri, **chaque recherche de prix est un
appel payant non compté**. La réserve se ferme avec ce lot ; le prix par requête de Google Search reste au ticket
`2026-11-22-la-recherche-web-compte-ses-jetons-pas-sa-requete`.
