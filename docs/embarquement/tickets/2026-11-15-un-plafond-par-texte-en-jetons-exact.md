# Un plafond par texte en jetons, exact

- **État** : ouvert (15 novembre 2026, embarquement ; décision de c7, option b)
- **Pour** : embarquement
- **Famille** : produit
- **Gravité** : souhait. Le plafond d'un texte du service d'embarquement se compte en caractères, pas en jetons.
- **Atteignable en service** : oui
- **Touche rag3weaver** : oui (le trait `Embedder`, les embarqueurs burn, `src/daemon/embeddings.rs`)

## Ce que c'est

Le service d'embarquement refuse depuis le 15 novembre un texte au-dessus de `max_chars_per_text` caractères
(12 000 par défaut, `RAG3WEAVER_EMBED_MAX_CHARS`). Il dit dans son identité le ratio qui traduit ce plafond en
jetons : `chars_per_token`, environ 3. Le trait `Embedder` ne compte pas les jetons, et aucun embarqueur n'expose son
tokenizer. Or la mémoire de l'attention, celle qui a tué le 7879, croît comme le carré du nombre de jetons, pas des
caractères. Un texte dense en jetons (du code, des symboles) peut donc dépasser l'estimation.

## Ce qu'il faudrait

- une méthode `count_tokens(&self, texte) -> Option<usize>` au trait `Embedder`, implémentée par chaque embarqueur
  burn (bge-m3, granite, minilm, multilingual) avec son propre tokenizer ;
- un plafond `max_tokens_per_text` dit dans l'identité, et le même refus nommé (413) quand il est dépassé.

À faire si un modèle à jetons courts ou un corpus de symboles le demande. Témoin : un texte sous le plafond en
caractères, mais au-dessus en jetons, est refusé et nommé.

## La conception, en une ligne (c7, 15 novembre au soir)

Chaque embarqueur burn (bge-m3, granite, minilm, multilingual) tient déjà un `tokenizer: Mutex<Tokenizer>`, réglé
**avec sa troncature** (`TruncationParams`). `count_tokens` doit donc compter **avant** la troncature : encoder sans
elle (une copie du tokenizer, `with_truncation(None)`), ou lire `Encoding::get_overflowing()` d'un encodage tronqué
(non vide : le texte dépasse). Un `encode(…).len()` simple rendrait le plafond du modèle, jamais plus.
