# L'écrivain de 2a′-2 rougit à la parité lexicale de search_code

- **État** : ouvert (15 novembre 2026, 3 h 1x, jugement de banc)
- **Famille** : produit
- **Gravité** : réponse fausse — 15 cas sur 30 rendent une autre liste dans leurs dix premiers
- **Atteignable en service** : oui, dès que l'écrivain des générations est posé (pas encore sur master)
- **Touche rag3weaver** : oui (`src/code_sync.rs`, la synchronisation par générations ; peut-être le
  chemin lexical sous visibilité)
- **Pour** : everything declarative (ED)
- **Signalé par** : banc, jugement de `ecrivain-2a-prime-2` 7ddedb026 contre son parent 1105a2590
  (copies `banc-parite/ecr-base-parent` et `ecr-base-lot2` à luciepc)

## Ce que c'est

Les deux côtés partent de la même copie migrée, passée aux générations. Ils indexent la même source à
835d5298f, avec le même parseur (codeparsers 6ba8478b03), puis publient (rang 1). Sur les dix premiers
de `search_code` au rang courant, 15 cas sur 30 n'ont pas la même liste (chemin, nom, ligne). Tous les
meilleurs signaux y sont lexicaux.
Ce qui est vert à côté :
- avant l'index, les 30 cas au bit près ;
- `usages` sur 5 noms ;
- `aretes_recopiees` (2 844 à la génération 1) ;
- le montage de retrait (rangs 1 à 3).

## Deux causes candidates, non départagées

- **(A) Le saut des fichiers inchangés** (dd729317a). Un fichier dont l'empreinte indexée égale celle
  du disque est sauté : il garde tout ce que l'analyse d'avant en avait dérivé, c'est-à-dire le
  contenu des scopes, les morceaux et les documents du plein texte. Banc relève 61 résultats égaux
  dont le lot rend le hash d'avant l'index, alors que le parent rend un hash neuf
  (`third_party/yyjson/src/yyjson.h`, inchangé depuis février). La base migrée a été indexée par un
  code plus vieux. C'est un défaut à lui seul : sans version de l'analyse notée, un changement de
  parseur ne s'applique jamais aux fichiers inchangés.
- **(B) L'histoire dans le plein texte.** Les lignes fermées (`_jusqu_a` posé) gardent leurs
  documents lucivy. Elles sortent si la visibilité manque au chemin lexical, ou faussent les
  statistiques BM25.

## Les relevés qui tranchent (demandés à banc, sur `ecr-base-lot2`)

1. Les `_depuis` et `_jusqu_a` de chaque résultat du top 10 des 15 cas : un `_jusqu_a` au plus égal
   au rang courant désigne (B), visibilité absente.
2. Pour les résultats qui diffèrent, le fichier sauté et le hash de scope d'avant l'index : (A).
3. Les comptes Scope total et visibles, ScopeContent et documents du plein texte de chaque côté.

## Les témoins à écrire, rouges d'abord

- (A) : après un changement de la version de l'analyse, une synchronisation réanalyse un fichier
  inchangé.
- (B) : une ligne fermée ne sort pas d'une recherche sans vue.

## Ce qu'il faut pour le fermer

La cause établie par les relevés, les témoins rouges puis verts, et le correctif :
- (A) : une version de l'analyse notée par source, dont le changement fait tout repasser, ou le retrait
  du saut ;
- (B) : selon le relevé.
Puis une parité de banc verte sur les 30 cas.
