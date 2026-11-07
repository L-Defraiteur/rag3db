# Le poids des positions du creux, par le service d'embarquement (7 novembre 2026)

*Lot demandé par l'orchestration pour que l'hôte de Lucie surligne en creux sans charger
`burn-embedder` : branche `spans-creux`.*

## L'option

`POST /embed_sparse` avec `{"texts": […], "spans": true}` rend, en plus du creux,
`spans: [[début, fin, poids], …]` par texte : la plage d'octets de chaque jeton dans le texte
reçu (offsets du tokenizer) et son poids avant la fusion au maximum. Les jetons spéciaux
(et UNK, comme dans le creux) n'ont pas de position ; une position de poids nul est gardée pour
que chaque mot soit couvert. Sans l'option, la réponse ne change pas. Aucun calcul de plus :
les offsets étaient jetés, les poids par position déjà calculés.

**Une plage peut commencer par l'espace qui précède le mot** : le Metaspace de BGE-M3 range
l'espace dans le jeton suivant (« ␣chat »). Un surligneur retire l'espace de tête.

Côté Rust : `SparseEmbedder::embed_sparse_spans` rend `(SparseVector, Vec<SparseSpan>)`.
BGE-M3 burn l'implémente, le factice par mot aussi (de quoi tester un surligneur sans modèle),
les autres le refusent en le disant. `DaemonEmbedder::embed_sparse_spans` envoie l'option ; un
service d'avant l'option répond sans `spans`, ce qui se dit (« champ 'spans' absent »).

## Témoins

- lib : la fusion d'une ligne (`fondre`, sans modèle) ; l'option sur le fil avec le factice ;
  un embarqueur creux sans positions qui le dit.
- `e2e_service_spans`, contre le service bge-m3 de `RAG3WEAVER_EMBED_SERVICE` : rouge sur
  l'ancien binaire (« champ 'spans' absent »), vert après le redéploiement. Sur « Le chat dort
  sur la base de données. » : 9 positions, 9 valeurs (chat 0,310, dort 0,281, données 0,269…).

En passant : le binaire `rag3weaver-embeddings` (daemon, burn-embedder, burn-ocr) ne se bâtissait
plus depuis 80ea996c2 (`Rag3dbConnection` hors de `rag3db-native` dans `verify_files`) ; la
base d'une vérification à blanc s'ouvre maintenant par une fonction à deux faces.

## Le service de luciepc

`rag3weaver-embeddings@bge-m3` (7879) redéployé le 7 novembre à 21 h 18 sur le binaire du lot
(sha256 65ec1d1db17d48ee) : 15 s de coupure, dense à écart nul sur un vecteur noté, relecteur,
OCR et creux présents. L'ancien binaire est gardé en `~/.local/bin/rag3weaver-embeddings.avant-spans`.

**Reste** : le binaire `~/.local/bin/rag3weaver-embeddings` est commun aux trois unités ;
`@granite-278m` (7878) et `@granite-107m` (7880) tournent encore sur l'ancien en mémoire et
prendront le neuf à leur prochain redémarrage. Ce redémarrage se fait annoncé, avec la parité
du dense de chacun sur un vecteur noté.
