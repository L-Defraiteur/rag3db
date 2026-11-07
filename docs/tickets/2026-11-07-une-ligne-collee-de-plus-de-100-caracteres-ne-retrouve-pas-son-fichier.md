# Une ligne collée de plus de 100 caractères ne retrouve pas son fichier par le signal exact

- **État** : ouvert, le 7 novembre 2026 ; pour la session recherche, à fermer avec la montée de
  lucivy qui corrige les passages exacts longs (4.3.2 attendue).
- **Famille** : produit
- **Gravité** : réponse fausse. Une phrase collée dans l'IDE ou passée à `search_code` ramène un
  autre fichier premier, sans le dire.
- **Atteignable en service** : oui, pour toute requête de plus de 100 caractères qui reprend un
  passage d'un fichier indexé (une ligne collée, un prompt cité).
- **Touche rag3weaver** : oui (la requête entière de `BM25SearchNode`, en strict au-delà de
  `MAX_FUZZY_WHOLE_QUERY_CHARS`, 77d0b0676).

## Ce que c'est

Au-delà de 100 caractères, la requête entière se cherche à distance 0 (la borne du coût,
77d0b0676). Or lucivy ne trouve plus un passage exact d'un document au-delà de 48 octets en strict
et de 64 en relâché, à un comme à quatre shards : la requête entière ne rend rien, le signal exact
(`bm25_exact`, dominant) manque, et la fusion classe premier ce que le vecteur et les mots trouvent.

## La cause

Chez lucivy : ticket « un passage exact long n'est plus trouvé »
(`docs/tickets/2026-11-07-un-passage-exact-long-n-est-plus-trouve.md`, branche lucivy
`tickets-2026-11-07`, sonde `probe_exact_passage_lengths`). La borne du produit n'en est pas la
cause : elle reste juste pour le coût (le flou coûtait 112 s à 197 termes) ; elle envoie seulement
la requête là où lucivy ne trouve plus.

## Le témoin

`e2e_lectures::une_ligne_collee_de_120_caracteres_retrouve_son_fichier` (branche
`passage-exact-long`, d75b73466), par le chemin de l'hôte : une ligne de 115 caractères (118
octets) collée d'un fichier indexé ; rouge le 7 novembre (lib 17a094d12) — premier
`src/sources.md` par `vector` et `bm25`, sans `bm25_exact`, au lieu de `src/reconciliation.md`.

## Ce qu'il faut pour le fermer

La montée de lucivy qui corrige les passages exacts longs ; le témoin vert sur elle, puis il entre
sur master avec la montée et ce ticket passe dans `closed/`.
