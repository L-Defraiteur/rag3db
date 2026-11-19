# La place des couches : ce que coûte le doublon (2)

19 novembre 2026, au soir. Mesure demandée par c7, sur la pile du 15 novembre. **Pas de code produit** : une sonde,
des chiffres, une recommandation. Lucie lira.

## 1. La question

En 2a′-3, une couche d'histoire (une branche suivie) peut être **rattrapée** : sa pointe est écrite d'abord, puis les
commits plus anciens remplissent le trou. Une ligne écrite hors d'ordre devient un **doublon au bord haut du trou** :
née au rang r, retirée au plafond R, alors que la pointe porte déjà la même. C'est la forme (2), le défaut retenu.

Ce que la mesure de ED du 15 novembre (`docs/everything-declarative/15-11-2026/01-la-mesure-de-2a-prime-3.md`) n'avait
pas : **combien coûte ce doublon** en lignes, en octets et en temps de lecture, pour le mettre en face de (3a) et (3a′).

## 2. Comment

- **La base** : un reflink de la copie migrée aux versions et repassée de G
  (`lucie-tablette:~/.cache/rag3weaver-build/mesures/repetition-versions-code-rag3db-depot-20261119-194739`), jamais
  la copie elle-même. Arbre indexé : `b79878da9b`.
- **La sonde** : `history_layers::tests::sonde_place_des_couches` (`#[ignore]`), branche
  `arbre-principal/sonde-place-des-couches`, en pile sur le lot 8 (`rattrapage-de-l-histoire`), hors push. En release,
  sous `poste lourd`, lib 6e2476d41.
- **Le scénario** : la couche `refs/heads/master` est écrite à **la pointe**, 20 commits au-dessus de l'arbre indexé,
  puis les **19 commits plus anciens sont rattrapés**, du plus récent au plus ancien. Le dépôt est le worktree de
  rag3db.
- **L'embarqueur** est celui des témoins, un hachage à la dimension de la base (768). Les vecteurs n'ont pas de sens,
  mais ils ont la bonne taille. On compte la place, pas la recherche.

## 3. Les chiffres

### Les lignes

| Table | Base | Pointe (+) | Rattrapage de 19 commits (+) | dont retirées au plafond |
|---|---:|---:|---:|---:|
| `Scope` | 117 548 | 1 257 | **15 254** | **15 254** |
| `File` | 17 138 | 20 | 181 | 181 |
| `ScopeContent` (partagé) | 103 489 | 169 | 61 | — |
| `ScopeContent_Chunk` (partagé) | 161 873 | 422 | 151 | — |
| `File_Chunk` | 17 138 | 20 | 181 | — |
| `Scope_Chunk` | 131 587 | 0 | 0 | — |

- **Toutes les lignes du rattrapage sont retirées au plafond** : la pointe porte déjà la version la plus récente de
  chaque fichier.
- **Le doublon** (`layer_duplicates`, au grain du fichier) : **173 des 181 fichiers réécrits sont identiques** à leur
  version de la pointe, soit 96 %. Les 8 autres ont vraiment changé entre leur commit et la pointe.
- **Par commit rattrapé** : ~9,5 fichiers et ~800 lignes `Scope`, soit ~84 `Scope` par fichier réécrit.
- **En proportion** : 19 commits rattrapés ajoutent **13 % de lignes `Scope`** à toute la base, et **0,09 %** aux
  morceaux de contenu. Les contenus sont partagés par empreinte : un fichier réécrit à l'identique ne réécrit pas son
  texte.

### Les octets

- **La base** : 4 116 029 440 → 4 124 950 528 octets, soit **+8,5 Mio (+0,22 %)** pour la pointe et le rattrapage
  ensemble. Les vecteurs des ~600 morceaux neufs en font ~2 Mio.
- **Le texte des morceaux** : +0,64 Mo à la pointe, +0,21 Mo au rattrapage.
- **L'index plein texte** (`.fts`) : 1 836 → 1 837 Mio.

### Le temps

- **L'écriture** : la pointe en 12,2 s ; un commit rattrapé en ~5,4 s (de 1,7 à 8,0 s), 19 en ~102 s.
- **La lecture d'une vue** (le compte des `Scope` visibles, une seule requête par point) :

| Vue | ms |
|---|---:|
| la pointe, avant le rattrapage | 4,57 |
| la pointe, après le rattrapage | 5,51 |
| le rang 10, au milieu du rattrapage | 4,02 |

L'écart est de l'ordre du bruit d'une mesure unique. **Aucun coût de lecture ne se voit à cette échelle.**

## 4. (3a) et (3a′) sur les mêmes comptes

Les deux options donnent à une ligne **une identité par son contenu**. Un rattrapage à l'identique peut alors étendre
l'intervalle de la ligne de la pointe au lieu d'en écrire une seconde.

| | Lignes écrites par le rattrapage | Coût permanent, mesuré par ED le 15 nov. |
|---|---:|---|
| **(2)**, les deux colonnes | 15 254 `Scope` + 181 `File` | aucun (la référence) |
| **(3a)**, une table d'intervalles | ~670 lignes neuves (les 8 fichiers changés), plus une rangée d'intervalle par ligne de **toute** la base, ~135 000 pour `Scope` et `File` seuls | ×2 sur outline et usages, ×10 sur le compte ; 1,25 s de pose pour 80 000 intervalles |
| **(3a′)**, une annexe des réapparitions | ~670 lignes neuves ; l'annexe reste vide, puisque le rattrapage est contigu à la pointe | ×1,3 sur outline et usages, ×6 sur le compte, **même annexe vide** : le moteur n'élague pas l'`EXISTS` |

Les ~670 lignes sont une estimation : 8 fichiers changés × ~84 `Scope` par fichier.

## 5. Recommandation

**Garder (2).** Sur 19 commits rattrapés, le doublon coûte **13 % de lignes `Scope`**, mais **0,2 % d'octets** et
**aucun temps de lecture mesurable**. Les contenus partagés absorbent le texte. (3a) et (3a′) économiseraient ces
lignes, mais elles font payer **chaque lecture, pour toujours** (×1,3 à ×10).

**Le seuil où la question revient** : le doublon croît avec le nombre de commits rattrapés, d'environ 800 lignes
`Scope` par commit sur rag3db. **Extrapolé, non mesuré** : 100 commits rattrapés feraient ~80 000 lignes, autant que
deux tiers de la base, pour ~45 Mio. Si des rattrapages de cette taille deviennent courants, la piste est la variante
du § 5 de ED : **l'identité par le contenu, sans annexe**. Un rattrapage identique et contigu abaisserait alors le
`_depuis` de la ligne de la pointe au lieu d'en écrire une seconde. Elle effacerait 96 % du doublon mesuré ici, avec
le coût de lecture de (2). La trancher est l'affaire de ED.

## 6. Ce que la mesure ne dit pas

- **Un seul passage** : les temps de lecture sont une requête par point, sans répétition.
- **Un seul lecteur** : le compte des `Scope` visibles, pas outline, usages ni impact (ceux de ED).
- **Une seule forme de trou** : un trou de 19 commits contigu à la pointe. Une réapparition disjointe (A → B → A),
  qui seule remplirait l'annexe de (3a′), n'y est pas comptée.
- **`Scope_Chunk` ne bouge pas**, ni à la pointe ni au rattrapage. C'est vu, pas expliqué.
