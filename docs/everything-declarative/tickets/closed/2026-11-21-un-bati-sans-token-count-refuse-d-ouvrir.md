# Un bâti sans `token-count` refuse d'ouvrir une base sous granite, même pour lire

- **État** : fermé le 21 novembre 2026, par le lot `token-count-only-for-chunking`.
- **Famille** : produit (l'ouverture d'une base de code, le découpage)
- **Gravité** : grave pour un bâti sans `token-count` : une base sous granite ne s'ouvre plus du tout, et le chat de
  l'IDE répond « il faut la feature token-count ». Apporté par le lot `chunk-bound-from-embedder` (règles 14), le mien.
- **Atteignable en service** : oui, l'hôte posé cette nuit l'a montré (G le rebâtit avec les features de service).
- **Touche rag3weaver** : oui (`src/backend.rs`, l'enregistrement du schéma de code ; `src/dataflow/record_nodes.rs`).
- **Pour** : everything declarative
- **Signalé par** : c7, d'après le rouge de l'hôte de la nuit.

## La cause

À l'ouverture, `backend.rs` (vers la ligne 3129, l'enregistrement du schéma de code) charge le compteur de jetons du
plus petit embarqueur (`crate::tokens::counter`) et refuse sans lui. Or lire, chercher et converser ne comptent aucun
jeton : seul le découpage d'un index en a besoin.

## Un piège pour le correctif

Ôter ce refus ne suffit pas. `Catalog::warm_chunker_cache` laisse hors du cache un découpage sans compteur, en
comptant sur ce refus ; et `record_nodes.rs` (vers la ligne 1813) écrit alors une ligne **sans aucun morceau**, en
silence, quand son découpeur manque. La réindexation d'un fichier par la montre écrirait des scopes sans morceaux.
Seul le chemin d'ingestion de `catalog.rs` (`Chunker::try_new`, vers la ligne 6875) refuse déjà.

## Ce qu'il faut pour le fermer

- Un bâti sans `token-count` ouvre et sert la base : lecture, recherche, chat.
- Toute écriture de morceaux qui compte des jetons refuse, en le disant (le message actuel de `tokens::counter`) : le
  chemin d'ingestion, la réindexation d'un fichier, la reprise de la dette. Jamais une ligne écrite sans ses morceaux.
- Témoin rouge sous `cfg(not(feature = "token-count"))` : une base sous granite s'ouvre et répond à une recherche ; un
  fichier réindexé est refusé en nommant la feature, et ne laisse aucun scope sans morceaux.

## Fermeture (21 novembre 2026, nuit)

- L'ouverture ne charge plus le compteur de jetons : une base sous granite s'ouvre et sert sans `token-count`.
- `ChunkRecordNode` refuse, en nommant la feature, toute ligne à découper dont le découpeur manque : plus jamais une
  ligne écrite sans ses morceaux. Le chemin d'ingestion du catalogue refusait déjà (`Chunker::try_new`).
- Témoin `a_build_without_token_count_opens_a_granite_base_and_refuses_only_its_chunks`, sous
  `cfg(not(feature = "token-count"))` : rouge avant le lot (la base refusait de s'ouvrir), vert après (elle s'ouvre,
  `read_file` lit, l'ingestion est refusée en nommant la feature, et aucun scope n'est écrit).
- La lib entière est verte avec `token-count` (2 808), comme la joue le pousseur. Sans la feature, 42 tests qui
  indexent sous granite échouent sur ce refus voulu ; sur master, ils échouaient déjà, plus tôt, à l'ouverture.
