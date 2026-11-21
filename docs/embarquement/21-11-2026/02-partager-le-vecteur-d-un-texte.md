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
- **Le retrait d'une version, et les « usagers » d'un texte** (question de Lucie, 21 novembre : « si mon worktree et un
  autre disent le même scope et que je retire le mien, il faut que l'autre reste ») :
  - un texte a pour usagers les morceaux qui pointent vers lui (ses arêtes `HAS_TEXT`). Le retrait d'un morceau retire
    son arête ; **un texte ne part que quand plus aucun morceau ne pointe vers lui**, et son vecteur part avec lui ;
  - **la course** entre un retrait et une écriture qui reprendrait le même texte est tenue par **le seul écrivain de la
    base** : le retrait comme l'écriture passent sous le verrou du catalogue de l'hôte, et ne se croisent pas ;
  - **à une condition** : le retrait de l'orphelin se fait dans ce même chemin d'écriture, par un seul énoncé qui teste
    l'absence d'arête et supprime (`MATCH (t:<Entité>_Text) WHERE NOT EXISTS { MATCH (t)<-[:HAS_TEXT]-() } … DELETE t`,
    borné aux textes que le retrait a touchés). Jamais en deux temps (compter, puis supprimer), ni par un balayage hors
    du verrou ;
  - **les lecteurs** (une recherche pendant le retrait) voient un instantané : un texte encore lu n'est pas retiré sous
    eux.
  - **La course exacte** (Lucie, 21 novembre) : un écrivain trouve le texte et va lui ajouter un morceau ; entre-temps, le
    retrait de son dernier propriétaire voit « plus de référence » et le supprime ; l'ajout arrive ensuite sur un texte
    disparu. Le verrou de l'hôte sérialise les transactions, mais **la recherche du texte existant se fait dans la même
    transaction que l'ajout de l'arête**, jamais avant, hors du verrou. Sinon, la décision « je reprends ce vecteur »
    repose sur un état périmé. D'où deux règles :
    1. **L'ajout, en un seul énoncé**, dans la transaction qui écrit le morceau : `MERGE` du texte par empreinte, puis
       l'arête. Un texte absent est recréé ; son vecteur va alors à la file d'embarquement (calculé, ou repris par
       empreinte) et n'est jamais laissé vide : le texte neuf compte comme un vecteur dû, comme un morceau neuf
       aujourd'hui.
    2. **Le retrait de l'orphelin, en un seul énoncé**, dans la transaction du retrait, comme dit plus haut.
  - **Le moteur** : il accepte plusieurs écrivains par défaut depuis le 11 octobre, mais l'hôte n'ouvre qu'**une
    connexion écrivaine par base**, et un second processus est refusé par le verrou de fichier (`F_WRLCK`). Ce qui
    casserait avec un second écrivain dans le même processus : deux transactions concurrentes, l'une qui ajoute une
    arête vers un texte, l'autre qui le voit sans arête dans son instantané et le supprime. Sans un conflit détecté au
    commit (l'arête vers un nœud supprimé), l'arête pendrait vers rien. Le lot ne s'appuie donc que sur l'écrivain
    unique, **par une garde, pas un commentaire** (c7) : l'écriture des textes partagés vérifie qu'elle passe par la
    connexion écrivaine unique du backend, et refuse en le disant si un second écrivain apparaît dans le même processus.
    Elle ne tente jamais sa chance. Celui qui ajoutera un second écrivain tombera sur ce refus, pas sur une arête qui
    pend.
  - **Aujourd'hui** (la reprise par empreinte, master 13d0e50333), la question ne se pose pas : le vecteur repris est
    **copié** dans la ligne du nouveau morceau (`record_nodes.rs:2988` le clone, `:3006` l'écrit par `write_vectors`).
    Retirer l'un ne touche pas l'autre.
- **La vie d'une couche de worktree** (Lucie, 21 novembre : « on ne gardera pas h24 tous les worktrees passés, on les
  virera une fois terminés, même en base ») :
  - quand une vue est fermée ou son worktree retiré (`close_view`, retrait après fusion), sa couche de travail part de la
    base : ses morceaux partent, puis les textes devenus orphelins, **dans la même écriture** ;
  - **un worktree fusionné ne perd presque rien** : son contenu vit désormais dans la couche de master, qui garde les
    textes. Seuls partent les textes propres à une branche abandonnée, et un texte qui revient plus tard se
    ré-embarque, ce qui reste rare ;
  - **ce qu'une branche d'un jour possède en propre**, mesuré sur la copie : sur les 7 156 morceaux neufs de l'écart d'un
    jour, **1 366 (19 %) portent un texte que la base n'a pas**. C'est ce qui partirait, vecteurs compris, si la branche
    était abandonnée ; le reste est à la base ;
  - **un délai de grâce sur les orphelins** (garder N jours un texte sans usager) sera une option de configuration si les
    chiffres le demandent un jour, pas maintenant ;
  - **ce qui manque pour s'y brancher** : ce que `close_view` retire aujourd'hui en base (la couche, ses lignes, rien),
    demandé à arbre.
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
3. **La séquence de la course** : on trouve le texte, on retire son dernier propriétaire, puis on attache le nouveau
   morceau. À la fin, le texte existe, l'arête aussi, le vecteur n'est pas vide (ou il est dû, et la file le
   calcule), et aucune arête ne pend vers rien.
4. **Un second écrivain est refusé** : l'écriture des textes partagés par une autre connexion que l'écrivaine du
   backend rend une erreur qui le dit, et n'écrit rien.
5. **Deux worktrees, le même scope** : le retrait de l'un laisse le vecteur de l'autre, et la recherche dans la vue de
   l'autre le trouve ; puis le retrait du second retire le texte orphelin et son vecteur (le compte des textes baisse
   de un).

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

## La recherche dense de B, mesurée (21 novembre, luciepc, release)

Sondes `probe_shared_text_search` et `probe_shared_text_search_filtered` (branche de sonde `sonde-partage-texte`, jamais
sur master), sur un reflink de la copie en règles 13, lib 424ce93e56.

**La construction de B sur la vraie base** : 134 763 textes pour 181 811 morceaux (26 % de vecteurs en moins), en 10 s ;
181 811 arêtes en 4 s ; l'index HNSW des textes (cosinus, `mu` 30, `ml` 60, `efc` 200) en 70 s.

**Sans filtre** (100 requêtes, des vecteurs de morceaux, rappel contre les 10 textes distincts exacts, par force brute) :

| | Médiane | p95 | Rappel@10 | Textes distincts sur 10 |
|---|---|---|---|---|
| aujourd'hui (index des morceaux) | 206,9 ms | 431,5 ms | 0,748 | 7,5 |
| **B (index des textes)** | **58,7 ms** | **98,0 ms** | **1,000** | 10 |

**Avec un filtre** sur l'occurrence (30 requêtes, prises dans le filtre, chemin `extension/rag3weaver/src/dataflow/`,
5 812 versions admises), projeté comme `vector_search_filtered` le fait : aujourd'hui morceau → version ← occurrence, B
texte ← morceau → version ← occurrence ; puis les versions admises passées en liste, comme le masque des occurrences :

| | Médiane | p95 | Rappel@10 | Textes distincts sur 10 |
|---|---|---|---|---|
| aujourd'hui, filtre par chemin | 365,2 ms | 479,2 ms | 0,757 | 7,6 |
| **B, filtre par chemin** | **136,1 ms** | **172,3 ms** | **0,870** | 10 |
| aujourd'hui, liste des versions | 458,3 ms | 496,4 ms | 0,757 | 7,6 |
| **B, liste des versions** | **249,4 ms** | **281,3 ms** | **0,870** | 10 |

- **Le troisième saut ne coûte rien de visible** : l'index plus petit d'un quart rend B plus rapide de 2 à 3 fois, avec
  ou sans filtre.
- **Le rappel est meilleur** : aujourd'hui, les doublons d'un même texte prennent des places (7,5 textes distincts sur
  10). Sous filtre, B reste à 0,870 : c'est l'approximation de HNSW dans un graphe projeté, la même qu'aujourd'hui.
- **La lecture des versions admises** coûte 51 ms. Le masque passé en liste coûte environ 110 ms de plus que le filtre
  par chemin, pour les deux formes.
- **Une variante sans le troisième saut** n'est pas utile : le saut ne coûte rien. Celle qui le supprimerait (copier la
  version admise sur le texte) défairait le partage.

## Ce qui se mesure ensuite

Sur le même écart d'un jour qu'ED (281 fichiers, environ 7 000 vecteurs), à luciepc, release :

- les **vecteurs écrits** : aujourd'hui, un par morceau neuf ; avec B, un par texte neuf ;
- le **temps d'ouverture de la vue**, et la part de `derive_embed` ;
- le **temps d'une recherche dense** sur la vue, avant et après, pour la projection à deux sauts.
