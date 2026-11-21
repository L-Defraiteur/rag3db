# Embarquement : partager le vecteur d'un texte entre ses morceaux

Demande de c7 (21 novembre, après la reprise par empreinte) : le second levier sur le coût d'ouverture d'une vue.
**Un vecteur n'est écrit sous HNSW qu'une fois par texte**, et les morceaux d'un même texte, dans plusieurs versions de
scope, le partagent au lieu de le réécrire. Page de conception, avant tout code ; c'est la couche d'ED, son avis est
demandé.

## Ce qui existe

- **Le partage par contenu** : une version de scope (`ScopeContent`) est repérée par son contenu. Deux occurrences au même
  contenu partagent la version, ses morceaux et ses vecteurs (page 06 d'ED, 21 novembre).
- **Ce qu'il ne couvre pas** : un scope qui change un peu a un contenu neuf, donc des morceaux neufs. Or la plupart de
  ces morceaux ont le même texte qu'à la version d'avant (ou qu'ailleurs). Mesuré : **74 % des morceaux d'une vue** ont un
  texte que la base embarque déjà, et **18 % des vecteurs d'une base** sont des textes répétés.
- **La reprise par empreinte** (lot `reuse-by-hash`, master 13d0e50333) évite déjà le calcul du modèle pour eux. Elle
  n'évite pas **l'écriture** : chaque morceau reçoit sa copie du vecteur, sous HNSW, à **36,6 ms** le vecteur.
- **La recherche dense** (`Rag3dbSearchBackend::vector_search_filtered`) projette en graphe
  `MATCH (n:<table des morceaux>) <motif> WHERE <condition> RETURN n`, puis cherche par HNSW (`QUERY_VECTOR_INDEX`) dans
  ce graphe.

## Deux schémas

| | **A. Le morceau partagé** | **B. Un nœud de texte à côté du morceau** (proposé) |
|---|---|---|
| Ce qui change de forme | Un morceau par texte, porté par plusieurs occurrences | Une table `<Entité>_Text`, repérée par `_text_hash`, qui porte le vecteur ; le morceau pointe vers elle (`HAS_TEXT`) |
| Les bornes (lignes, position) | Passent sur l'arête : **tous leurs lecteurs changent** | Restent sur le morceau : rien ne change pour eux |
| Le plein texte et le creux | Sur le morceau partagé | Restent sur le morceau |
| L'index HNSW | Inchangé (sur les morceaux) | Passe sur `<Entité>_Text` |
| Ce qui change de code | L'écriture des morceaux, leurs lecteurs, la recherche | L'écriture des vecteurs, leur fraîcheur, la recherche dense |

**B** touche bien moins de code. C'est la forme proposée.

## B en détail

- **L'écriture** : pour chaque morceau, `MERGE` du nœud de texte par `_text_hash`, puis l'arête `HAS_TEXT`. Seuls les textes
  **sans vecteur pour le modèle courant** vont au modèle, puis sous HNSW. Un texte ne change jamais : un vecteur posé sur
  lui reste juste pour ce modèle, sans marque de fraîcheur à tenir.
- **La recherche dense** : HNSW sur l'index de `<Entité>_Text`, **le filtre de vue dans la recherche**, par le graphe
  projeté, jamais après. Sinon, les k textes les plus proches peuvent tous être invisibles, et la recherche rend trop peu
  (le piège nommé par ED). La couche ne vit pas sur le morceau : elle vit sur l'occurrence. Le chemin est
  `Text <-HAS_TEXT- Chunk -CHUNKED_FROM-> ScopeContent <-(dérivée)- Scope`, avec `Visible(couche, rang)` sur `Scope`. Le
  filtre réutilise le masque des occurrences que le signal dense applique déjà (`occurrence_mask_options`,
  `Visibilite::clause_sur`, `compile_filter_for_vector_in` dans `read_view.rs`), allongé de deux sauts. Un résultat (un
  texte) se rend en ses morceaux visibles, puis en leurs lignes ; un même texte en deux endroits visibles rend deux
  résultats au même score.
- **Le retrait d'une version** : ses morceaux partent avec leurs arêtes. Un texte qui n'a plus aucun morceau part avec
  son vecteur, au même passage ou par un balayage à part.
- **Plusieurs modèles** : une colonne et un index par modèle sur `<Entité>_Text`, comme aujourd'hui sur les morceaux.
- **La migration des bases existantes** : une montée en versions par table de morceaux. Créer `<Entité>_Text` (un nœud par
  `_text_hash` distinct), y copier le vecteur d'un morceau au vecteur frais, poser les arêtes, bâtir l'index HNSW du texte,
  puis retirer la colonne et l'index des morceaux. Sur la base du dépôt : environ 133 000 textes pour 161 873 morceaux. Le
  bâti de l'index se paie une fois.
- **Ce qui reste à départager avant le code** :
  - **le coût de la projection à deux sauts** (texte ← morceau, plus le motif de vue) à chaque recherche, à mesurer sur la
    copie ;
  - **les autres dialectes** (postgres) : la même forme en SQL (une table de textes, une jointure).

## L'avis d'ED (21 novembre)

**B lui va** ; A casserait bien plus, et rien dans sa couche ne le ferait préférer. Ses trois faits sont repris
ci-dessus : le chemin de la vue, le filtre dans la recherche, et ceci :

- **Le gain se mesure à règles égales avant de s'annoncer.** Les morceaux sont déjà partagés par contenu ; le nœud de texte
  ne gagne que les **textes identiques dans des contenus différents**. Les 74 % d'une vue ont été mesurés sous des règles
  différentes de celles de la base, d'où un chiffre trop haut. Les 18 % de doublons de la base, eux, tiennent.
- **Le texte embarqué est bien celui du morceau** : `EmbedNode` embarque le champ texte du morceau, et `_text_hash` vaut
  `content_hash(chunk.text)` (`ChunkRecordNode`). Sur la copie, la marque `_embed_hash__granite_278m` égale `_text_hash`
  pour tous les morceaux qui portent un vecteur.

## Les témoins, rouges d'abord

1. Deux contenus différents qui partagent un morceau de même texte : **un seul vecteur écrit**, et chacun le trouve.
2. **Une vue où les textes les plus proches sont invisibles rend quand même ses k résultats visibles** (le filtre dans la
   recherche).
3. Le retrait de la dernière version qui porte un texte retire son vecteur ; un texte encore porté le garde.

## Le gain, mesuré à règles égales (21 novembre, luciepc, release)

La sonde d'écriture en couche d'ED (branche `sonde-ecriture-en-couche-2`, 14bb82024b, aux règles 13 de la base), sur un
reflink neuf de la copie de G en règles 13 (`mesures/embarquement-partage-20261121`, à luciepc), lib 424ce93e56. Le lot :
l'arbre du 20 novembre à 22 h 30 (3e5279cef0), 278 fichiers d'écart avec l'arbre indexé.

- **La vue** : 5 585 contenus neufs, **7 156 morceaux neufs** (7 146 textes distincts). `derive_embed` : 53,6 s, dont
  **53,2 s d'écriture** (7,4 ms par vecteur, en release à luciepc ; 36,6 ms à la tablette en debug).
- **Le recouvrement** (`probe_reuse_by_text_hash`) : **5 790 de ces morceaux (80,9 %) ont un texte que la base embarque
  déjà**, sous un autre contenu. Le partage par contenu ne les couvre pas ; B les couvre.
- **La base** : 181 811 morceaux avec un vecteur granite ; 85 656 portent l'un de 38 608 textes répétés, soit **environ
  47 000 vecteurs redondants (26 %)**.
- **Ce que B changerait pour cette vue** : environ 1 366 vecteurs écrits au lieu de 7 156, soit de l'ordre de 10 s
  d'écriture au lieu de 53. La base, migrée, perdrait un quart de ses vecteurs et de son index HNSW.

## Ce qui se mesure ensuite

Sur le même écart d'un jour qu'ED (281 fichiers, environ 7 000 vecteurs), à luciepc, release :

- les **vecteurs écrits** : aujourd'hui, un par morceau neuf ; avec B, un par texte neuf ;
- le **temps d'ouverture de la vue**, et la part de `derive_embed` ;
- le **temps d'une recherche dense** sur la vue, avant et après, pour la projection à deux sauts.
