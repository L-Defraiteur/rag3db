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
