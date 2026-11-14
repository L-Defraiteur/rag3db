# Une recherche sans filtre paie le masque des versions entier, à chaque appel

- **État** : ouvert (14 novembre 2026).
- **Famille** : produit
- **Gravité** : lenteur. Environ 0,8 s ajoutées à chaque `search_code` sans filtre sur une base à versions, pour un index de la taille du dépôt rag3db (79 662 occurrences)
- **Atteignable en service** : oui, dès que 1b fait monter une vraie base aux versions. Aucune base de Lucie ne l'est encore
- **Touche rag3weaver** : oui (`src/dataflow/generic_search_nodes.rs` : `occurrence_roots_for`, `occurrence_mask_options`)
- **Pour** : everything declarative
- **Signalé par** : everything declarative, mesure du drapeau `tout_visible` de 2a′-2 le 14 novembre 2026, sur une copie de `code-rag3db-depot` migrée par 1b (page `docs/everything-declarative/14-11-2026/04-le-pas-2a-prime-2-l-ancre-immuable.md`, § 2)

## Ce que c'est

Sur une base à versions, une recherche projette ses résultats, qui sont des contenus partagés
(`ScopeContent`), vers les occurrences publiques (`Scope`). Pour cela, `occurrence_mask_options`
calcule à chaque appel le **masque des versions**, en trois temps :
1. **les racines** : sans filtre, le repli d'`occurrence_roots_for` lit tous les `_uuid` de `Scope`
   par `select_all` ;
2. **les références** : `derived_references` est joué par lots de 5 000 racines, ce qui fait 16
   requêtes pour 79 662 racines ;
3. **le filtre** : `_uuid IN [...]` sur toutes les versions trouvées, passé à la recherche.

Sur la copie, sous `poste mesure`, avec 3 tours de chauffe puis 20 tours :

| étape | médiane | p95 |
|---|---|---|
| (a) les racines, `select_all` | 19,7 ms | 20,9 ms |
| (b) le masque, `derived_references` par lots | **788,9 ms** | 839,3 ms |

Le masque ne dépend que de l'état publié de la base. Il est pourtant recalculé à chaque appel, et
c'est presque tout le coût d'une recherche sans filtre.

## Ce qu'on pourrait faire

C'est la forme de repli que proposait la mesure du drapeau (page 04, § 2), étendue au masque, comme
le demande c7 :
- garder en mémoire, par (couche, rang) publié, **l'ensemble des offsets visibles et le masque des
  versions qui en découle** ;
- le calculer une fois, à la publication de la génération (ou à l'ouverture pour la génération
  courante) ;
- le rendre tel quel à chaque recherche sans filtre.

Un rang publié ne change jamais, et ses références non plus : il n'y a rien à invalider entre deux
publications. La couche de travail de l'ancre, réécrite sur place, doit garder son propre calcul,
ou invalider à chaque écriture. C'est à trancher avec les chiffres de B et C.

## Comment le voir

`tests/sonde_recherche_sans_filtre.rs` sur une copie d'une base à versions, avec
`RAG3WEAVER_SONDE_BASE` et `RAG3WEAVER_SONDE_MANIFESTE` : les lignes `(b) masque`.
