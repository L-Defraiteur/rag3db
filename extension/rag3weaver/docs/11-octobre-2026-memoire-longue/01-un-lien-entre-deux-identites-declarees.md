# Un lien entre deux identités déclarées — la face dataflow de `Write::Link`

*11 octobre 2026. Mémoire longue. La page avant le code, pour Lucie et pour la
relecture de rag3db-97 (dataflow) et rag3db-6f (l'IR). Rien n'est codé ; le
témoin de la garde existante, lui, est sur master (`49142d58f`).*

**La décision** (Lucie, au réveil) : oui aux deux lots, dans cet ordre — le
nœud, puis le huitième refus de `"reactions"` —, « mais toujours générique — si
c'est un mécanisme qui doit être pensé encore plus loin dans l'abstraction, à
faire aussi ». Donc pas « un nœud pour ancrer une mémoire » : la face dataflow de
`Write::Link`, valable pour toute paire d'entités déclarées, gérées ou non, avec
des gardes **déclarées** et non câblées.

**Ce que la page conclut, en une phrase** : il ne faut pas de nœud de plus. Il
faut **retirer à `RelationBatchNode` une garde qui ne protège rien**, lui donner
ce qui lui manque pour être général, et faire passer **tous** les liens posés par
un outil par le même chemin — y compris celui d'`AddRefNode`, qui annonce
aujourd'hui des liens qui n'existent pas.

## 1. Ce qui existe, mesuré

**L'IR.** `Write::Link { relation, ends: Option<(String, String)>, props }`
(`ir/src/form.rs:248`, `70a1b97ab`) : les arêtes arrivent **par uuid** (`from_uuid`,
`to_uuid`, puis les propriétés). Un seul constructeur en production, le nœud de
drain du catalogue (`LinkRecordNode`, `dataflow/record_nodes.rs:1157`). Deux
traductions : rag3db, `UNWIND … MATCH (a:From {_uuid}), (b:To {_uuid}) MERGE
(a)-[r:REL]->(b)` ; PostgreSQL, `INSERT … ON CONFLICT (from_uuid, to_uuid) DO
NOTHING`. Idempotent dans les deux.

**Trois chemins posent une relation depuis un outil**, et aucun ne fait la même
chose :

| | bouts désignés par | existence des bouts | garde | propriétés | écrit par |
|---|---|---|---|---|---|
| `RelationBatchNode` | identité (`hashsafe` exact) | **vérifiée** (`get_many`), lot refusé sinon | **refuse tout bout géré** | refusées | `mettre_en_file_les_liens` + `drain()` |
| `AddRefNode` (la part « lien ») | identité de la fiche → uuid | **non vérifiée** | aucune | aucune | `Catalog::link` |
| `Catalog::link` | uuid | non vérifiée | aucune | oui | file + drain |

**Le témoin de la garde** (`tests/e2e_garde_des_liens.rs`) établit, champ à champ,
ce qu'elle protège :

1. elle refuse deux bouts gérés, et n'écrit rien — épinglée telle qu'elle est ;
2. la même relation posée par le chemin qu'elle barre, entre une entité à cycle
   de vie, révision, dates serveur et date de transition, une entité révisée et
   une entité **immuable** : **aucun champ d'aucun bout ne change** ;
3. après le lien, chaque garantie déclarée tient : la révision lue avant est la
   bonne, l'immuable rejoue à l'identique et refuse d'être modifié, la transition
   déclarée passe et date sa revue, le retour non déclaré se refuse.

**Elle ne protège donc aucune garantie déclarée.** Son histoire le confirme :
arrivée le 20 septembre dans un commit de sauvegarde d'expériences (`ab95c3a2d`),
sans justification, copie de la garde d'`EntityBatchNode` — dont la seule raison
écrite est « ne pas contourner les garanties de l'écriture métier » d'une
**écriture de lignes en lot**, et dont la clause jumelle de cycle de vie a été
retirée le 2 octobre comme « une prudence » (`213eeb214`). Poser une relation
n'écrit aucune ligne. La garde délimitait une portée — les liens de l'expérience
MTG, entre entités brutes —, pas un invariant.

**Et un défaut trouvé en chemin** : `Catalog::link` vers un bout absent rend
`Ok` et n'écrit rien (le `MATCH` ne trouve pas, le `MERGE` ne fait rien, sans
erreur). `RelationBatchNode` s'en garde ; `AddRefNode` non, et rend
`executed: true` pour une fiche qui n'existe pas. Épinglé par le même témoin,
ticket `2026-10-11-un-lien-vers-un-bout-absent-rend-ok-sans-rien-ecrire.md`.

## 2. Plus loin dans l'abstraction : un seul nœud, pas deux

La question de Lucie était : faut-il unifier « l'instantané » et « les
identités » sous une seule forme ? **La carte répond qu'il n'y a qu'une forme** :

- `RelationBatchNode` **n'a rien d'un instantané**. Il n'a pas de paramètre
  `snapshot`, ne marque rien, ne s'inscrit dans aucune session (c'est
  `EntityBatchNode` qui en a). Son seul lien avec les instantanés est son nom de
  fiche (`link_snapshot.mmd`) et la phrase de sa garde ;
- ce qu'il fait — **des identités déclarées en entrée, l'existence vérifiée, le
  lot validé entier avant d'écrire, un `Write::Link` en sortie** — est
  exactement la face dataflow demandée.

Donc : **`RelationBatchNode` devient le nœud de lien général**, sans nouveau type
(la règle du dépôt : un type de plus ne se justifie pas, deux qui font la même
chose se fondent). Ce qui change en lui :

1. **la garde câblée part.** Elle est remplacée par les gardes **déclarées** du
   §3 ; sans déclaration, un lien entre deux entités déclarées est permis, gérées
   ou non ;
2. **les propriétés déclarées sont permises** — un lien reposé avec d'autres
   propriétés prend les dernières, sans arête en double, dans les deux dialectes
   (`MERGE … SET`, `ON CONFLICT … DO UPDATE` ; relu par rag3db-6f, qui signale que
   le second suppose la contrainte d'unicité `(from_uuid, to_uuid)` sur la table de
   relation, à vérifier quand PostgreSQL sera monté). `RelationDef.properties` existe
   (`config.rs:123`) et `Write::Link` porte des `props` ; le refus « use an entry
   entity for payload » tombe pour une relation qui déclare ses propriétés, et
   reste pour celle qui n'en déclare pas (une propriété non déclarée se refuse en
   la nommant) ;
3. **il draine la fermeture de ses liens, pas toute la file** — *corrigé après la
   relecture de rag3db-97*. J'avais écrit « ses seuls liens » : ce serait faux dans
   un cas réel. Un bout mis en file dans le même run sans être encore posé ne serait
   pas trouvé par le `MATCH` du lien, et le lien se perdrait sans erreur. Le
   catalogue sait déjà drainer la **fermeture** d'une cible et remettre le reste en
   file (`drainer(…, Some((graine, pour_ecrire)))`, que `link_jusqu_a` emploie par
   `tenir_l_exigence_d_ecriture`) : `RelationBatchNode` appelle cela, exposé en
   `pub(crate)`, au lieu de `drain()` ;
4. **il dit qu'il n'est pas atomique, et la transaction vient à part.** Borner le
   drain ne le rend pas atomique. `begin`/`commit`/`rollback` sont dans
   `DbConnection` depuis `bb7dddde5`, et la transaction par paquet les emploie.
   Les deux dialectes déclarent `transactions: true` (depuis `87e9739c9` ; j'avais
   écrit le contraire, corrigé par rag3db-97). **Le vrai obstacle** : le drain de
   la fermeture peut emporter des écritures qui ne sont pas des liens — des
   créations en file, avec leurs fichiers de plein texte et leurs vecteurs, hors
   de la transaction de la base. Un `ROLLBACK` ne les défait pas, d'où
   l'empoisonnement du catalogue dans la transaction par paquet. La marche 1 rend
   donc `atomic: false`. **La forme légère, pour une marche à elle** (rag3db-97) :
   si la fermeture à drainer ne contient **que des liens**, `begin`/`commit`
   autour est sûr, puisqu'un lien n'a aucun effet hors de la base ; sinon,
   `atomic: false` ;
5. **il compte ce qui est posé — avec l'IR, pas sans elle.** Un `MERGE` ne dit pas
   s'il a créé. Sans relire (ce qui ouvrirait une course), le nœud ne peut pas
   distinguer « posé » de « déjà là ». C'est donc le compte que rag3db-6f propose
   pour `Write::Link` (créé / existant : `RETURN count(r)` côté rag3db, le
   `rowcount` côté PostgreSQL) qui fera le rapport honnête. D'ici là, le rapport
   dit `submitted` et `verified`, pas `posed`.

**La fiche** : `templates/tools/link.mmd`, générique, une ligne
(`RelationBatchNode(relation=$relation, links=$links)`), décrite pour un agent
(« relie des choses déclarées par leurs champs d'identité »). `link_snapshot.mmd`
reste le temps que les outils de l'expérience MTG (`experiments/mtga`, seuls à
la déclarer, six outils, aucun bout géré) la quittent, puis part.

**`AddRefNode` passe par le même chemin** pour sa part « lien ». La vérification
d'existence va **au catalogue**, pas dans un module de nœuds (relecture de
rag3db-97) : c'est là que l'identité devient uuid, et `entity_uuid`, déterministe,
ne prouve rien sur l'existence. Une méthode du genre
`Catalog::bouts_existants(relation, paires) -> Result<Vec<(uuid, uuid)>, BoutsAbsents>`
(résolution, puis lecture par lot), appelée par les deux nœuds et par `link`.
**Un bout « existant » est en base ou en file** : la même raison qu'au point 3
— un bout écrit plus tôt dans le même run, pas encore drainé, n'est pas absent.

**La lecture d'existence passe par l'IR** (relecture de rag3db-6f) : pas
`get_many`, qui ramène les lignes entières, mais
`Select::by_uuids(table, [Column::Node("_uuid")])`, traduit par le dialecte et
exécuté avec `$uuids`. Elle rend une ligne par uuid présent ; les absents sont
ceux qui manquent. Par tranches de 512. rag3db la traduit
(`UNWIND $uuids AS u MATCH (m:T {_uuid: u}) RETURN u, m._uuid`) ; **PostgreSQL
pas encore**, et `Catalog::link` est le verbe de tous les backends : la
traduction PostgreSQL de ce `Select`
(`SELECT "_uuid" FROM t WHERE "_uuid" = ANY($uuids)`) est donc une **condition
de la marche 1 bis**, que rag3db-6f pose. C'est ce qui ferme le défaut de
l'annonce fausse pour lui, sans attendre que le verbe du catalogue change.

**Et pourquoi l'IR ne bouge pas** (relu et accepté par rag3db-6f) : `Write::Link` reste en uuids. L'identité
(`hashsafe`) est une notion du manifeste ; l'uuid en dérive de façon
déterministe (`Catalog::entity_uuid`), et c'est le nœud — l'étage qui connaît le
manifeste — qui la résout. Un dialecte n'a pas à savoir ce qu'est un champ
d'identité. *(Point à relire par rag3db-6f.)*

## 3. Les gardes déclarées

Lucie : « les gardes de cycle de vie déclarées et non câblées ». Une seule garde
proposée, parce que c'est la seule qu'un usage réel demande aujourd'hui, et
qu'une liste de gardes « au cas où » est une liste que personne ne relira :

```json
"relations": {
  "ANCHORED_TO": { "from": "Memory", "to": "Subject",
                   "from_states": ["current", "to_review"] }
}
```

**`from_states` / `to_states`** : le bout doit être dans l'un de ces états de son
cycle de vie pour qu'un lien **s'y pose**.

**La couche, choisie en codant et relue par rag3db-97** : c'est une **politique
d'écriture du backend**, comme `writes` pour une entité — une enveloppe
`BackendRelation` autour de la `RelationDef` du catalogue, qui reste intacte.
Deux raisons : une garde d'états **se recharge** (ce n'est pas une migration du
schéma, et la comparaison d'ouverture ne regarde que les définitions nues) ; et
les 24 constructions de `RelationDef` du dépôt n'ont pas à changer. **Le prix,
écrit là où l'on déclare** : un appel nu à `Catalog::link` ne voit pas la garde —
un verbe brut, comme du SQL. Elle tient pour **les deux nœuds qui posent des
liens** (`RelationBatchNode` et `AddRefNode`), par la même fonction que
l'existence (`Catalog::verifier_les_bouts`), et c'est épinglé par un témoin.

L'état d'un bout est lu **tel qu'il sera après la file** : en base, recouvert par
une création en file (son état, ou l'initial du cycle), puis par une mise à jour
en file qui change l'état. Vérifié au chargement (les états
existent dans le cycle de vie du bout ; un bout sans cycle de vie ne peut pas en
déclarer) et à l'écriture (le lot est refusé en nommant le bout et son état).
Pour la mémoire : on n'ancre pas une mémoire `superseded`.

**Ce qui n'est pas proposé, et pourquoi** :

- **faire monter la révision d'un bout quand on le relie** : le témoin montre
  qu'aujourd'hui aucun chemin ne le fait, et une révision protège des éditions
  concurrentes **d'un record** ; une ancre n'en est pas une. Si un usage veut
  qu'« ancrer » compte comme « modifier », ce sera une déclaration de plus
  (`touches: ["from"]`), pas un défaut ;
- **défaire un lien** : pas dans ce lot. Une mémoire remplacée garde ses ancres,
  et c'est son état (`superseded`) qui dit qu'elles ne comptent plus. *Correction
  après la relecture de rag3db-6f* : j'avais écrit que `Write::Link` n'avait pas
  d'inverse dans l'IR. Il en a un, **`Write::Unlink`**, sur master depuis
  `c3734d975` (des arêtes par leurs bouts, traduit par les deux dialectes ;
  l'annulation des liens de l'ingestion passe déjà par lui). Le jour où un outil
  doit défaire un lien, c'est un paramètre de ce même nœud, et l'IR est prête.

## 4. Ce que le gabarit `memory` en reçoit

Deux outils, déclarés, sans une ligne de code de plus que le §2 :

```json
"anchor":         { "graph": "../../tools/link.mmd", "bindings": { "relation": "ANCHORED_TO" },
                    "description": "Ancre des mémoires à des sujets. Une mémoire ancrée repasse « à revoir » quand son sujet change." },
"supersede_link": { "graph": "../../tools/link.mmd", "bindings": { "relation": "REPLACES" },
                    "description": "Dit qu'une mémoire en remplace une autre." }
```

Avec `from_states` sur les deux. **La réaction `review_anchored` cesse alors de
tourner à vide** : le témoin `e2e_reaction_montee` pose aujourd'hui son ancre par
`Catalog::link` ; il la posera **par l'outil**, ce qui en fait du même coup le
témoin de bout en bout de la surface exposée.

## 5. Le huitième refus, après

Le refus de chargement : une réaction qui remonte une relation qu'aucun outil
n'écrit. **Il vient après ce lot**, sinon il rendrait le gabarit `memory` livré
inchargeable (la raison est écrite au ticket
`2026-10-11-une-reaction-peut-remonter-une-relation-qu-aucun-outil-n-ecrit.md`).

**Plus loin dans l'abstraction, ici aussi** : savoir qu'« un outil écrit la
relation R » ne doit nommer aucun type de nœud en dur. **Le mécanisme existe à
moitié** (relecture de rag3db-97) : `Choices::Relations` (`@relations`,
`node_registry.rs:41`) marque déjà un paramètre comme « nomme une relation
déclarée ». Il lui manque le sens : `Relations { acces: Lecture | Ecriture }`,
**sur le paramètre**, pas une liste au niveau du schéma du nœud, qui doublerait
les paramètres et dériverait. `RelationBatchNode` et `AddRefNode` marquent
`relation` en écriture ; `ReactTransitionNode` en lecture.

Le refus a déjà son traceur : `wired_params` (`graph_tool.rs`) suit quel paramètre
de nœud chaque paramètre d'outil alimente, liaison du manifeste ou valeur fixe de
la fiche. Pour chaque relation lue par une réaction, au moins un outil déclaré
doit l'écrire. **Un piège à couvrir d'un témoin** : un sous-graphe déclaré
(`GraphNodeFactory::templated`) qui expose un paramètre câblé sur le `relation`
d'un `RelationBatchNode` intérieur doit **hériter** de la marque, comme il hérite
déjà du type. La même marque servira à l'atelier (montrer qui écrit quoi) et aux
droits par clés.

## 6. Les marches et leur coût

*En passes, ordre de grandeur relatif.*

| marche | ce qu'elle fait | témoin | coût |
|---|---|---|---|
| **1** | `RelationBatchNode` général : garde retirée, propriétés déclarées, drain de la fermeture, transaction quand le dialecte en a ; `Catalog::bouts_existants` (base ou file), employé aussi par `AddRefNode` | `e2e_garde_des_liens` retourné (le lien entre bouts gérés **passe par l'outil**, les garanties tiennent) ; `add_ref` vers une fiche absente se refuse en la nommant ; propriétés déclarées posées, non déclarées refusées | 1 |
| **1 bis** | `Catalog::link` et la mise en file des liens **refusent un bout absent en le nommant**, vérifié par lot (une lecture des uuids par paquet) ; si le coût est visible, la variante par lot qui rapporte les bouts absents pour l'indexation | `un_lien_vers_un_bout_absent_rend_ok_et_n_ecrit_rien` **retourné** (le ticket se ferme) ; `e2e_code` avant/après, durée et comptes identiques | ½ à 1 |
| **2** | `from_states` / `to_states` dans la déclaration de relation, vérifiés au chargement et à l'écriture | un état hors liste refusé en le nommant ; un état inconnu refusé au chargement | ½ |
| **3** | `link.mmd`, et `anchor` / `supersede_link` dans le gabarit `memory` | `e2e_reaction_montee` ancre **par l'outil** ; `gabarits` charge | ½ |
| **4** | le sens sur `Choices::Relations` (lecture / écriture), le huitième refus par `wired_params` | le gabarit `memory` charge ; un manifeste à réaction sans écrivain se refuse en nommant la relation ; un sous-graphe qui expose le `relation` d'un nœud intérieur hérite de la marque | ½ à 1 |

En tout, **deux et demie à quatre passes**, la marche 1 bis portant la mesure. Les compteurs de liste blanche ne bougent pas
(aucun type nouveau) ; le ticket du nœud manquant se ferme à la marche 1 (« il
n'y avait pas de nœud manquant, il y avait une garde de trop ») ; celui de
l'ancre à la marche 3 ; celui du huitième refus à la marche 4 ; celui du lien
vers un bout absent à la marche 1 bis.

## 7. Les quatre choix — tranchés par l'orchestration, renversables par Lucie

1. **Fondre** : `RelationBatchNode` devient le nœud général, pas de type nouveau
   (§2).
2. **Le verbe du catalogue refuse lui-même un bout absent, en le nommant**
   (tranché par l'orchestration, renversable par Lucie) : « `Ok` sans rien
   écrire » est la classe de défaut de toute la nuit. **Mesuré, pas supposé** :
   l'indexation du code pose des centaines de milliers de liens, donc la
   vérification se fait **par lot** — une lecture des uuids des deux bouts par
   paquet, jamais une par lien — et le coût se rend avant/après sur `e2e_code`.
   S'il est visible, l'indexation passe par une variante par lot qui **rapporte**
   les bouts absents (jamais un `Ok` muet). Le ticket du bout absent se ferme
   ici. Et le compte des arêtes posées que rag3db-6f propose dans l'IR
   (`RETURN count(r)`, `RETURNING`) reste un complément : il constate après
   coup ce que la vérification refuse avant, et attrape ce qu'elle ne peut pas
   voir (un bout retiré entre la lecture et l'écriture).
3. **Une seule garde déclarée** (`from_states`/`to_states`) ; pas de révision qui
   monte ; pas d'`Unlink` dans ce lot — `Write::Unlink` existe, le nœud y passera
   dans un lot à lui (§3).
4. **Le huitième refus par une marque sur le paramètre** (`Choices::Relations`
   avec son sens, §5), jamais une liste de types en dur.
