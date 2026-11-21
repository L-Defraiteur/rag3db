# Un bâti sans `token-count` refuse d'ouvrir une base sous granite, même pour lire

- **État** : ouvert (21 novembre 2026, nuit)
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
