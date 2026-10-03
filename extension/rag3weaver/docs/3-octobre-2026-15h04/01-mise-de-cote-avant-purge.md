# La mise de côté avant purge, et l'annulation en bloc d'une fin

**3 octobre 2026.** Page de conception de la seconde étape de la
synchronisation par périmètre, décidée par Lucie : `keepFor`, 7 jours par
défaut, réglable par entité, `0` pour la couper ; l'annulation en bloc d'une
fin vient avec. Rien n'est codé à l'écriture de cette page.

## 1. Ce que fait la mise de côté

Une fin de synchronisation qui **retire** des lignes (`onMissing: delete`) les
retire vraiment, par le chemin de suppression habituel : rien ne reste dans
les tables vivantes ni dans les index, donc aucune ligne mise de côté ne peut
ressortir dans une recherche, un `get` ou une relation suivie. Avant de les
retirer, elle en **copie** l'essentiel dans une table interne. Si une ligne
reparaît dans un lot avec la **même identité et le même `_content_hash`**, elle
est rendue depuis la copie, **sans redécoupage ni réembarquement** : seul
l'index plein texte est repayé, et c'est la partie bon marché.

Une transition d'absence (`onMissing: {transition}`) ne retire rien : la ligne
reste en place, la mise de côté ne la concerne pas.

## 2. La table interne

Une table par entité, `_Aside_<Entité>` (nom à confirmer), créée avec l'entité
quand elle déclare `snapshot` :

| colonne | contenu |
|---|---|
| `_uuid` | l'identité de la ligne retirée (clé) |
| `_content_hash` | le hash de contenu au moment du retrait — la condition de retour |
| `row` | la ligne entière, champs déclarés et colonnes internes, en JSON |
| `chunks` | les chunks : texte, bornes, et les vecteurs denses par modèle (`embedding__<slug>`), en blob |
| `session` | la session dont la fin l'a retirée |
| `removed_at` | millisecondes du retrait — la purge s'en sert |

**Ce qu'on ne copie pas encore** : les vecteurs **creux**. Ils ne vivent que
dans l'index sparse (lucistore), pas dans les lignes de chunks. Deux voies,
à trancher en codant : les relire dans l'index avant le retrait si le handle
sait les rendre par identifiant, ou les recalculer au retour (le creux est
l'embarquement le moins cher). La première est préférable ; la seconde est le
repli.

**Ni les relations** : `DETACH DELETE` les emporte, et une relation dépend de
l'autre extrémité, qui a pu changer. Une ligne rendue revient sans ses
relations ; c'est la synchronisation des relations (`link_*`) qui les repose.
À dire dans le rapport de retour.

## 3. Le retour

Au marquage d'un lot (`mark_snapshot`), pour les uuids du lot **absents de la
table vivante avant l'écriture** et présents dans la mise de côté : si le
`_content_hash` de la ligne entrante est égal à celui de la copie, on rend la
ligne depuis la copie (ligne, chunks, vecteurs) au lieu de l'ingérer
normalement, puis on retire la copie. S'il diffère, la copie est périmée : on
la retire et la ligne s'ingère normalement.

Question de place dans le chemin : l'ingestion a lieu avant le marquage. Le
retour doit donc être décidé **avant** `ingest_entities` — dans le lot
(`EntityBatchNode`), qui connaît déjà ses uuids. C'est le point le plus
délicat : il touche le chemin d'ingestion, chemin de masse compris.

## 4. La purge, une dette bornée

Les copies plus vieilles que `keepFor` sont des dettes, comme les autres
rattrapages (`rattraper_le_decoupage`, les dérivées à re-rendre) : une passe
bornée, à chaque drain ou à chaque fin, en retire un nombre limité, sans
jamais bloquer une écriture. `keepFor: 0` ne copie rien.

**La réserve rag3db** : une ligne supprimée n'y est jamais récupérée (la place
reste perdue jusqu'à une correction du moteur ; un `SET` rend la sienne). Une
mise de côté purgée par `DELETE` ferait donc grossir la base de tout ce qui
passe par la corbeille, une seconde fois. Deux voies :

- **purger par `SET`** : vider le blob et la ligne JSON (`chunks = ''`,
  `row = ''`) plutôt que supprimer la ligne, comme `cypher_blob_store` le fait
  pour les blobs d'index ; la ligne vide reste, minuscule, et resservira à la
  prochaine mise de côté du même uuid ;
- **attendre la récupération au checkpoint dans le moteur** (proposée, pas
  faite).

Je propose la première tant que la seconde n'existe pas.

## 5. L'annulation en bloc d'une fin

Avec la mise de côté, une fin appliquée devient réversible : ses retraits sont
dans la mise de côté, marqués de sa session ; ses transitions ont changé un
champ que la marque d'absence (`_absent_since`) désigne. `undo_snapshot_finish
(entité, périmètre, session)` :

- rend chaque ligne mise de côté par cette session (comme un retour, sans
  condition de hash) ;
- pour les transitions de cette session, il faut l'état d'avant : la fin doit
  donc le noter (dans le rapport persisté, ou dans la mise de côté sous une
  forme légère : uuid, champ, état d'avant). À trancher : je propose de
  persister le rapport de chaque fin appliquée (une ligne interne par
  session), ce qui sert aussi d'historique.

L'annulation n'est possible que tant que les copies n'ont pas été purgées : au
plus `keepFor`. Le refus le dit.

## 6. Ce qui reste à trancher en codant, sans Lucie

- le nom de la table (anglais : `_Aside_<Entité>` ou `_<Entité>_Aside`) ;
- les vecteurs creux : relus dans l'index ou recalculés ;
- la persistance du rapport de fin (pour l'annulation des transitions).

## 7. Les tests, d'abord sur l'entité synthétique

1. une ligne retirée puis reparue identique revient sans réembarquement (le
   compteur d'embarquement ne bouge pas) et ressort dans la recherche ;
2. reparue modifiée : la copie est jetée, la ligne s'ingère ;
3. `keepFor: 0` : rien n'est copié ;
4. la purge retire les copies plus vieilles que `keepFor`, et pas les autres ;
5. l'annulation en bloc rend les lignes retirées et les états d'avant ;
6. une ligne mise de côté ne sort dans aucune recherche (plein texte, dense,
   `get`).

## 8. Ce qui est codé (3 octobre, soir), et où le code s'écarte de cette page

Les trois points du §6, tranchés :

- **la table** : une seule, `_snapshot_aside`, pour toutes les entités,
  clé `{entité}:{uuid}`, posée à chaque ouverture (`CREATE … IF NOT
  EXISTS`) — une base d'avant la reçoit sans migration ; colonnes `_entity`,
  `_uuid`, `_content_hash`, `_row` (JSON), `_chunks` (JSON), `_session`,
  `_removed_at` ;
- **les vecteurs creux** : recalculés au retour. Ils ne se relisent pas
  dans l'index (le handle n'a pas de `get`), et ils sont le signal le moins
  cher ;
- **le rapport de fin** : persisté dans `_catalog_meta`
  (`snapshot_finish:{org}/{project}:{entité}:{session}`), avec l'état
  d'avant de chaque transitionnée (`previousStates`) et une marque
  d'annulation.

**Écart au §3 : la ligne ne revient pas « depuis la copie », elle s'ingère
par le chemin de toujours, et seuls ses vecteurs reviennent.** Le
découpage est déterministe et bon marché ; l'embarquement est le coût.
`take_from_aside`, avant le graphe d'`ingest_entities`, rend à
`EmbedNode` (service `known_vectors`) les vecteurs denses mis de côté,
par uuid de chunk et `_text_hash` : un chunk dont le texte est le même
n'est pas réembarqué, sur le chemin de masse (`Enrich`) comme sur celui de
toujours. Avantages : la ligne qui revient porte les champs non contenus
du lot qui la ramène (pas ceux de la copie), et aucune ligne n'est écrite
hors du chemin habituel. La copie est vidée une fois l'écriture faite —
une ingestion qui échoue ne la perd pas. `FlushResult.restored` et le
rapport du lot (`restored`) nomment les lignes rendues.

Ce qui reste comme dit :

- la copie est faite avant toute mise en file des suppressions ; si elle
  échoue, rien n'est retiré ; une suppression qui échoue vide sa copie ;
- la purge est bornée (512 copies par passe), **par SET** (`_row = ''`,
  `_chunks = ''`), à chaque fin ; `purge_snapshot_aside_at` prend l'instant
  en paramètre pour les tests ;
- `undo_snapshot_finish` (outil `undo_snapshot`) réingère les retirées
  depuis leur copie — donc sans réembarquement — et rend leur état d'avant
  aux transitionnées dont l'état n'a pas bougé depuis la fin ; une fin
  s'annule une fois ; refusée, en le disant, si ses copies sont purgées.

Limites nommées :

- **PostgreSQL** : la relecture d'une colonne `vector` en liste de
  flottants n'est pas éprouvée ; la copie y part sans vecteurs, la ligne
  revient réembarquée. Le reste des requêtes a sa forme PostgreSQL, non
  jouée (pas de serveur sur ce poste) ;
- seul le **modèle courant** est copié ; une ligne revenue sous un autre
  modèle est réembarquée pour lui ;
- l'état d'avant d'une transitionnée est écrit **sans la garde** de la
  machine à états (une annulation n'est pas une transition), et le document
  plein texte de la ligne garde l'état transitionné jusqu'à sa prochaine
  écriture ;
- une ligne rendue revient **sans ses relations** ;
- un retour par la file (`create`, `drain`) ne consulte pas la mise de
  côté ; seule `ingest_entities` le fait.

**Trouvé en codant** : la suppression ne retirait jamais le creux de
l'index lucistore (`DeleteRecordNode`, et `RechunkDeleteNode` à chaque mise
à jour qui redécoupe) ; corrigé sur la même branche
(`retirer_le_creux_des_chunks`). Et la recherche dense ne répondait plus
après une table vidée puis repeuplée — un défaut du moteur, corrigé par
la session cœur C++ (`e2e_recherche_dense_apres_suppressions`).

Tests : `tests/e2e_synchronisation.rs` (le retour sans réembarquement, par
les deux chemins ; la copie périmée jetée ; `keepFor: 0` ; la purge ;
l'annulation des retraits et des transitions ; aucune recherche ne sort une
ligne mise de côté ; le creux), `scripts/test_backend_snapshot.py`
(l'annulation et le retour par le backend).
