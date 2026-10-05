# Les pages d'un COPY journalisé sont perdues quand la base rouvre par le rejeu

- **État** : ouvert — **bloque le basculement** du `COPY` journalisé en défaut (condition posée par l'orchestration ; 5 octobre 2026). Pas de correctif encore : une page de conception d'abord
- **Gravité** : espace perdu dans le fichier de la base, pour toujours, qui s'accumule. Aucune donnée perdue, aucune réponse fausse
- **Atteignable en service** : aujourd'hui seulement avec `CALL force_checkpoint_on_copy=false` ; après le basculement, à chaque arrêt sans point de reprise qui suit un `COPY` journalisé (mort du processus, ou fermeture avec `force_checkpoint_on_close=false`)
- **Touche rag3weaver** : oui après le basculement — une indexation tuée entre deux points de reprise
- **Ouvert le** : 5 octobre 2026, session cœur C++ (la liste complète jouée à blanc avec le défaut basculé)
- **Pour** : cœur C++ (reprise, gestion de l'espace)

## Ce que c'est

Un `COPY` écrit ses pages dans le fichier de la base pendant qu'il s'exécute. Journalisé, il n'est suivi d'aucun point de reprise : ses lignes sont au journal. Si la base est rouverte par le rejeu, les lignes sont réinsérées — dans d'autres pages. Celles que le `COPY` avait écrites ne sont plus à personne : ni une table ne les désigne, ni l'espace libre ne les connaît.

## Mesuré

La liste complète, défaut basculé dans l'arbre (non commité), moteur à `71cffbc4b`. Le contrôle de fuite de la suite Cypher (`test/test_runner/fsm_leak_checker.cpp:116`) retire toutes les tables, fait un point de reprise et compte les pages encore occupées, qui doivent être celles de l'en-tête, du catalogue et des métadonnées :

| Cas | Pages occupées | Attendues |
|---|---|---|
| `ddl~ddl.AddNodePropertyRecovery` | 403 | 11 |
| `ddl~ddl.AddRelPropertyRecovery` | 949 | 11 |
| `ddl~ddl.DropTablesReloadWithoutForceCheckpoint` | 232 | 5 |

La forme commune : un `COPY` (dans le cas ou dans le chargement du jeu de données), pas de point de reprise, `RELOADDB`. Toutes les pages du `COPY` fuient.

## Non mesuré encore

- Le `COPY` forcé d'aujourd'hui, tué avant son point de reprise : par raisonnement il laisse les mêmes pages sans propriétaire, et le défaut actuel aurait donc déjà cette fuite sur mort. À mesurer avant de l'affirmer.
- Le `COPY` replié (`71cffbc4b`) tué avant sa validation : même question.

## Le correctif

Pas encore proposé. À lire d'abord : ce que le fichier peut porter au-delà de ce que le dernier point de reprise connaît ne se réduit peut-être pas aux pages d'un `COPY` (pages d'index, débordements de chaînes, ce qu'un point de reprise interrompu a écrit) ; comment le moteur sait ce que le dernier point de reprise connaît (l'en-tête, l'espace libre sérialisé) ; et ce que font les moteurs établis (PostgreSQL ne rend pas ces pages au rejeu, il les laisse à son ménage) — rendre à l'espace libre au point de reprise suivant suffit peut-être, plutôt que tronquer.

## Pour le fermer

Une page de conception, le rouge par un témoin du moteur (les trois formes : journalisé, forcé tué, replié tué), le correctif, la relecture du banc ; les trois cas Cypher verts sous le défaut basculé.
