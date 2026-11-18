# Un plafond par texte en jetons, exact

- **État** : ouvert (15 novembre 2026, embarquement ; décision de c7, option b)
- **Pour** : embarquement
- **Famille** : produit
- **Gravité** : haute depuis le 18 novembre (souhait avant) : un texte sous le plafond en caractères a fait paniquer
  le calcul de bge-m3 sur la carte ; voir « Le relevé du 18 novembre ».
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

## Le relevé du 18 novembre (G, fenêtre de bascule, 15 h 32)

Le témoin de parité de G a envoyé au 7879 (bge-m3, binaire a70f1e46) un texte de **11 000 « é »** : 11 000
caractères, sous le plafond de 12 000, mais **11 002 jetons**. Le calcul a paniqué sur la carte : « calcul en panique
sur la carte : Expand 8194 != 11002 ». bge-m3 n'a que 8 194 positions.

- **Le plafond en caractères ne protège donc pas** un texte dense en jetons : ici, environ 1 jeton par caractère, et
  non 3 caractères par jeton. Le service doit compter en jetons et couper, ou refuser par un refus nommé, **avant**
  la carte.
- **Lu dans le code, le 18** : `src/burn_bge_m3_embedder.rs` règle le rembourrage du tokenizer (`with_padding`), mais
  **aucune troncature**. Le `tokenizer.json` n'en porte pas non plus, sinon 11 002 jetons ne seraient pas arrivés au
  modèle. La conception ci-dessus suppose « un tokenizer réglé avec sa troncature » : c'est faux au moins pour
  bge-m3. Il faut relire les autres embarqueurs burn (granite, minilm, multilingual) avant le lot.
- **Témoin à écrire, rouge d'abord** : un texte de 11 000 « é » envoyé à l'embarqueur bge-m3 (ou au service) rend un
  vecteur coupé à la fenêtre du modèle, ou un refus nommé, jamais une panique.
- Pas de code avant le tour d'embarquement (c7, 18 novembre).
