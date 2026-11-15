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

## Deux causes candidates

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

## Le départage (banc, 15 novembre 2026, vers 3 h 40) : c'est (A)

- **(B) écarté.** Aucun des 128 uuid du top 10 du lot dans les 15 cas rouges n'est fermé : tous sont à
  `_depuis` 0 et `_jusqu_a` NULL. La requête des non visibles au rang courant ne rend aucune ligne.
- **(A) confirmé.** À uuid égal, le lot garde le hash de l'analyse d'avant, et le parent en calcule
  un neuf. Exemple, la fonction `index` de `experiments/mtga/scripts/*_variant.py` : le parent rend un
  seul hash (afaba06…) pour les trois variantes au contenu identique ; le lot en rend trois, ceux de la
  base migrée.
- **Les comptes.** Au parent : Scope 77 149 ; ScopeContent 84 211, dont **14 201 orphelins** laissés
  par le chemin d'avant. Au lot, dans l'état du retrait : Scope 81 919, dont 77 239 visibles et 4 673
  fermés ; ScopeContent 74 274, aucun orphelin.
- **La parité est mal posée sur une base migrée** indexée par un code plus vieux. La parité propre
  part d'une base indexée par le code du parent lui-même, copiée, puis réindexée des deux côtés sur
  une source qui diffère de quelques fichiers. Banc la jouera après le correctif. L'actuelle sera
  rejouée elle aussi, sur une copie neuve, et devra passer aux orphelins du parent près.
- **Les orphelins du parent** sont un défaut du chemin d'avant, pas de ce lot ; ils faussent le BM25
  de toute base réindexée par lui. Un ticket à part, chez banc ou ici.

## Les témoins à écrire, rouges d'abord

- (A) : après un changement de la version de l'analyse, une synchronisation réanalyse un fichier
  inchangé.
- (B) : une ligne fermée ne sort pas d'une recherche sans vue.

## Ce qu'il faut pour le fermer

La cause établie par les relevés, les témoins rouges puis verts, et le correctif :
- (A), décidé par c7 (15 novembre, 3 h 1x) : ni retirer le saut, ni une version par source seule.
  **La version de l'analyse (parseur, règles d'import, gabarit) entre dans l'empreinte indexée** d'un
  fichier, et un fichier n'est « inchangé » que si son contenu et cette version coïncident ; sinon il
  repasse. Le témoin a deux branches : la version change, et un fichier au contenu identique est
  réanalysé ; la version ne change pas, et il est sauté. C'est ce qui rend applicable sans réindexer à
  la main un changement de parseur comme le passage function → method de codeparsers ;
- (B) : selon le relevé.
Puis une parité de banc verte sur les 30 cas.
