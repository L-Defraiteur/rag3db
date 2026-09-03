# Rapport de session — 3 septembre 2026

Dix-huit commits, trois sessions en parallèle. Ce document dit ce qui a été
fait, puis **ce qui reste à réparer et dans quel ordre** — c'est sa dernière
section qui compte le plus.

## 1. Ce qui a été livré

### PostgreSQL est prouvé — huit tests, et il n'en avait aucun

`PostgresDialect` faisait 944 lignes, compilait, et n'avait **jamais parlé à une
base**. `tests/e2e_postgres.rs` couvre maintenant : le schéma, l'écriture, le
vecteur, le plein texte, la fusion hybride, les relations, les cellules
multi-locataires, et les accents.

Ce que la vraie base a trouvé — **tout du code qui compilait et n'avait jamais
tourné** :

| défaut | conséquence |
|---|---|
| l'hôte formaté avec le `Debug` d'une énumération | aucune connexion n'était possible ; le dialecte n'avait donc jamais pu être faux |
| `Handle::current()` demandé à l'appel | « no reactor running » depuis les fils de lucivy, verrous empoisonnés |
| cinq organes parlant Cypher en dur | checkpoints, blobs, nœuds de cellule, comptage, et neuf appels prenant les enveloppes `generate_*_ddl` qui se rabattent en silence sur `Rag3dbDialect` |
| treize requêtes en lot en `unnest($items) AS v(colonnes)` | forme que `unnest` n'a pas ; **tout le chemin d'écriture était mort-né** |
| `cypher_to_pg_param` filtrant les `Map` | une `List<Map>` — les lignes d'un lot — perdait chaque map en silence |
| aucun index hors clés primaires | `_row_id`, `to_uuid`, `_parent_uuid`, le préfixe des blobs : le chemin le plus chaud balayait la table entière |
| le plein texte natif sans cloisonnement | **fuite entre cellules** : deux locataires, mêmes descriptions, même score |
| `vector_search_filtered` avec un alias inexistant et ses paramètres non transmis | le chemin vectoriel **filtré** n'avait jamais tourné |

### Le plein texte servi par la base

lucivy coûte trop cher en espace disque pour de la production : sur postgres,
elle demandait un **second corpus** dans le magasin de blobs. D'où
`MoteurTexte::{Auto, Lucivy, Natif}` — une **option**, pas un remplacement.

Deux étages : la base fait le **rappel** (index GIN trigramme, qui vit avec les
données), nous faisons l'**ordre** (Jaro-Winkler, sur quelques dizaines de
candidats). Vérifié que `fuzzystrmatch` n'offre pas jaro sur l'image, donc
l'ordre est en Rust — `src/jaro.rs`, qui passe les valeurs de référence de
Winkler.

Puis **le trigramme indexe les chunks** et non l'entité : l'extrait devient le
texte du chunk, sans appariement de spans.

Et les **accents sont normalisés des deux côtés** — inspiré de ragkit, avec son
piège : `unaccent()` est STABLE, donc inutilisable dans une expression d'index ;
il faut un enrobage IMMUTABLE avec le dictionnaire nommé, employé à l'index **et**
à la requête.

### Les deux forks de Kuzu, repérés et tranchés

Dans [`docs/3-septembre-2026-14h42/`](../../../docs/3-septembre-2026-14h42/) à la
racine, par la session `rag3db-57`.

**LadybugDB : non.** Pas d'histoire git commune, renommages en sens contraire,
et surtout leurs extensions et API sorties en dépôts séparés — 171 de nos
fichiers ne se fusionnent plus fichier par fichier. Leur descente de prédicat
sert l'ART, la nôtre la similarité : orthogonales.

**Vela : oui, par les lectures.** Même histoire git, aucun renommage, tout dans
un arbre. Trois lignes reportées, et **mesuré** : un second processus peut lire
pendant qu'un écrivain travaille, ce qu'il lit est cohérent, et le seul refus
observé (`Couldn't replay shadow pages under read-only mode`) est **transitoire**.

### Le reste

- rag3weaver porte sa **licence LRSL** avant que la racine passe en MIT.
- **codeparsers** est sorti en dépôt séparé (session `rag3db-b5`), `.h` envoyé à
  la grammaire C++ (+14 041 scopes), et les fichiers texte entrent dans l'index.
- L'**identité git** professionnelle est retirée de l'historique de codeparsers.
- La branche a **fusionné dans `master`**, en avance rapide.
- Doc de vision 14 : [le schéma comme artefact](../vision_roadmap_08_2026/14-le-schema-comme-artefact.md).

## 2. Trois défauts trouvés *dans nos propres tests*

Ils méritent leur section parce qu'ils sont de la même famille, et que la
journée l'a fait ressortir trois fois : **c'est le silence qui est le défaut,
pas l'erreur.**

**Quatre tests de `serveur.rs` tombaient au hasard.** `port_libre()` liait le
port 0 puis relâchait. Deux fausses pistes avant la bonne — la course est
**interne** (un test relâche, un voisin prend et tient), et une adresse de
bouclage privée n'y change rien puisqu'un écouteur sur `0.0.0.0` répond partout.
La correction : ces tests n'ont jamais eu besoin d'un port **liable** mais d'un
port **sourd**. `127.0.0.N:1`, où lier demande des privilèges. 0 échec sur 100.

**`scope_bm25_index_per_cell_never_sees_the_other_cell` criait à la fuite** là
où le cloisonnement était intact : ses deux textes « exclusifs » partageaient le
mot `private`, et le test ne tenait que sous l'ancien mode `Contains`. Corrigé
au montage — des mots disjoints — pour qu'il soit indifférent au mode.

**`e2e_postgres` mentait sur elle-même** : sept tests partageant une base, chacun
rasant le schéma, donc six échouaient en parallèle. Le `--test-threads=1` que
j'utilisais corrigeait le symptôme et laissait le piège. Le verrou est
maintenant dans la suite.

## 3. Ce qui reste à réparer, dans l'ordre

L'ordre est celui du **coût de ne pas le faire**, pas celui de la difficulté.

### 1. La marque d'eau d'ingestion — *préalable, pas suite*

`Consistency::{Immediate, Eventual, Strict}` est **intra-processus** : la file
d'attente vit dans le `Catalog` de l'écrivain, en mémoire. Vérifié par la
session voisine — `Strict` appelle `self.drain()`, et `has_pending()` ne teste
que `!self.pending.is_empty()`. Un lecteur d'un autre processus a son propre
catalogue, dont la file est vide : il demande `Strict` et obtient `Immediate`,
**sans que rien ne le dise**.

Le verrou n'a jamais protégé de ça — il rendait l'accès concurrent
*impossible*, pas *ordonné*. On ne perd donc rien qu'on avait : on découvre une
garantie qui n'a jamais franchi la frontière du processus, et qui devient
visible maintenant qu'on peut la franchir.

**Forme proposée** : l'écrivain publie son avancement dans `_catalog_meta`, que
les deux backends savent déjà écrire ; le lecteur qui veut `Strict` attend que
la marque dépasse ce qu'il attend.

**Pourquoi en premier** : c'est le préalable à ce que `rag3daemon` cesse de
relayer les lectures, et tant qu'il relaie, rien ne presse — mais bâtir dessus
sans elle installerait une promesse fausse.

### 2. La reprise sur refus transitoire dans `rag3daemon`

Le refus de pages fantômes est transitoire : cinq à six sur quatre-vingts
cycles, résolus en trois tentatives, aucun perdu. Quelques millisecondes
d'attente suffisent. **Ce n'est pas une lecture qui échoue, c'est une lecture
qui attend.** À faire avec le 1, pas avant.

### 3. Les poids du combo, qui restent une supposition — et le banc n'existait pas

**Correction** : ce document affirmait plus bas que « le banc qui les tranchera
est `e2e_phase0b` ». C'était faux. `e2e_phase0b` est une suite de tests, pas une
mesure : elle avait départagé les modes BM25 parce que des tests passaient ou
non selon le mode, pas parce qu'elle chiffrait quoi que ce soit. Le banc a été
écrit depuis (§6) et il mesure le seuil ; les poids restent à y ajouter.


0,35 trigramme / 0,65 Jaro-Winkler, posés au jugé. Le banc qui les tranchera est
`e2e_phase0b`, celui qui a déjà départagé `Contains`, `ContainsSplit` et
`Parse`. Ragkit mesure 60 % / 85 % / 87 % sur **des noms de médicaments** — ça
valide la *forme* à deux étages, pas les *poids* pour un corpus de code et de
documents.

### 4. Le seuil de confiance — **mesuré, puis posé**

Voir §6. Fait sur le chemin trigramme ; reste à mesurer sur le chemin lucivy et
sur le vecteur, qui demandent un vrai embarqueur.

### 5. Le filtre utilisateur sur le chemin texte natif — **fait**

`search_texte_natif` recevait la **cellule** mais pas la condition de filtre
générale : ni appliquée, ni signalée.

Le test écrit avant la correction — `le_filtre_utilisateur_tient`, trois produits
de même description et de prix différents — en a trouvé **deux** au lieu d'un :

| chemin | ce qu'il faisait |
|---|---|
| texte natif | rendait les trois produits sous `price < 20`, **en silence** |
| vectoriel filtré | `missing FROM-clause entry for table "p"` — **toute** recherche sous filtre utilisateur mourait |

Même cause : la jointure chunk→parent était écrite en **Cypher en dur** dans
`compile_filter_for_vector`, alors que tout le reste du filtre passait déjà par
le dialecte (`filter_join_clause`). Le backend postgres recevait ce `MATCH`
sous le nom `filter_match` et le **jetait**.

Corrigé par `SchemaDialect::chunk_parent_join` — `MATCH (n)-[:X_CHUNKED_FROM]->(p:X)`
d'un côté, `JOIN X AS p ON p._uuid = n._parent_uuid` de l'autre — et par
`compile_filter_utilisateur`, qui rend le filtre **sans la cellule** : celle-ci
reste un paramètre à part, pour qu'une frontière de locataire ne dépende jamais
de la présence d'un `WHERE`.

**Puis le silence lui-même a été fermé.** `SearchBackend::honore_le_filtre()`
rend `false` par défaut — délibérément : un backend neuf est bruyant tant qu'il
n'a pas dit le contraire. Quand il ne garantit rien, les deux chemins poussent
« les résultats ne sont peut-être PAS restreints au domaine demandé ».

Et en vérifiant que l'avertissement **arrive** à l'agent, trois coupures :

| où | ce qui se perdait |
|---|---|
| `VectorSearchNode` | disait « les résultats ne sont PAS restreints » dans son **journal**, que l'appelant ne lit pas |
| `merge_port_values` | ne fusionnait pas deux `SearchMeta` — donc on ne *pouvait* pas brancher deux signaux sur le même port |
| schéma des fabriques | `meta` absent de `BM25SearchNode` et `RenderResultsNode`, alors que les nœuds l'émettaient — la **composition** ne le voyait pas |

La dernière est la plus coûteuse : `search.mmd`, l'outil réellement offert aux
agents, compose par-dessus `search_base` et réécrivait la fiche à partir des
seuls résultats. **Aucun avertissement du moteur n'atteignait un agent depuis
que la composition existe** — pas même celui du 29 août, qui avait pourtant été
réparé un étage plus bas.

Fermé par un passe-plat `render.meta`, deux arêtes dans les gabarits, et un
test qui compare le schéma déclaré à l'implémentation.

**Puis les deux sources ont été unifiées.** Vérifié d'abord qu'aucun nœud
feuille ne calcule ses ports depuis sa configuration — seuls `Graph` et
`GraphNode` le font, et eux n'ont pas de schéma statique. Les ports sont donc
une constante du **type**, pas de l'instance : une seule écriture suffit.

Le schéma de fabrique reste le rédacteur, parce qu'il est le seul disponible
**avant** d'avoir une instance : le sondeur de `GraphNodeFactory::templated`
lit des schémas de type sur une définition où les `$param` peuvent encore
traîner. Les 44 implémentations de nœud délèguent maintenant à
`ports_declares(&XFactory)` — 372 lignes supprimées pour 137.

Deux choses se sont vues en le faisant :

- Le test ne comparait que les **noms** de ports. `KBGatherNode.aggregates`
  était `required` au schéma et facultatif au nœud ; en unifiant, le runtime
  s'est mis à attendre une entrée qui ne vient pas — sept tests d'ingestion.
  Le nœud avait raison (il se nourrit aussi du service `pending_aggregates`),
  le schéma a été corrigé, et le test compare désormais nom, type **et**
  `required`.
- Le test ne couvrait que les 26 types constructibles sans configuration. Il
  couvre les 34, avec une table de configurations minimales, et **refuse de
  rétrécir** : un type neuf sans entrée dans cette table fait échouer le test
  au lieu d'être sauté en silence.

### 6. `test/api/db_locking_test.cpp` affirme un contrat mort

Il énonce l'exclusion lecteur/écrivain que le report de Vela a levée. Il n'est
compilé par personne et ne l'était déjà pas en amont — code de test mort hérité
de Kuzu. À supprimer ou à réécrire, pas à laisser mentir.

### 7. Le fond de la concurrence de Vela, non relu

MVCC, rotation de WAL, points de reprise non bloquants. C'est le plancher de
l'écriture multi-processus, et personne ne l'a examiné. **Préalable à toute
adoption de leur moitié écriture**, pas à la lecture qu'on vient de prendre.

### 8. Le renommage `kuzu` → `rag3db`, à questionner

2 899 fichiers à chaque reprise de l'amont. Ce n'est pas urgent, mais c'est une
dépense récurrente dont personne n'a énoncé le bénéfice.

## 4. Ce qui n'a pas été fait, et qu'il ne faut pas croire fait

- **Rien n'a été compilé de LadybugDB ni de Vela** hors le report de trois
  lignes. Leurs builds restent non tentés.
- **Les 171 fichiers hors du cœur** face à Ladybug ne sont pas chiffrés, et ne
  peuvent pas l'être depuis ce dépôt.
- **Aucun de mes tests postgres n'éprouve `Strict`** : ils passent tous
  `Immediate`, et tiennent parce que l'ingestion est synchrone juste avant.
- **Les deux tests rouges de `e2e_simple_entity` sont réparés** — et le
  diagnostic de mars était faux. Voir §5.
- **Le miroir Rust de `search_base` reste couplé au graphe de production.**
  `search_in_rust`, 70 lignes dans les tests, recopie `search_base.mmd` nœud
  par nœud pour prouver que l'analyseur Mermaid et l'API Rust construisent la
  même chose. L'intention est juste, le sujet ne l'est pas : toute évolution du
  graphe de production oblige à éditer le miroir. **Le générer serait pire** —
  il prouverait que l'analyseur est d'accord avec l'analyseur. Le bon geste est
  un fixture minimal dédié, découplé de `search_base`.

## 5. Le batch delete/update — six mois de mauvais diagnostic

`simple_batch_delete_multiple` et `simple_batch_update_multiple` étaient rouges
depuis le 8 mars 2026. Le
[rapport de l'époque](../../../docs/8-mars-2026-01h54/12-rapport-progression-batch-crud.md)
concluait à un index plein texte **corrompu par le `DETACH DELETE`**, et
proposait de le reconstruire.

Ce n'était pas ça. Le montage donnait la même phrase aux trois produits :

```
Alpha : "Alpha description content here"
Beta  : "Beta  description content here"
Gamma : "Gamma description content here"
```

Chercher « alpha description » après avoir supprimé Alpha remontait **Beta**,
sur le mot `description`. Le test n'affirmait donc rien sur la suppression. Même
histoire côté mise à jour, avec une seconde couche : `name` est indexé
(`is_title`), donc « Alpha original » remontait Alpha **par son propre nom**,
quel que soit son contenu.

C'est exactement le défaut trouvé le matin même sur
`scope_bm25_index_per_cell_never_sees_the_other_cell`. **Un test dont les
fixtures partagent leur vocabulaire n'affirme rien** — et ici il a fait accuser
le moteur pendant six mois.

Réparé au montage : trois vocabulaires disjoints (`xylophone`, `tourbillon`,
`sextant`), aucun mot du contenu qui reparaisse dans le nom, et un contrôle
positif à côté de chaque assertion négative. Les deux tests passent. La
suppression et la réécriture retirent bien l'ancien contenu de l'index.

### Et un vrai défaut, trouvé en passant

`UpdateResult.chunks_created` et `chunks_deleted` étaient des **zéros écrits en
dur** dans `UpdateRecordNode`, et l'événement `EntityUpdated` partait avec les
mêmes. Deux champs présentés comme des mesures qui n'en étaient pas — visibles
dans la sortie du test depuis toujours (`status=Updated, reembedded=true,
chunks_deleted=0, chunks_created=0`) sans que rien ne les regarde.

Le nœud ne pouvait pas les connaître : le rechunkage a lieu en aval
(`rechunk_delete`, `rechunk_chunk`). Les nombres existaient pourtant —
`batch_cascade_delete_returning_count` rend `uuid, count(c)` et le détail
partait dans une somme. Ils remontent maintenant par un service `chunk_counts`,
recollés dans `Catalog::drain`, qui émet aussi l'événement à cet endroit pour
qu'il ne porte plus de zéro. Le test les affirme.

## 6. Le seuil de confiance — ce que le banc a dit

Le banc est `e2e_postgres::ou_vit_la_frontiere_entre_le_vrai_et_le_bruit` :
vingt descriptions, cinq vocabulaires disjoints, et quatre familles de requêtes.

**La première version ne mesurait rien**, et c'est la leçon la plus utile :
toutes ses requêtes reprenaient les mots exacts de leur cible, donc tout valait
1,0000 — trigramme et Jaro rendent 1 sur une correspondance verbatim — et le
bruit lointain rendait zéro. Séparation parfaite, information nulle. J'ai
d'ailleurs failli lire ces 1,0000 comme une normalisation par requête ; le score
n'est pas normalisé, c'étaient mes requêtes qui étaient trop faciles.

La frontière vit dans le **bruit proche** : des mots du corpus, une combinaison
qui ne désigne rien. C'est la confusion réelle, pas « zzzz qqqq ».

### Ce que la mesure a trouvé

| famille | plancher 0,6 (défaut) | plancher 0,3 |
|---|---|---|
| exacte | 1,00 | 1,00 |
| dégradée (fautes, troncature) | **0,00** — « sauterau baroqe » ne rapporte rien | ≥ 0,84 |
| bruit proche | ≤ 0,74 | ≤ 0,76 |
| bruit lointain | rien | rien |

**Deux défauts distincts, et le premier était invisible.**

`<%` ne rapporte que ce qui dépasse `pg_trgm.word_similarity_threshold`, 0,6 par
défaut. Ce plancher décidait donc ce que Jaro avait le droit d'ordonner : le
dessin à deux étages ne fonctionnait pas — la base tranchait à la place de
l'étage qui sait trancher. Une requête à deux fautes rendait **zéro résultat**,
et une falaise de rappel ne se voit pas comme une erreur, elle se voit comme
« ça n'existe pas ». Le backend pose maintenant 0,3 (`PLANCHER_RAPPEL`).

Le rappel ouvert, la frontière apparaît. C'est une mesure, pas le 0,7 de
ragkit — le leur a été calibré sur des noms de médicaments, des noms propres
courts où deux chaînes proches désignent la même molécule. La **forme** est
établie ; le **nombre** est à remesurer sur un corpus réel, ce corpus-ci étant
synthétique et petit.

### Les poids, mesurés à leur tour

Second banc, `les_poids_du_combo_se_mesurent` : il demande au backend ses
candidats **bruts**, calcule Jaro lui-même et balaie les partages en Rust — donc
il mesure la **formule**, sans exposer de réglage que la production n'utiliserait
pas. Critère : la marge entre la pire requête dégradée et le meilleur bruit
proche.

| trigramme / Jaro | marge |
|---|---|
| 1,00 / 0,00 | **−0,03** |
| 0,35 / 0,65 (l'ancien réglage, au jugé) | ≈ +0,077 |
| **0,20 / 0,80** | **+0,086** |
| 0,00 / 1,00 | +0,058 |

Le fait qui compte n'est pas le nombre : **le trigramme seul ne sépare pas**,
sa marge est négative. Bon signal de *rappel*, mauvais signal de *classement* —
la thèse des deux étages, mesurée plutôt que supposée. Le plateau est plat entre
0,10 et 0,30 ; 0,20 est au milieu.

### Deux scores, pas un — et c'est ce qui unifie les backends

Le score qui **ordonne** et le score qui **rassure** ne peuvent pas être le même
nombre. Le premier est relatif et change de nature selon le backend : Jaro
pondéré de trigramme sur PostgreSQL, du **BM25 non borné** sur lucivy. Aucun
seuil ne s'y pose.

Le second doit être absolu. C'est **Jaro-Winkler pur**, recalculé chez nous sur
le texte rendu, borné, identique quel que soit qui l'a rendu :

| famille | Jaro pur |
|---|---|
| exacte | 1,00 |
| dégradée | ≥ 0,915 |
| bruit proche | ≤ 0,857 |

`SEUIL_CONFIANCE = 0,88` partage l'intervalle, et **les deux chemins de texte
passent par la même fonction** (`marquer_la_confiance`). Le même nombre veut
donc dire la même chose sur PostgreSQL et sur lucivy.

lucivy sait aussi faire du Jaro-Winkler, mais comme **métrique d'acceptation
floue** (`fuzzy_metric`) : elle élargit le rappel, elle ne rend pas de score.
C'est l'autre moitié du même outil, et sa place est celle de `PLANCHER_RAPPEL`
côté PostgreSQL — le rappel, pas la confiance. À brancher quand on voudra
ouvrir le rappel de lucivy comme on vient d'ouvrir celui du trigramme.

### Marquer, pas filtrer

Rien n'est retiré. Une recherche sous le seuil pousse un avertissement :

> rien de probant pour « couronne metamorphique » : le meilleur candidat est à
> 0.72, sous le seuil de 0.80 — ce qui suit partage des mots avec la requête
> sans forcément y répondre

Et cet avertissement **arrive** à un agent, ce qui n'était pas vrai ce matin :
c'est la tuyauterie réparée à la §4 (méta fusionnable, passe-plat `render.meta`,
schémas de fabrique recousus) qui le porte jusqu'à la fiche. Les deux moitiés de
la journée se rejoignent là.

Le test affirme les trois choses : que la falaise existe au plancher par défaut,
que 0,80 partage bien l'intervalle une fois le rappel ouvert, et qu'une requête
exacte n'est **pas** marquée.

## 7. Le chemin du retour vers lucivy — il n'existait pas

lucivy pèse trop en espace disque pour de la production aujourd'hui, et sera
allégée. D'ici là, `MoteurTexte::{Auto, Lucivy, Natif}` doit rester une
**option** : chaque moteur marche à sa sauce, aucun n'est éliminé.

Or `set_moteur_texte` n'avait **aucun appelant**. Les trois valeurs existaient
sur le papier. Un test les a empruntées — et lucivy sur PostgreSQL rendait zéro.

Trois défauts en file, chacun caché par le précédent :

| # | ce qui se passait |
|---|---|
| 1 | l'insertion rend `ID(n)` (chaîne) sur rag3db et `_row_id` (**entier**) sur PostgreSQL ; le code ne lisait que la chaîne, donc la table uuid→identifiant restait **vide** |
| 2 | `InternalNodeId::parse` refusait un entier nu — et **un test épinglait ce refus** |
| 3 | `resolve_and_enrich_chunked` composait son Cypher en dur : lucivy est un index Rust, sa résolution ne parlait que rag3db |

Le premier est le pire : sans table d'identifiants, le cache **et** l'indexation
lucivy étaient sautés **sans un mot**. Un index se créait, se commitait sans
erreur, et ne contenait aucun document. La recherche rendait zéro, ce qui
ressemble à « ça n'existe pas ».

Réparé : la lecture de l'identifiant accepte les deux types et **avertit** quand
il est illisible ; `parse` accepte le `_row_id` nu (la table est déjà nommée par
la requête, donc `table_id` vaut 0) ; et la résolution passe par une nouvelle
méthode de dialecte, `resolve_parents_with_chunks` — `OPTIONAL MATCH` d'un côté,
`LEFT JOIN ... ON c._parent_uuid = n._uuid` de l'autre. L'ordre des colonnes est
extrait dans `colonnes_de_chunk`, parce que la lecture qui suit se fait **par
position** : un ordre divergent ne donnerait pas une erreur mais des champs
échangés.

**Et une quatrième chose, de la même famille que la journée.** Dix montages de
services à la main, dans sept fichiers de tests, reconstruisent le registre du
catalogue — et aucun n'avait le dialecte. Complétés, mais la duplication reste :
c'est le troisième cas aujourd'hui de deux copies tenues à la main qui divergent,
après les schémas de nœuds et le miroir Rust de `search_base`. Un
`Catalog::register_search_services` les fondrait.
