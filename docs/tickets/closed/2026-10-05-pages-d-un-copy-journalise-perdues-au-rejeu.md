# Les pages d'un COPY journalisé sont perdues quand la base rouvre par le rejeu

- **État** : corrigé le 10 octobre 2026 (`0aed3c4b5`, « fix(stockage): le fichier de la base a une étendue connue ») — l'étendue du fichier dans l'en-tête au point de reprise, version de stockage 40, l'excédent rendu à l'espace libre à l'ouverture en écriture, sans troncature. Reclassé le 10 octobre : un défaut du moteur d'aujourd'hui, hors stèle. Limites à la fin
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

`test/transaction/journaled_copy_test.cpp`, `OwnerlessPagesTest` (neuf cas) : les trois formes de
`COPY` tuées et deux morts de suite — rouges avant le correctif (640, 638, 637, 1 269 pages
occupées pour 9) ; la lecture seule entre-temps ; le rejeu par valeur (parité avec une base
témoin, puis avec index vectoriel et plein texte) ; la base de la version 39 ; les pages rendues
réutilisées par le COPY suivant. Et les sept témoins du point de reprise interrompu
(`checkpoint_test.cpp`) font le contrôle de fuite.

### Le témoin produit, resté sans fin (arbre principal, 11 octobre 2026, nuit)

Le témoin rag3weaver (`e2e_tx_par_paquet_arret`, branche `temoin-fuite-3`) devait tuer
l'écrivain au second paquet, après que le premier (150 000 scopes, plus qu'un groupe plein) a été
validé au journal. Les essais du 10 octobre ont trouvé une base de 114 Mo pour un journal de 33 Ko,
et la question était : par quel chemin le premier paquet évite-t-il le journal ?

**Réponse : aucun. Le premier paquet n'est jamais validé à cette taille.**

- **Petit corpus** (16 fichiers de 250 fonctions, 4 000 scopes au premier paquet, sonde
  `sonde_le_premier_paquet_quitte_le_journal_par_l_index_vectoriel_releve`) :
  - le premier paquet est validé au journal : 14,6 Mo avec vecteurs, 14,4 Mo sans, à la mort de
    l'écrivain ;
  - l'index vectoriel ne tombe pas ;
  - la piste « `CREATE_VECTOR_INDEX` force un point de reprise qui vide le journal »
    (`create_hnsw_index.cpp`) est donc réfutée à cette taille.
- **Taille réelle** (600 × 250, un seul écrivain, avec vecteurs, `RAG3DB_PROFILE_JOURNAL=1`, délai
  de 25 min dont 4 de compilation) :
  - le fichier de la base atteint 114 Mo à la 8ᵉ minute, puis ne bouge plus pendant 13 minutes ;
  - le journal reste à 33 851 octets, écrits à l'ouverture ;
  - aucune ligne « [journal] transaction validée » n'apparaît : aucune transaction n'a été
    validée, forcée ou non ;
  - le délai a tué la passe.
- **Ce que veut dire 114 Mo pour 33 Ko** : c'est l'état d'un premier paquet **en cours**. Un COPY
  écrit ses groupes pleins dans le fichier avant sa validation, et le journal n'est écrit qu'à la
  validation. Les essais du 10 octobre ont lu cet état en vol, pas un journal évité.

Ce qui reste :
- Pourquoi un paquet de 600 fichiers et 150 000 scopes reste plus de 13 minutes sans écrire, après
  ses COPY : ce n'est pas lu. Les candidats sont la résolution des relations, l'embarquement, et le
  plein texte en mémoire.
- Un témoin produit ne peut pas exister sans un corpus long tant que ce paquet ne se valide pas en
  quelques minutes. Les témoins du moteur ci-dessus portent la preuve du correctif.

## Non mesuré encore (périmé)

- Le `COPY` forcé d'aujourd'hui, tué avant son point de reprise : par raisonnement il laisse les mêmes pages sans propriétaire, et le défaut actuel aurait donc déjà cette fuite sur mort. À mesurer avant de l'affirmer.
- Le `COPY` replié (`71cffbc4b`) tué avant sa validation : même question.

## Le correctif

Pas encore proposé. À lire d'abord : ce que le fichier peut porter au-delà de ce que le dernier point de reprise connaît ne se réduit peut-être pas aux pages d'un `COPY` (pages d'index, débordements de chaînes, ce qu'un point de reprise interrompu a écrit) ; comment le moteur sait ce que le dernier point de reprise connaît (l'en-tête, l'espace libre sérialisé) ; et ce que font les moteurs établis (PostgreSQL ne rend pas ces pages au rejeu, il les laisse à son ménage) — rendre à l'espace libre au point de reprise suivant suffit peut-être, plutôt que tronquer.

## Pour le fermer

Une page de conception, le rouge par un témoin du moteur (les trois formes : journalisé, forcé tué, replié tué), le correctif, la relecture du banc ; les trois cas Cypher verts sous le défaut basculé.

## Deux limites du correctif (relecture du banc, 10 octobre)

- Les pages perdues avant la version 40 ne reviennent jamais : le premier point de reprise en 40 écrit une étendue qui les englobe. Une base abîmée par l'ancien moteur se réindexe.
- Une ouverture en écriture qui rend des pages marque l'espace libre à persister : un point de reprise de fermeture écrira, sans qu'aucune requête ait écrit.
