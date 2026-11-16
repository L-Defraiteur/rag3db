# Les anciens morceaux géants sont à réécrire

- **État** : ouvert (16 novembre 2026, embarquement ; demande de c7)
- **Pour** : embarquement
- **Famille** : produit
- **Gravité** : moyenne. Un index déjà posé garde des morceaux de dizaines de milliers de caractères, dont le
  vecteur ne porte que le début (coupé au plafond du service depuis 828ff28153).
- **Atteignable en service** : oui (les bases de code de Lucie, le dépôt rag3db : 31 morceaux de plus de
  12 000 caractères dans l'index B′ du banc, jusqu'à 53 294)
- **Touche rag3weaver** : oui (`src/catalog.rs`, `src/dataflow/index_nodes.rs`)

## Ce que c'est

Le découpeur des `ScopeContent_Chunk` coupe par lignes : une ligne énorme fait un morceau énorme. Le défaut
d'amont est chez ED : `docs/everything-declarative/tickets/2026-11-16-le-decoupeur-laisse-passer-un-morceau-de-53-000-caracteres.md`.
Une fois le découpeur corrigé, **les nouveaux index** seront bons, mais les index déjà posés gardent leurs
morceaux géants : rien ne les redécoupe tant que leur fichier ne change pas.

## Ce qu'il faut faire

Sur le modèle de la réécriture des morceaux sans texte (`Catalog::chunks_sans_texte`,
`reecrire_les_morceaux_sans_texte`, appelées par `run_index` avant les vecteurs, avec la variable de
suspension `RAG3WEAVER_REECRIRE_LES_VIDES`) :
1. `Catalog::oversized_chunks(factor)` : les morceaux dont le texte dépasse `factor × max_size` du découpage
   de leur entité (défaut `factor = 4`), comptés par table.
2. `Catalog::rewrite_oversized_chunks(factor)` : leurs entités redécoupées depuis leur contenu (le même
   chemin que les morceaux sans texte), les morceaux neufs entrant dans la dette de vecteurs.
3. `run_index` l'appelle avant les vecteurs et le dit au journal : « N morceau(x) géant(s) : M entité(s)
   redécoupée(s) — il en reste K ». Une variable `RAG3WEAVER_REWRITE_OVERSIZED=0` la suspend (anglais).

**Ce qui ne se touche pas** : le découpeur lui-même (le ticket d'ED) ; les morceaux sous le facteur.

## Comment le prouver

- `oversized_chunks_are_rewritten_before_the_vectors` (lib, `index_nodes.rs`) : un catalogue où un morceau
  de 53 000 caractères est écrit à la main ; après `run_index`, il n'y a plus aucun morceau au-dessus du
  facteur, la ligne du journal le dit, et leur concaténation rend le texte. Rouge d'abord.
- Suites exigées par chemins : `catalog.rs` → `e2e_code_sync` ; le journal → `e2e_lectures`.

## Taille

Une demi-journée.

## Ce qui le bloque

Le ticket d'ED sur le découpeur, atterri d'abord : sans lui, la réécriture referait le même morceau géant.
