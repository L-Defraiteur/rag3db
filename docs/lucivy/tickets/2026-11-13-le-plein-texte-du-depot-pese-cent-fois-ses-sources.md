# Le plein texte de la base du dépôt pèse cent fois ses sources

- **État** : ouvert (13 novembre 2026, au soir). La mesure se fait le 14, dans un creux, sur une
  copie. Ensuite, le lot « profil léger », à la décision de Lucie.
- **Gravité** : place disque. Lucie a fixé un budget de rétention de 15 Go par dépôt, et le chiffre
  dépend de ce ticket.
- **Atteignable en service** : oui (`~/.local/share/rag3weaver/code-rag3db-depot.rag3db.fts`).
- **Touche rag3weaver** : oui (manifeste du backend de code, `fts_positions`) ; lucivy peut-être
  (compression du texte stocké).
- **Famille** : produit

## Ce que c'est

La base du dépôt pèse 4,1 Go. Le plein texte en fait 4,0 Go, presque tout dans la table
`Lucivy_Scope`. En face : 37 Mo de sources suivies par git (8 438 fichiers) et un graphe de 2,9 Go.
D'après la documentation de lucivy, un index avec positions pèse environ six fois son texte, et
environ 2,6 fois sans. Cent fois, c'est d'un autre ordre : les réglages n'expliquent qu'un facteur 2 à
3, le reste vient d'ailleurs.

## Les pistes à départager (la mesure, pas une supposition)

1. **Des documents supprimés qui restent dans leurs segments.** rag3weaver prend pour `_node_id`
   l'offset de la ligne rag3db, et réindexe une ligne par `delete_by_node_id` puis un nouvel ajout
   (`extension/rag3weaver/src/fts_handle.rs`, `upsert_document` / `reindex_document`). Le document
   supprimé occupe sa place jusqu'à ce qu'une fusion le purge.
2. **Des doublons vivants d'un même scope**, si le graphe crée une ligne par version de scope sans
   supprimer les anciennes.
3. **Des segments jamais fusionnés, d'avant la 4.3.5.** La base est antérieure au correctif. Si c'est
   la cause, la réponse est une reconstruction du `.fts` à la décision de Lucie, pas un lot de code.
   `check_dictionary` (PR lucivy #29) dira aussi si elle a perdu des mots. La sonde de hotfixs a déjà
   compté 0 sur les copies des trois bases de Lucie.
4. **Le texte stocké lui-même et les positions** : la part qui reste une fois le reste purgé.

## La mesure (le 14, sur copie)

- Copier ensemble la base, le wal, les extensions et le `.fts`, l'hôte à l'arrêt ou la base fermée.
  Ne jamais toucher l'original.
- Par segment de chaque shard de `Lucivy_Scope` : la taille, `max_doc` contre `num_docs` (la part
  supprimée), la taille par fichier (texte stocké `.store`, postings, fichiers de positions,
  dictionnaire).
- `num_docs` contre le nombre de lignes `Scope` du graphe (les doublons vivants).
- La taille du texte indexé, pour le rapport `.fts` / texte.

## Les leviers, une fois la mesure faite

- **Décidé par Lucie** (13 novembre au soir) : **dictionnaire partagé + `positions: false`** pour le
  backend de code. Le dictionnaire partagé est déjà actif. Aujourd'hui, seul
  `experiments/mtga/backend/backend.json` met `fts_positions: false` ; le backend de code garde `true`.
  `derived_in_ram` ne s'ajoute pas : lucivy refuse la combinaison
  (`lucivy_core/src/query.rs:149-150`, « positions: false leaves nothing for derived_in_ram to
  rebuild »), et `positions: false` supprime déjà les fichiers qu'il aurait reconstruits en mémoire.
  Le réglage se fixe à la création : la base est à reconstruire, sur copie d'abord.
- **À chiffrer, sans rien changer avant** : la compression du texte stocké. Elle est en lz4 par
  défaut. Le moteur sait faire du zstd (`src/store/compressors.rs`, feature `zstd-compression`, pas
  activée par défaut), mais `SchemaConfig` ne l'expose pas. Avec `positions: false`, chaque résultat
  est vérifié en relisant le texte stocké : la décompression est donc sur le chemin de chaque requête,
  et zstd y est plus lent que lz4.
- **Une purge des supprimés**, si la piste 1 domine : une fusion qui réécrit les segments, ou une
  reconstruction.

## À lire dans le code avant le lot

- **Ce que `positions: false` retire aux requêtes** : phrases, proximité, surlignage, ce que les
  pré-recherches v3 contains et regex demandent. La doc de `SchemaConfig` dit « mêmes réponses ; les
  spans et les correspondances à travers les mots coûtent une lecture des candidats ». C'est à vérifier
  type de requête par type de requête.

## La mesure avant/après du lot

- La taille du `.fts`, et le rapport `.fts` / texte indexé.
- Le temps du jeu fixe de requêtes. Lucie accepte « un poil plus lent », pas davantage.
- **Les résultats du jeu fixe** : mêmes documents dans le même ordre, ou la différence listée. Un
  rapport qui baisse en perdant des réponses n'est pas un gain.
- Rouge d'abord : un témoin qui borne le rapport `.fts` / texte.

## Notés en passant (lucivy, vues filtrées, page 30 d'arbre principal)

- **Une recherche filtrée par `allowed_ids` est classée dans la vue** : N vaut la taille de la vue, le
  df des contains et regex est compté par la pré-recherche, qui ne voit que la vue, et la longueur
  moyenne reste celle du corpus, exprès (`lucivy_core/src/bm25_global.rs:58-64`, `with_subset_docs`
  l. 75, posé par `lucivy_core/src/search_dag.rs:451`).
- **Défaut probable, à prouver par un témoin rouge d'abord** : `doc_freq(term)`
  (`bm25_global.rs:109-115`) somme `searcher.doc_freq` sur tout l'index. Une requête par terme qui ne
  passe pas par une pré-recherche aurait donc N de la vue et df du dépôt. Le témoin : une requête par
  terme sous `allowed_ids` sur dix documents doit avoir le même score que sur un index réduit à ces
  dix documents. Après `check_dictionary` et la mesure.
- **Amélioration à chiffrer, pas à faire** : le masque est reconstruit à chaque requête, en parcourant
  le champ `_node_id` de chaque segment (`sharded_handle.rs:703-735`). On pourrait le garder en cache
  par segment et par génération.
- **`indexed: false` est ignoré pour un champ texte** (`lucivy_core/src/handle.rs`, `build_schema`,
  branche `"text"`) : tout champ texte est indexé. L'honorer est le choix de Lucie, à présenter avec les
  chiffres de cette mesure. D'ici là : un avertissement et une ligne de doc.
