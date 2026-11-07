# Ce que le moteur doit au modèle des vues

Une lecture, pas une proposition (demandée par c7 le 7 novembre 2026, pour c7 et everything
declarative). La vision lue est `visions/2026-11-06-15h46-un-index-par-depot-des-vues-par-worktree.md` :
des versions par contenu au grain du scope, et une vue = une table chemin → version. Ce qui
suit dit ce que le moteur et le crate savent déjà faire pour elle, ce qui manquerait, et ce que je
mesurerais d'abord. Aucun changement du moteur n'est proposé sans mesure.

Tout est lu sur master 9dc626c70 (le moteur à 97daa41ed, la lib en service 04063ee06). Les
chemins C++ sont relatifs à la racine, les chemins Rust à `extension/rag3weaver/`.

## 1. Ce que dit déjà une mesure sur git

Les dix branches distantes les plus récentes (7 novembre, après-midi : `tty-1`, `tty-1-2`,
`docs-i4`, `ordre-des-verrous`, `ordre-des-verrous-2`, `csr-listes-temoin-2`, `creux-par-l-hote`,
`docs-recherche-7-nov`, `hooks-claude-code-4`, `hooks-claude-code-4b`), au grain du fichier,
sur un master de 7 137 chemins (3 571 de code) :

| ce qu'on compte | valeur |
|---|---|
| chemins qu'une branche change contre **sa base** (merge-base) | 0 à 8 (code : 0 à 5) |
| chemins qu'elle diffère du **master du moment** | 5 à 53 |
| chemins touchés par au moins une des dix | 27 (15 de code) |
| versions distinctes d'un chemin touché, master compris | 2 pour 19 chemins, 3 pour 7, 4 pour 1 |

Deux enseignements, avant toute question au moteur :

- **Une vue se définit contre sa base, pas contre le principal du moment.** Contre le master du
  moment, la vue de `hooks-claude-code-4` (35 commits de retard) pèse 53 chemins, dont 46 sont
  le travail des autres arrivé depuis. Si la vue est « ce qui diffère du principal », elle grossit
  à chaque push de quiconque et il faut la recalculer ; si elle est « ce que la tâche a changé
  depuis sa base », elle pèse ses 7 chemins et ne bouge que par la tâche. La vision dit
  « la liste des chemins dont la version diffère du principal » : c'est à trancher, et la mesure
  penche pour la base.
- **Le partage est massif.** Dix vues ensemble ajoutent 27 versions de fichier (moins d'une
  sur deux cents chemins) ; au grain du scope, la part partagée serait plus grande encore (mesure
  M2 ci-dessous).

## 2. Ce que le moteur sait déjà faire

### L'index dense filtré : un pré-filtre par masque

- `QUERY_VECTOR_INDEX` accepte un graphe projeté. Le crate pose `PROJECT_GRAPH_CYPHER('_vf_<table>',
  'MATCH (n:T) <jointure chunk → parent> WHERE … RETURN n')`, interroge, puis le retire
  (`src/rag3db_search_backend.rs` l. 78-153 : DROP, PROJECT, QUERY, DROP, quatre requêtes par
  appel, valeurs insérées en littéraux car la projection ne prend pas de `$param`).
- Le moteur en fait un **masque roaring rempli avant le parcours** (`extension/vector/src/function/query_hnsw_index.cpp`
  l. 372-392, 423-431), que la couche basse du HNSW lit pendant la recherche : seuls les voisins
  admis sont suivis (`extension/vector/src/index/hnsw_index.cpp` l. 1525-1576, 1624-1640). Ce
  n'est pas un post-filtre sur les k rendus.
- La stratégie suit la **sélectivité** (admis / lignes de la table, `getFilteredSearchType`,
  l. 1578-1592) : sous 0,08 une recherche « à deux sauts à l'aveugle », sous 0,4 « à deux sauts
  dirigée », au-dessus un saut filtré. Les seuils, `efs` (200) et `ml` se règlent par requête ;
  le crate n'en passe aucun.
- Les lignes pas encore dans le graphe (au-delà de `numCheckpointedNodes`) sont balayées en force
  brute **sous le masque** (l. 675-720, correctif 695eee28e du 27 août).

**Pour une vue, c'est la bonne forme** : le filtre d'une vue est presque tout admettre (le
principal moins les quelques lignes que la vue remplace, plus les siennes), donc une sélectivité
proche de 1 et la stratégie la plus simple.

### Le plein texte filtré : un sous-ensemble poussé dans lucivy

- Le signal BM25 résout le filtre en offsets par une requête (`MATCH (n:Scope) … WHERE … RETURN
  OFFSET(id(n))`, `src/read_view.rs` l. 333-384, `src/dialect.rs` l. 2499-2530) et les passe à
  `search_filtered` (`src/fts_handle.rs` l. 154-171). `doc_freq` et `N` sont comptés sur le
  sous-ensemble : le score est celui d'un index qui ne contiendrait que le permis.
- Mesuré dans lucivy (`docs/26-aout-2026-20h29/09-reponse-lucivy-prefiltre-sparse.md` l. 94-118) :
  sur 40 000 documents, ×1,14 à ×1,27 contre la recherche nue ; trier les ids compte
  (540 000 ids : 6,0 ms non triés, 0,22 ms triés, `src/read_view.rs` l. 374-380).
- Le moteur, lui, n'a plus d'index plein texte : l'extension FTS de l'amont est retirée
  (1275099c2, 10 octobre).

### Un filtre de chemin existe déjà de bout en bout

`search_code` prend `scope` (préfixes) et `language`, traduits en `file_path STARTS WITH` et
appliqués au plus près de chaque index : projection pour le dense, offsets pour lucivy et
sparse (`src/work_domain.rs` l. 75-98, 430-437). La forme d'une vue, « tel chemin, telle
version », est de la même famille.

### Les empreintes et la montre

- `File.content_hash` (blake3 du texte) existe ; la montre compare la date et la taille, puis
  hache, et ne réindexe que si l'empreinte change (`src/watch.rs` l. 439-452, 566-577).
- `Scope.key` = blake3(fichier, nom, type, signature) : une identité stable quand les lignes
  bougent, **pas une empreinte de contenu** (`src/code.rs` l. 357-363). La colonne système
  `_content_hash` (sur toute entité, `src/schema.rs`) hache les champs de contenu, ceux qui
  s'embarquent : c'est déjà la clé qui dit « mêmes vecteurs », mais pas une version complète (ni
  position, ni nom) ; les champs exacts qu'elle couvre pour `Scope` sont à confirmer.
- La réindexation est **par fichier entier** (`code_sync::reingest_file`, `src/code_sync.rs`
  l. 239-345) : analyse, retrait des scopes disparus, oubli des arêtes sortantes, ingestion,
  résolution. Le court-circuit de l'inchangé (`split_unchanged`) épargne déjà les vecteurs d'un
  scope dont tous les champs sont égaux.

### La suppression d'une ligne indexée

- Une ligne de nœud effacée est un tombstone MVCC ; son entrée HNSW est retirée **en place** dans
  la même transaction, puis le graphe est réparé chez ses voisins (`deleteFromGraph`,
  `finalizeDelete`, `hnsw_index.cpp` l. 998-1036, 1183-1246, 1399-1435).
- Coût mesuré (ticket fermé `2026-10-12-supprimer-une-ligne-indexee-hnsw-coute-des-secondes`) :
  de 1,4 à 6,9 ms par ligne selon la forme ; 45 morceaux en 242 ms en service.

## 3. Ce qui manquerait, ou coûterait — sans proposition

Chaque point est une question à mesurer, pas un manque prouvé.

1. **Un filtre sur chaque recherche, y compris celles du principal.** Dès que des versions de
   tâches vivent dans les mêmes tables, la recherche du principal doit elle aussi les cacher :
   plus aucune recherche n'est nue. Or le masque est **refait à chaque appel** : la projection est
   reparsée, replanifiée, et son sous-plan parcourt la table (le parent et sa jointure aux
   morceaux) pour remplir un roaring de la taille de la table (`query_hnsw_index.cpp` l. 142-152,
   374, 428-431) ; `visited` aussi (l. 652-655). Côté lucivy, la résolution des offsets est une
   requête de la même taille. Ce coût, proportionnel à la table et non à la vue, n'est mesuré
   nulle part : c'est **la** mesure qui décide (M3). Si elle est petite devant les 200 à 750 ms
   d'un `search_code`, rien ne manque ; sinon, la question devient celle d'un masque gardé par vue
   et par génération, au plus près du HNSW — une question pour le cœur, chiffres en main.
2. **L'instantané L6.** Aujourd'hui toute recherche filtrée **sort de l'instantané**
   (`SearchSourceNodeFactory::snapshot_safe`, `src/dataflow/node_factories.rs` l. 669-694),
   parce que `PROJECT_GRAPH_CYPHER` est un `CALL` autonome classé écriture et refusé sous
   `READ ONLY`. Avec des vues, toute recherche serait filtrée, donc aucune ne serait cohérente
   avec le reste de l'appel. Le lot « projection-lecture-seule » du cœur (8e616de9f, 08eebb0ff,
   pas encore sur master) est un **préalable** de la marche 2 de la vision, pas une option.
3. **La sélectivité compte les morts.** Le dénominateur est le nombre de lignes de la table,
   supprimées comprises (`src/graph/on_disk_graph.cpp` l. 168-182), et rien ne compacte une
   table de nœuds : un décalage effacé n'est jamais réutilisé. Chaque vue fermée laisse ses
   versions orphelines en lignes mortes ; le masque, `visited` et la sélectivité grossissent avec
   l'histoire, pas avec l'index vivant. Combien de versions par jour, et quand cela pèse, est la
   mesure M5.
4. **L'effacement d'une vue qui meurt.** Au coût mesuré (≈ 5 ms par morceau en service), une vue
   de 7 fichiers et quelques centaines de morceaux s'efface en quelques secondes ; une vue
   contre le principal du moment (53 fichiers) en une dizaine de secondes ou plus. Le compte de références
   de la vision le borne aux versions que plus rien ne cite. Pas de mesure récente d'un
   effacement de milliers de lignes ; une ancienne laissait voir un `finalizeDelete`
   superlinéaire en lot (même ticket, l. 69-71).
5. **Le rappel sous un filtre de tâche seule.** Une recherche qui ne veut que les versions d'une
   tâche (« ce qui a changé ici ») a une sélectivité minuscule : stratégie à l'aveugle, amorcée
   par les **dix premiers décalages** du masque, pas les plus proches (`hnsw_index.cpp`
   l. 1594-1622), sans repli exhaustif. Sous quelques centaines de lignes, un balayage exact
   serait juste et bon marché ; le moteur ne le choisit pas de lui-même. À mesurer (M4) avant
   d'en parler.
6. **Les relations par version** : rien n'existe, et c'est la marche la plus lourde (les arêtes
   partent d'une version ; la résolution d'un symbole se fait dans la vue). Le contrôle des deux
   sens (`tools/check_rel_directions`, étape 8 de `docs/procedures/bascule-de-lib.md`) en sera le
   témoin naturel.

Un point relevé en passant, **à vérifier par recherche** : le signal sparse filtre par des
offsets résolus sur la table **parent** (`Scope`), alors que sa poignée lucivy est celle de la
table des **morceaux** (`src/dataflow/generic_search_nodes.rs` l. 1717-1723, 1781-1783 ;
`src/record_nodes.rs` l. 2232-2240). Si les deux espaces d'offsets diffèrent, le sparse filtré
rend aujourd'hui de mauvaises lignes. Je ne l'ai pas joué : ticket
`docs/tickets/2026-11-07-le-filtre-du-signal-creux-compare-des-offsets-de-parents-a-des-offsets-de-morceaux.md`.

## 4. Ce que je mesurerais d'abord, sur la base de code de Lucie

Sur une copie reflink de `code-rag3db-depot` (jamais la base en service), sous la lib épinglée :

- **M1, fait ci-dessus au grain du fichier** : la taille des vues. À refaire chaque semaine sur
  les branches vivantes, contre la base et contre le master du moment.
- **M2, les versions au grain du scope** : pour les 27 chemins, découper chaque version par
  codeparsers et compter les scopes dont l'empreinte change d'une version à l'autre ; la part
  des morceaux et des vecteurs que les vues partageraient. C'est ce qui chiffre l'économie de
  l'embarqueur.
- **M3, le coût du filtre sur chaque recherche** : `search_code` sur les huit questions du banc
  de recherche, nu puis sous un filtre de forme vue (le principal moins 50 chemins, plus 50
  lignes), par signal ; pour le dense, `RAG3DB_VECTOR_TRACE=1` sépare la projection, le
  remplissage du masque et le parcours ; pour lucivy, le temps de `resolve_filter_to_ids` à part.
  Une médiane et un p95 sur trente appels.
- **M4, le rappel sous filtre** : pour un masque de forme vue (sélectivité 0,99) et un masque de
  tâche seule (50 à 500 lignes), comparer les k rendus aux k exacts d'un balayage complet.
- **M5, ce que l'histoire coûte** : ajouter puis effacer N vues de 7 fichiers (N = 10, 50, 200)
  sur la copie, et suivre la durée de l'effacement, la taille de la table et la latence de M3. La
  question : à partir de combien de vues fermées la recherche paie-t-elle ses morts.

Chaque chiffre se dit avec la lib qui l'a produit et la base mesurée (procédure
« prouver l'artefact mesuré ») ; un écart entre deux mesures se départage sans harnais avant
d'accuser le moteur.
