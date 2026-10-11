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
2. **les propriétés déclarées sont permises.** `RelationDef.properties` existe
   (`config.rs:123`) et `Write::Link` porte des `props` ; le refus « use an entry
   entity for payload » tombe pour une relation qui déclare ses propriétés, et
   reste pour celle qui n'en déclare pas (une propriété non déclarée se refuse en
   la nommant) ;
3. **il ne vide plus toute la file du catalogue.** Aujourd'hui il appelle
   `drain()`, qui draine **tout** ce qui attend, pas seulement ses liens, et
   « writes may be partial » sans retour arrière. Il doit drainer **ses** liens
   (`link_jusqu_a` avec exigence, ou un drain borné à sa relation) et rendre ce
   qui a été effectivement posé ;
4. **il compte ce qui est posé**, pas ce qui est soumis : `submitted`, `posed`,
   `unchanged` (déjà là), et un refus nommé si un bout a disparu entre la
   vérification et l'écriture — sous la forme C, les écritures d'un backend sont
   sérialisées, donc ce cas ne peut venir que d'un outil direct sur la base.

**La fiche** : `templates/tools/link.mmd`, générique, une ligne
(`RelationBatchNode(relation=$relation, links=$links)`), décrite pour un agent
(« relie des choses déclarées par leurs champs d'identité »). `link_snapshot.mmd`
reste le temps que les outils de l'expérience MTG (`experiments/mtga`, seuls à
la déclarer, six outils, aucun bout géré) la quittent, puis part.

**`AddRefNode` passe par le même chemin** pour sa part « lien » : la vérification
d'existence et le comptage deviennent une fonction partagée avec
`RelationBatchNode`, appelée par les deux. C'est ce qui ferme le défaut de
l'annonce fausse pour lui, sans attendre que le verbe du catalogue change.

**Et pourquoi l'IR ne bouge pas** : `Write::Link` reste en uuids. L'identité
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
cycle de vie pour qu'un lien **s'y pose**. Vérifié au chargement (les états
existent dans le cycle de vie du bout ; un bout sans cycle de vie ne peut pas en
déclarer) et à l'écriture (le lot est refusé en nommant le bout et son état).
Pour la mémoire : on n'ancre pas une mémoire `superseded`.

**Ce qui n'est pas proposé, et pourquoi** :

- **faire monter la révision d'un bout quand on le relie** : le témoin montre
  qu'aujourd'hui aucun chemin ne le fait, et une révision protège des éditions
  concurrentes **d'un record** ; une ancre n'en est pas une. Si un usage veut
  qu'« ancrer » compte comme « modifier », ce sera une déclaration de plus
  (`touches: ["from"]`), pas un défaut ;
- **défaire un lien** : `Write::Link` n'a pas d'inverse dans l'IR. Une mémoire
  remplacée garde ses ancres, et c'est son état (`superseded`) qui dit qu'elles
  ne comptent plus. Le jour où un lien doit vraiment partir, c'est un
  `Write::Unlink` à poser dans l'IR d'abord, avec ses deux traductions.

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
relation R » ne doit nommer aucun type de nœud en dur. Proposition : le schéma
d'un nœud déclare **lesquels de ses paramètres nomment une relation qu'il écrit**
(et lesquels nomment une relation qu'il lit), comme il déclare déjà ses ports et
ses paramètres. `RelationBatchNode` et `AddRefNode` déclarent `relation` écrite ;
`ReactTransitionNode` déclare `relation` lue. Le refus se lit alors sur le
manifeste seul : pour chaque relation lue par une réaction, au moins un outil
déclaré l'écrit — par une liaison du manifeste ou par une valeur fixe de sa fiche.
La même déclaration servira à l'atelier (montrer qui écrit quoi) et à la vision
des droits par clés.

## 6. Les marches et leur coût

*En passes, ordre de grandeur relatif.*

| marche | ce qu'elle fait | témoin | coût |
|---|---|---|---|
| **1** | `RelationBatchNode` général : garde retirée, propriétés déclarées, drain de ses seuls liens, comptage du posé ; fonction partagée d'existence et de comptage, employée aussi par `AddRefNode` | `e2e_garde_des_liens` retourné (le lien entre bouts gérés **passe par l'outil**, les garanties tiennent) ; `add_ref` vers une fiche absente se refuse en la nommant ; propriétés déclarées posées, non déclarées refusées | 1 |
| **2** | `from_states` / `to_states` dans la déclaration de relation, vérifiés au chargement et à l'écriture | un état hors liste refusé en le nommant ; un état inconnu refusé au chargement | ½ |
| **3** | `link.mmd`, et `anchor` / `supersede_link` dans le gabarit `memory` | `e2e_reaction_montee` ancre **par l'outil** ; `gabarits` charge | ½ |
| **4** | le huitième refus, sur la déclaration « relation écrite / lue » des schémas de nœud | le gabarit `memory` charge ; un manifeste à réaction sans écrivain se refuse en nommant la relation | ½ à 1 |

En tout, **deux à trois passes**. Les compteurs de liste blanche ne bougent pas
(aucun type nouveau) ; le ticket du nœud manquant se ferme à la marche 1 (« il
n'y avait pas de nœud manquant, il y avait une garde de trop ») ; celui de
l'ancre à la marche 3 ; celui du huitième refus à la marche 4 ; celui du lien
vers un bout absent à la marche 1 pour les outils, et reste ouvert pour le verbe
du catalogue tant qu'il rend `Ok` sans rien écrire.

## 7. Ce qui reste à décider

1. **Fondre plutôt qu'ajouter** : `RelationBatchNode` devient le nœud général, pas
   de type nouveau (§2). L'alternative est un `LinkNode` à côté, et la garde laissée
   en place : deux nœuds pour un geste.
2. **Le verbe du catalogue** : laisser `Catalog::link` rendre `Ok` sur un bout
   absent (les outils s'en gardent par la fonction partagée), ou le faire refuser
   lui-même — plus sûr pour tous les appelants, y compris l'indexation du code, et
   à mesurer, puisqu'elle pose des centaines de milliers de liens.
3. **Une seule garde déclarée** (`from_states`/`to_states`) ; pas de révision qui
   monte, pas de `Unlink` (§3).
4. **Le huitième refus par une déclaration dans le schéma des nœuds** (§5), plutôt
   qu'une liste de types en dur.
