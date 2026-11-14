# Le plein texte de la base du dépôt pèse cent fois ses sources

- **État** : ouvert ; **mesuré le 14 novembre** (voir « La mesure »). Le poids vient surtout de fichiers
  générés, importés ou minifiés indexés par rag3weaver. Ensuite, le lot « profil léger », à la décision de Lucie.
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

## La mesure (14 novembre, 13 h 10, sur copie)

Copie par reflink de `Lucivy_Scope` seul (le plein texte ; la base rag3db n'est pas lue), cohérente : tous
les segments nommés par chaque `meta.json` sont complets. 4 shards, 27 segments, 4,59 Go. Lue par les
métadonnées, puis par le test ignoré `diag_poids_du_texte` (branche lucivy `diag-poids-du-texte`, 7174fdd),
sur une copie de la copie.

**Le poids vient de ce qui est indexé, pas du moteur.**

- **Le texte indexé fait 928 Mo** (champ `content`), pas 37 Mo. L'index pèse **4,9 fois son texte**,
  normal pour un index avec positions (la doc annonce environ 6). Le « cent fois » compare l'index aux
  seules sources suivies par git.
- **10 % des documents portent 91,6 % du texte.** Médiane 275 o, p90 20 Ko, p99 133 Ko, maximum 1,9 Mo.
  Les plus gros sont générés ou importés : la table Unicode d'`utf8proc` (1,9 Mo), un fichier
  « Copyright 2013 Google » de 470 Ko (probablement brotli), des **bundles JavaScript minifiés de 220 Ko
  en plusieurs versions**. Rien de tout ça ne sert à une recherche de code.
- **21 % des contenus sont des doublons exacts** : 71 024 contenus distincts pour 89 886 documents.
- **Documents supprimés : 12 %** (12 174 sur 102 068). La piste 1 n'est pas la cause principale.
- **Composition de l'index** : `sfxpost` 21 %, `word_pos_map` 19,4 %, `word_sfxpost` 17,7 %, `posmap`
  15,8 %, texte stocké (`.store`, lz4) 10,4 %, dictionnaire partagé 11,4 %, `sibling_v3` 1,6 %. Les
  fichiers de positions (`word_pos_map`, `posmap`, `sibling_v3`) font **environ 37 %**.
- Les gros segments sont au plafond de fusion (10 000 documents), à 300-400 Mo chacun.
- **Écart non expliqué** : 89 894 documents vivants, tous de `_node_id` distincts, alors que l'hôte annonce
  68 585 documents `Scope` pour le backend `rag3db` (branche master). Ce `.fts` n'est probablement pas
  celui du backend ouvert. À vérifier côté rag3weaver avant de reconstruire quoi que ce soit.

### Ce que ça change aux leviers, dans l'ordre du gain

1. **Ne pas indexer les fichiers générés, importés ou minifiés** (côté rag3weaver) : sources hors git, les
   dossiers de dépendances importées, les fichiers minifiés, une taille maximale par document. C'est le
   plus gros levier : le texte indexé pourrait revenir près des sources suivies.
2. **`positions: false`** (décidé par Lucie) : environ 37 % de l'index, plus les positions dans les
   postings.
3. **Les doublons exacts** (21 % des contenus) : à comprendre côté rag3weaver (des scopes imbriqués qui
   répètent le texte, ou des fichiers copiés).
4. **La purge des supprimés** (12 %) : la fusion selon la part de supprimés, plus tard.
5. **La compression du texte stocké** (10 %) : la dernière, un gain de quelques pourcents.

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

## Le départage des comptes de Scope (hotfixs, 14 novembre 2026)

À la question de c7 : lucivy compte 89 894 documents vivants dans `Lucivy_Scope`, tous de
`_node_id` distincts, et `daemon_status` annonce 68 585 Scope. Pour comparer les deux sur un même
état, j'ai pris **une seule copie fidèle**, base et `.fts` ensemble : l'instantané
`avant-835d5298f/code-rag3db-depot`, pris par G hôte arrêté le 13 novembre à 20 h 07, `fuser` vide.
J'ai lu le graphe en lecture seule par la lib 835d5298f, et le `.fts` sur une copie neuve, par le test
lucivy `diag_node_ids_vivants` (branche `diag-mots-perdus-tous-champs`).

| mesure, sur la même copie | valeur |
|---|---:|
| Scope du graphe | 79 662 (plus grand offset 93 718) |
| documents de `Lucivy_Scope`, `max_doc` | 100 350 |
| documents vivants (`num_docs`) | 88 471, tous de `_node_id` distincts |
| documents supprimés encore dans leurs segments | 11 879 |
| Scope qui ont leur document | 79 662 : tous |
| **documents vivants sans Scope (orphelins)** | **8 809** |
| `_node_id` au-delà du plus grand offset | 0 |

**Lecture : des documents orphelins dans le `.fts`.** Ce sont des Scope effacés du graphe mais pas de
l'index, et ils tombent tous dans les places laissées vides par les Scope effacés. C'est la famille
des réindexations : la piste 1 de ce ticket pour les supprimés, et une forme vivante en plus.

Les 89 894 de lucivy et les 68 585 de `daemon_status` viennent d'autres instants que cette copie. Ils
ne se comparent ni entre eux ni à elle. Pour une jointure à l'instant présent, il faut un nouvel
instantané pris hôte arrêté.

**Ce que je n'ai pas fait** : savoir si les recherches rendent ces orphelins, ce qui demande la
jointure par l'hôte, ni d'où ils viennent, c'est-à-dire quel chemin d'effacement oublie le `.fts`.
Les relevés sont au portable, dans `~/.cache/rag3db-locks/sonde-coherence/` (`node-ids.txt`,
`scope-offsets.txt`, `node-ids.log`).

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

## Autres leviers discutés avec Lucie le 13 au soir (à chiffrer après la mesure)

- **Fusion selon la part de documents supprimés.** Le moteur a `set_del_docs_ratio_before_merge`
  (`src/indexer/log_merge_policy.rs:81`), mais à 1,0 par défaut, donc éteint, et `lucivy-core` ne le
  règle pas (`handle.rs:166-169`, seulement la taille maximale des fusions). Un segment qui a atteint la
  taille maximale n'est plus repris : ses versions supprimées n'en sortent jamais. Si la piste 1 domine,
  c'est le premier lot : exposer ce seuil dans `SchemaConfig` (par exemple `merge_deleted_ratio: 0.25`),
  avec un témoin rouge d'abord qu'un segment à la taille maximale soit repris passé le seuil.
- **La compression du texte stocké, en série d'essais** : la taille des blocs (`docstore_blocksize`,
  qui existe déjà dans les réglages de l'index, 16 384 par défaut, mais pas dans `SchemaConfig`), les
  niveaux de zstd (1, 3, 9) contre lz4, le cache de blocs décompressés (il en existe un petit). Mesurer
  la taille, la latence p50/p95/p99 et les octets décompressés par requête, sur les requêtes `contains`
  et `regex`. Utile seulement si le texte stocké pèse : 6 % dans l'index rouge avec positions, plus sans.
- **zstd avec un dictionnaire entraîné, piloté par rag3weaver par dépôt.** C'est la dernière étape, si
  les essais simples ne suffisent pas. lucivy fournit le mécanisme : un dictionnaire reçu, avec un
  identifiant, utilisé par les nouveaux segments et noté dans chacun, migré aux fusions, oublié quand plus
  aucun segment ne le cite. rag3weaver fournit la politique : entraîner sur les fichiers du dépôt, par
  langage, et réentraîner quand il a beaucoup changé. Un dictionnaire n'est jamais modifié, seulement
  ajouté, donc aucune réindexation n'est nécessaire à chaque entraînement. C'est un changement de format :
  une 4.4, avec une réindexation, une seule fois.

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

- **FAUX tant que ce n'est pas corrigé** : je l'avais lu ainsi dans le code (`bm25_global.rs:58-64`,
  `with_subset_docs` l. 75, posé par `search_dag.rs:451`). Le témoin du soir montre l'inverse : sous
  `allowed_ids`, le score mélange N de l'index entier et df de la vue, pour tous les types de requête.
  Ticket : `2026-11-13-une-recherche-filtree-ne-classe-pas-comme-sa-vue.md`. La page 30 ne peut pas
  dire « classé dans la vue » avant le correctif.
- **Témoin fait, rouge (voir le ticket ci-dessus)**, plus large que prévu. Le soupçon de départ :
  `doc_freq(term)`
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
