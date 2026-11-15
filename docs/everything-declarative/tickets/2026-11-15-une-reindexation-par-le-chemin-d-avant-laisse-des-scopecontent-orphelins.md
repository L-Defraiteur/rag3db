# Une réindexation par le chemin d'avant laisse des ScopeContent orphelins

- **État** : ouvert (15 novembre 2026)
- **Famille** : produit
- **Gravité** : réponse fausse. Les versions de contenu orphelines restent dans le plein texte et
  pèsent sur les statistiques BM25 du lexical ; à cela s'ajoute leur coût en place
- **Atteignable en service** : oui. Toute réindexation, par `sync_source` ou `reingest_file`, d'une
  base à versions de contenu qui n'est pas passée aux générations
- **Touche rag3weaver** : oui (`src/code_sync.rs`, le chemin d'instantané ; les versions de contenu
  `ScopeContent`)
- **Pour** : everything declarative (ED)
- **Signalé par** : banc, comptes du départage de la parité de l'écrivain (15 novembre 2026, vers 3 h 40) ;
  voir `docs/everything-declarative/tickets/2026-11-15-l-ecrivain-de-2a-prime-2-rougit-a-la-parite-lexicale.md`

## Ce que c'est

Banc a réindexé une copie migrée de `code-rag3db-depot` par le chemin d'avant (le parent
1105a2590 de l'écrivain), à la source 835d5298f. Il compte 77 149 Scope et 84 211 ScopeContent, dont
**14 201 orphelins** : des versions de contenu que plus aucun Scope ne cite. Le lot de l'écrivain
(générations), sur la même base et la même source, n'en compte **aucun** (74 274 ScopeContent).

## La cause (banc)

La fin d'instantané (`apply_snapshot_finish`) efface les occurrences que la source n'a plus, mais
pas leurs versions de contenu (`ScopeContent`, à identité de contenu), qu'elle exclut. Elles restent
donc sans origine (`NOT EXISTS` sur `ScopeContent_DERIVED_FROM`). Sur la copie de banc, un document
sur six est orphelin.
- Le masque des versions les écarte des résultats, mais elles restent au plein texte des versions :
  le BM25 compte leurs documents et leurs termes (nombre de documents, fréquences, longueur moyenne).
- Elles forcent aussi le masque même sans filtre, donc le chemin lent : le court-circuit sans filtre
  ne vaut que sans orpheline (ticket d'arbre principal
  `2026-11-14-la-recherche-a-versions-masque-toutes-les-occurrences-sans-filtre`).

La preuve : la sonde `releve_des_lignes_par_cypher`
(`extension/rag3weaver/tests/sonde_parite_search_code.rs`, branche `banc-parite-search-code`
1927bfd463) ; le relevé `banc-parite/sorties/ecrivain-2/depart-parent.json` à luciepc ; le jugement,
`docs/banc/14-11-2026/02-le-juge-de-parite-de-search-code.md` § 6. Ce ticket reprend celui que banc
avait ouvert en même temps (`docs/banc/tickets/…-les-versions-orphelines-du-chemin-d-avant-faussent-
le-bm25-d-une-base-reindexee.md`, fermé comme doublon).

## L'argument de l'écrivain

Sous les générations, une version de contenu ne se perd pas : la ligne qui la cite est retirée, pas
réécrite. Elle reste citée par sa ligne fermée, et la rétention les effacera ensemble. Le chemin
d'avant réécrit la ligne sur place et laisse l'ancienne version sans lien. Passer une base aux
générations met fin au défaut pour elle ; il reste pour les bases à versions qui n'y passent pas.

## Le témoin

Pas encore. Sur une base à versions sans générations : indexer, changer le corps d'une fonction,
réindexer le fichier, puis compter les `ScopeContent` qu'aucun `Scope` ne cite. On en attend 0 ;
rouge attendu sur master.

## Ce qu'il faut pour le fermer

La lecture de c7 (15 novembre 2026, nuit, rapportée par banc) : le ticket est pour ED. La bascule
d'une base vers les générations met fin à ses orphelines ; d'ici là, un ménage sur une copie se fait
seulement si une mesure l'exige.


Soit la fin d'instantané reprend une version quand sa dernière occurrence part, ou la retire du plein
texte avec elle, avec le témoin ci-dessus vert et les comptes du plein texte des versions égaux aux
lignes. Soit toutes les bases à versions passent aux générations, et le chemin d'avant n'écrit
plus de versions. Dans les deux cas, une purge des orphelins existants, sur une copie d'abord.
