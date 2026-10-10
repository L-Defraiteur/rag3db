# Les pages d'un COPY journalisé sont perdues quand la base rouvre par le rejeu

- **État** : ouvert — reclassé le 10 octobre 2026 par l'orchestration : **un défaut du moteur d'aujourd'hui**, pas seulement du `COPY` journalisé ; ne perd ni ne corrompt (de l'espace, qui s'accumule), donc hors de la condition 1 de la stèle ; corrigé maintenant parce que le chargement final des relations de rag3weaver (1,1 million de lignes en une transaction) est dans la zone à chaque arrêt brutal. Correctif en cours (page 06)
- **Gravité** : espace perdu dans le fichier de la base, pour toujours, qui s'accumule à chaque mort. Aucune donnée perdue, aucune réponse fausse
- **Atteignable en service** : oui, aujourd'hui : toute mort du processus pendant un `COPY` d'au moins un groupe plein de nœuds (131 072 lignes), journalisé ou non ; et après un `COPY` journalisé validé, toute réouverture par le rejeu
- **Touche rag3weaver** : oui — le chargement final des relations d'une première indexation (1,1 million de lignes) ; les paquets de 2 048 fichiers (~30 000 scopes) sont en deçà du groupe plein
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

## Mesuré le 10 octobre, sans harnais

Un `COPY` de 200 000 lignes (`id INT64`, `name STRING`), le fils meurt base ouverte, la base
est rouverte, toutes les tables retirées, un point de reprise, puis pages du fichier moins pages
libres. La même scène validée puis fermée après un point de reprise : 9 pages occupées.

| Scène | Pages occupées |
|---|---|
| `COPY` forcé (le défaut d'aujourd'hui), tué avant sa validation | 559 |
| `COPY` replié (seuil 64 Ko), tué avant sa validation | 560 |
| `COPY` journalisé validé, tué, rejoué | 559 |
| `COPY` forcé validé, tué après son point de reprise | 9 |
| `COPY` forcé de 300 000 lignes, tué avant sa validation | 1 129 |
| deux morts de suite sans point de reprise (témoin, 200 000 lignes chacune) | 1 269 — la fuite s'additionne |

**Le seuil du groupe plein.** Un `COPY` n'écrit ses pages avant sa validation que par groupe
plein de nœuds (131 072 lignes) ; en deçà, tout reste en mémoire jusqu'au point de reprise, et
rien ne fuit. Un `COPY` de 3 000 lignes, journalisé, fermé sans point de reprise puis rouvert :
4 pages occupées, aucune fuite. Les témoins sont donc à 200 000 lignes.

## Témoins

`test/transaction/journaled_copy_test.cpp`, `OwnerlessPagesTest` : les trois formes de `COPY`
tuées, et deux morts de suite — rouges avant le correctif (640, 638, 637, 1 269 pages occupées
pour 9).

## Non mesuré encore (périmé)

- Le `COPY` forcé d'aujourd'hui, tué avant son point de reprise : par raisonnement il laisse les mêmes pages sans propriétaire, et le défaut actuel aurait donc déjà cette fuite sur mort. À mesurer avant de l'affirmer.
- Le `COPY` replié (`71cffbc4b`) tué avant sa validation : même question.

## Le correctif

Pas encore proposé. À lire d'abord : ce que le fichier peut porter au-delà de ce que le dernier point de reprise connaît ne se réduit peut-être pas aux pages d'un `COPY` (pages d'index, débordements de chaînes, ce qu'un point de reprise interrompu a écrit) ; comment le moteur sait ce que le dernier point de reprise connaît (l'en-tête, l'espace libre sérialisé) ; et ce que font les moteurs établis (PostgreSQL ne rend pas ces pages au rejeu, il les laisse à son ménage) — rendre à l'espace libre au point de reprise suivant suffit peut-être, plutôt que tronquer.

## Pour le fermer

Une page de conception, le rouge par un témoin du moteur (les trois formes : journalisé, forcé tué, replié tué), le correctif, la relecture du banc ; les trois cas Cypher verts sous le défaut basculé.
