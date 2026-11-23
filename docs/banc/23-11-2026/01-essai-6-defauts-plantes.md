# Essai 6 : cinq défauts plantés, trouvés par l'agent du terminal avec et sans index

23 novembre 2026, nuit et matin. Idée de Lucie : mettre la recherche de code devant de vrais défauts. Un défaut
d'une ligne est planté dans le crate. L'agent reçoit la phrase vague d'un utilisateur, qui dit le symptôme et
jamais le lieu, et doit trouver la ligne puis la corriger.

## Les cinq défauts

Chacun est sur sa branche partie de master, `banc/essai6-a` à `-e` sur origin (jamais vers master), au nom et au
message neutres (« essai 6, variante x »). **Chacun est prouvé** : son test couvrant passe sur master
(`a47ab7f5bb`) et rougit sur sa branche, à luciepc, lib `2c4bddcd3c`, features de service.

| cas | genre | lieu planté | ligne fausse → juste | test couvrant | distance du symptôme |
|---|---|---|---|---|---|
| a | borne inversée | `src/search.rs:2192`, `within_edits` | `< max` → `<= max` | `backend::tests::a_name_with_a_typo` (4 rouges) | loin : outil → `ExactNameNode` / `FuseResultsNode` → `typo_of` → `within_edits` |
| b | condition retournée dans la fusion | `src/dataflow/generic_search_nodes.rs:2538`, tri des dominants de `FuseResultsNode::execute` | `(Some(_), None) => Greater` → `Less` | `…::tests::a_dominant_label_goes_first` | proche |
| c | champ oublié à la sérialisation | `src/dataflow/resultat.rs:68`, `UnifiedResult::best_signal` | `skip_serializing` → `skip_serializing_if = "Option::is_none"` | `…::le_morceau_d_une_ligne_suit_le_rang_pas_l_echelle_du_score` | moyenne |
| d | clé de cache trop large | `src/suivies.rs:706`, empreinte de `Resolveur::resoudre` | `taches` absent du tuple haché → présent | `suivies::tests::sans_mouvement_de_refs_la_resolution_vient_du_cache` | loin : état de l'index → `rapport_des_suivies` → `contexte` → `resoudre` |
| e | erreur avalée | `src/catalog.rs:9236`, `commit_lot_after_native` | `Ok::<(), CatalogError>(self.flush_blob_store("lot").unwrap_or_default())` → `self.flush_blob_store("lot")` | `catalog::tests::committed_rows_with_failed_fts_never_announce_readiness_or_reuse_handle` | loin : `ReindexFileNode` → `reindexer_en_lot` → `commit_lot` → `commit_lot_after_native` |

La ligne de e proposée au départ (`Ok(….unwrap_or_default())`) ne compilait pas (type indéterminé) : corrigée par un
second commit sur la branche, puis prouvée rouge.

### Les phrases

- a : « Quand je cherche un nom de fonction en me trompant d'une lettre, ça ne la trouve plus alors qu'avant si. »
- b : « Quand je tape exactement le nom d'un truc, la recherche me met d'abord des résultats approchants et le bon arrive plus bas. »
- c : « Dans la vue de recherche, je ne vois plus quel signal a trouvé chaque ligne : c'est vide partout. »
- d : « Quand je crée une nouvelle tâche d'agent dans l'atelier, elle n'apparaît pas dans les couches de l'index tant que rien ne bouge ailleurs. »
- e : « Après avoir sauvé un fichier, la recherche plein texte me rend parfois l'ancien contenu, surtout après un redémarrage, et il n'y a d'erreur nulle part. »

**La phrase c est inexacte** : la vue de recherche de l'IDE (`ui/ide/src/views/Search.tsx:160`) affiche `signal`,
pas `bestSignal`. Le défaut vide `bestSignal` dans la réponse JSON de `search_code` et dans le journal de recherche
de l'hôte (`daemon_host.rs:2525`). Gardée telle quelle pour que tous les agents jouent la même phrase ; elle a
coûté des appels à tous.

## Les formes jouées

1. **Sans historique** : le crate seul (`extension/rag3weaver`, `codeparsers` compris), défaut compris, copié dans un
   dossier neuf à un seul commit. Ni diff ni log à lire. L'hôte l'ouvre comme un dépôt neuf et l'indexe (2 109
   fichiers, environ 57 000 morceaux, granite-278m par luciepc). C'est la forme qui mesure la recherche.
2. **Avec historique** : le même crate à deux commits, `master` (« état initial », le crate au master d'où part la
   branche) et la branche `banc/essai6-x` (le défaut). `git diff master` rend une ligne. Elle mesure ce qu'un agent
   tire de git.
3. **Le témoin sans index**, sur la forme sans historique : un agent de code extérieur au produit, sur un autre
   modèle, n'ayant que grep et la lecture (ni index, ni outil MCP, ni git, ni copie de master, ni compilation).
   L'hôte ne sait pas retirer les outils de recherche à son propre agent (le socle de l'agent de code se greffe à
   tout manifeste `kind: "code"`, quel que soit `tools`) : **le témoin n'est donc pas à modèle égal**, et l'écart
   mesure l'index et le modèle ensemble.

Index vu fini avant chaque tour (« indexation terminée » en dernière ligne du journal). Un premier passage de d sur
un index échoué (« Too many open files », puis une session de Scope restée ouverte 2 h 36) est classé **non jugé**
et rejoué sur une copie neuve.

## Résultats

| cas | avec index, sans historique | avec index, avec historique | témoin sans index |
|---|---|---|---|
| a | **exact**, corrigé (21 appels, 116 s) | **exact**, désigné par git (11, 63 s) | **exact**, corrigé (5, 18 s) |
| b | voisin (19, 53 s) | voisin (18, 74 s) | **exact**, corrigé (7, 31 s) |
| c | **exact**, corrigé (92, 259 s) | **exact**, corrigé, désigné par git (93, 651 s) | **exact**, corrigé (32, 154 s) |
| d | **exact**, corrigé (40, 115 s) | **exact**, sans édition (26, 95 s) | **exact**, corrigé (7, 30 s) |
| e | voisin (40, 152 s) | raté (25, 128 s) | raté, mais un vrai défaut trouvé sur master (25, 111 s) |
| **total** | 3 exacts, 2 voisins | 3 exacts, 1 voisin, 1 raté | 4 exacts, 1 raté |

Correctifs atterris : identiques à master au bit près pour a et c ; d place `taches` après `worktree_du_backend`
(sans effet sur le sens : son fichier joué à luciepc, `suivies::tests` 15 sur 15 verts, le test couvrant compris).

### Les outils, avec index, sans historique

| cas | recherche par sens | grep | lecture | usages | impact | git | cargo (sans bâti possible) |
|---|---|---|---|---|---|---|---|
| a | 1 | 1 | 8 | 0 | 0 | 6 | 3 |
| b | 1 | 7 | 8 | 0 | 0 | 0 | 0 |
| c | 11 | 20 | ~40 | 0 | 0 | 8 | 9 |
| d | 12 | 5 | 18 | 0 | 0 | 2 | 3 |
| e | 1 | 21 | 17 | 0 | 0 | 0 | 0 |

**`usages` et `impact` ne sont jamais appelés**, même pour les trois défauts à trois appels du symptôme.

## La cause côté recherche, cas par cas

Relevée dans les sessions gardées de l'hôte : chaque appel `search_code` de l'agent, et si le lieu planté sortait
dans sa réponse.

| cas | requêtes de l'agent | le lieu dans les résultats ? | lecture |
|---|---|---|---|
| a | « fuzzy search trigram levenshtein distance » ; avec historique « fuzzy typo search distance lev_distance trigram » | **non** : `search.rs:2147` (voisin de 40 lignes), pas `within_edits` | trouvé ensuite par grep `within_edits\|typo_of` |
| b | « search ranking score exact match » ; « search score rank exact match bm25 boost » ; « BM25Mode exact boost » | **non** : le tri des dominants n'est jamais sorti ; `FuseResultsNode` une fois, par son nom | l'agent a lu la région (2450 à 2560, la ligne 2538 comprise) par `read_file` **et ne l'a pas reconnue** : raté de lecture, sur une région que la recherche n'avait pas donnée |
| c | dix requêtes (« vue recherche signal résultat », « format search result to json render »…) | **une fois sur onze**, et seulement avec le mot littéral (« "8 nov" bestSignal chunk.signal ») | trouvé par grep `skip_serializing.*best_signal` (appel 27) |
| d | « créée nouvelle tâche agent couche index worktree layer » puis neuf requêtes de noms | **non** pour le symptôme ; **oui** pour « fn resoudre resolveur » (appel 21), une requête de nom que l'agent a formée en lisant `contexte` | la recherche a servi de navigation par nom, pas de recherche par sens |
| e | « save file index plein texte commit sync flush » ; avec historique quatre requêtes (« plein texte index save flush commit restart »…) | **non** : `flush_blob_store` (10150) et l'ouverture du plein texte (1775, 1509) sortent, **l'appel qui avale l'erreur (9236) jamais** | l'agent tient la conséquence (marque de génération non posée, réversion silencieuse à l'ouverture), pas la ligne |

**Aucune requête formée à partir du symptôme n'a sorti la fonction plantée.** Les trois succès de `search_code`
sont des requêtes de noms (d) ou un mot littéral (c). Ce sont des cas pour la recherche : ils entrent au jeu de
questions comme « symptôme → lieu », avec les requêtes de l'agent telles quelles.

## Ce que dit l'historique

- `git diff master` n'est **jamais** l'appel qui trouve (en c, il arrive à l'appel 58, après le diagnostic).
- git désigne le lieu deux fois sur cinq, toujours par le commit du défaut : `git show` en a (appel 4),
  `git log -S best_signal -p` en c (appel 53).
- Sur b, d et e, l'agent ne regarde pas l'historique du tout.
- Avec historique, l'agent corrige moins (1 cas sur 5 contre 3) : il s'arrête sur l'identification. Même modèle,
  même invite ; pas d'explication sûre.
- Sans historique, l'agent appelle tout de même git (a : 6 fois, c : 8 fois) sur un dépôt à un commit, sans rien
  en tirer.

## Ce que ça dit

1. **Sur ces cinq défauts, l'index ne fait pas encore gagner.** Le témoin à grep et lecture trouve 4 sur 5, avec 3
   à 13 fois moins d'appels. Deux biais : le modèle (le témoin n'est pas le modèle du terminal) ; et **mes défauts
   sont faciles pour un grep** (en b, c et d, un commentaire ou les champs voisins contredisent la ligne fausse :
   « Les dominants devant » au-dessus d'un `Greater`, « des tâches » dans la doc d'une empreinte qui ne les hache pas,
   tous les autres champs en `skip_serializing_if`).
2. **La recherche par sens ne relie pas un symptôme à sa cause.** Elle rend des voisins de vocabulaire (le fichier
   de la fonction, une fonction sœur), jamais la ligne. Ce qui a trouvé : grep sur un nom lu, lecture, git.
3. **Le seul cas où l'index devance le témoin est e**, le plus profond : l'agent tient le bon mécanisme, le témoin
   part sur une autre cause.
4. **L'agent perd beaucoup d'appels après avoir trouvé** : c en 92 appels pour un lieu désigné à l'appel 27, à
   vérifier sans pouvoir compiler (9 `cargo test` ou `npm test`) et à interroger git sur un seul commit.
5. **b est un raté de lecture** : la région lue, la ligne sous les yeux, quatre causes générales et des
   contournements à la place.

## Une trouvaille sur master, confirmée

Au cas e, le témoin n'a pas vu le défaut planté mais a trouvé un trou qui existe sur master, relu sur
`origin/master` : `src/dataflow/record_nodes.rs:4981`, `reindex_fts_rows`. Hors transaction par lot (sans tampon),
une ligne absente du `NodeIdCache` n'est pas réindexée en plein texte : `continue`, sans erreur ni journal. Le
cache naît vide à l'ouverture, donc une ligne écrite avant le dernier démarrage puis mise à jour garde son ancien
document. Le chemin d'effacement du même fichier a le repli par la base (`select_node_ids`, commentaire ligne
4093) ; la mise à jour ne l'a pas.

**Trouvé par l'essai, confirmé depuis** par un test rouge, avec un chemin produit : le retour arrière d'une
migration (`UpdateRecordNode::undo`, hors lot, après un redémarrage) laisse l'ancien document plein texte. Le
correctif est le lot `cx-memoire-fts-unbuffered`. Un témoin parti dans une autre direction que le défaut planté
a donc trouvé un vrai défaut : c'est un résultat de l'essai en soi.

## Pour un prochain jeu

- **Défauts sans commentaire voisin qui les trahit**, et dont le symptôme ne partage aucun mot avec le code.
- **Témoin à modèle égal** : un réglage déclaré des outils d'un équipier, pour que l'hôte lance son propre agent sans
  les outils de recherche (demandé comme petit lot).
- **Lancer un agent depuis une branche donnée** : aujourd'hui une tâche part toujours du HEAD du dépôt
  (`ouvrir_une_tache_depuis(…, depuis: None)`). Graine produit.
- **Un tour de « corrige » mesuré**, quand la mémoire entre deux tours sera posée.
- Mesurer `usages` et `impact` : pourquoi l'agent ne les appelle jamais sur une chaîne d'appels.
