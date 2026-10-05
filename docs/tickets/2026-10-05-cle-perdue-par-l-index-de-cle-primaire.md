# Une clé sort de l'index de clé primaire après deux COPY de 50 000 lignes

- **État** : ouvert, côté moteur (le cœur C++ prend l'isolement). **Bloque la stèle.**
- **Gravité** : perte. Avant le filet du 5 octobre 2026, des arêtes étaient sautées en silence.
- **Atteignable en service** : oui, sur le chemin par défaut, sans annulation, sans transaction par paquet, sans panne de mémoire.
- **Touche rag3weaver** : oui (toute pose de liens par clé : COPY des liens, MERGE des liens).

## Ce que c'est

Une table de nœuds neuve reçoit deux COPY de 50 000 lignes, chacun validé. Ensuite, une de
ses clés n'est plus trouvée par l'index de clé primaire : `MATCH (n:T {_uuid: …})` rend 0,
alors que la ligne est là, avec le bon uuid, et qu'un parcours de la table la rend. Le COPY
d'arêtes qui suit est refusé : « Copy exception: Unable to find primary key value … ».

## Recette

Le corpus est fabriqué par `extension/rag3weaver/tests/e2e_tx_par_paquet_arret.rs` avec
`TX_ARRET_GROS=1` : 400 fichiers Rust de 250 fonctions chacun
(`pub fn f{i}_{j}() -> u32 { j }`), soit 100 000 scopes et 100 000 symboles. La première
indexation se fait en paquets de 200 fichiers, donc en deux paquets. La table `Symbol`
reçoit deux COPY de 50 000 lignes, puis le chargement final pose les 100 000 arêtes
`DEFINES` (Scope → Symbol) par un seul COPY.

```bash
TX_ARRET_ROLE=temoin TX_ARRET_BASE=<dossier>/base.rag3db TX_ARRET_GROS=1 \
  RAG3WEAVER_INGEST_PROFILE=1 [RAG3WEAVER_TX_PAR_PAQUET=1] \
  ./run_e2e.sh --test e2e_tx_par_paquet_arret role_enfant
```

- **Avec la transaction par paquet** : la clé perdue est le symbole `f7_144`, uuid
  `d58f2bfd-f652-752d-a54c-608329b4d6d9`. La même clé revient à chaque passe (deux passes,
  avec et sans la preuve d'existence du levier 2), donc le défaut est **déterministe**.
- **Sans elle**, sur le chemin par défaut : une autre clé, `48e7ae74-1fdc-e93a-01bb-afdf7a7ad8c1`.
- Sur la base gardée et rouverte :
  - `MATCH (n:Symbol) RETURN count(n)` rend 100 000 ;
  - tous les uuids, recalculés depuis leur clé, sont justes (0 écart, 0 doublon dans Symbol comme dans Scope) ;
  - `MATCH (n:Symbol {_uuid: 'd58f2bfd-…'}) RETURN count(n)` rend 0.
- Moteur : `37608cf4b` et `47a80373e`.

**Les bases gardées** (copier avant d'ouvrir) :
- `~/.cache/rag3weaver-build/sonde-absent/avec/`, avec la transaction par paquet (fermée sans point de reprise après le refus) ;
- `~/.cache/rag3weaver-build/sonde-absent/sans-tx/`, le chemin par défaut.

La sonde qui recalcule les uuids (`tests/sonde_uuid_absent.rs`) est jetable et n'est pas
commitée.

## Témoin

`e2e_tx_par_paquet_arret::rouge_attendu_defaut_du_moteur_un_gros_paquet_defait_se_reprend_et_son_point_de_reprise_finit`.
Il est hors de la batterie jusqu'au correctif (`TX_ARRET_ROUGE_ATTENDU=1` le joue) : il
meurt sur ce refus avant son paquet piégé.

## Ce que le filet du 5 octobre change pour l'utilisateur, d'ici le correctif

Avant, le COPY refusé se repliait sur MERGE dans la même session (`REPLI_EN_MASSE`). Ce
MERGE cherche ses bouts par la **même** clé, ne la trouve pas, et ne pose rien. L'arête
était sautée en silence, et la synchronisation finissait sur un graphe troué.

Désormais, un COPY refusé par le moteur rend la base à rouvrir
(`refus_du_copy`, ticket `2026-10-05-copy-refuse-pour-memoire-table-faussee-en-memoire`) :
**une indexation de cette taille s'arrête sur une erreur nommée**, qui porte la clé
introuvable, au lieu de finir avec un graphe troué. C'est le bon comportement : on ne
l'adoucit pas.

## Une base existante peut-elle savoir qu'elle est trouée ?

Rien n'était compté. Le chemin MERGE des liens ne lit pas les lignes rendues : une arête
dont un bout est introuvable ne produit rien, et sa référence est quand même résolue
comme réussie. La seule trace était l'avertissement du COPY refusé, au moment de la
synchronisation : `SourceSyncReport::bulk_load_refused` et les avertissements du drain
(« chargement en masse refusé … Unable to find primary key value … »). Rien n'est persisté
dans la base.

Une base peut se vérifier elle-même par des requêtes qui ne passent pas par l'index de clé
primaire, par exemple pour le code :

```cypher
MATCH (s:Scope) WHERE NOT EXISTS { MATCH (s)-[:DEFINES]->(:Symbol) } RETURN count(s)
```

Chaque scope définit son symbole : un compte non nul signale des arêtes manquantes. Les
autres relations n'ont pas d'invariant aussi simple.

## Pour le fermer

Le correctif du moteur, puis le témoin rouge attendu vert et remis dans la batterie.
