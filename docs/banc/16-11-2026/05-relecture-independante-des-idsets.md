# Relecture indépendante des ensembles d'identifiants des copies froides

Porteur : `docs/arbre-principal/tickets/2026-11-16-basculer-les-trois-bases-vers-les-generations.md`,
branche Arbre cx-arbre-bascule-bases-20261116-r1, tête annoncée6dce5695e1.
Relecture Banc uniquement sur preuves existantes : aucun accès DB, HTTP,
compilation, ingestion ou service. Aucun verdict de migration finale rendu.

## Artefacts effectivement vérifiés

Racine aux postes concernés :
`~/.cache/rag3weaver-build/bascule-live-20261116/`.

- Portable : `id-sets-portable-01/proof-manifest.json`, SHA256
  `1cc2d5b1812a494d48b26cf2a63823819aee64c5deed72d6faea44cb94168ce1`,
  **34 fichiers** intégralement rehachés, tailles et SHA conformes.
- PC : `id-sets-pc-01/proof-manifest.json`, SHA256
  `903c3fcbbba0da52b3a8dcbdac40c8d189906d4af319bc42b4b4d08068bdb0c2`,
  **70 fichiers** intégralement rehachés, tailles et SHA conformes.

Les fichiers rehachés sont les preuves, jamais les bases. Les inventaires JSON
`source-before`, `source-after`, `copy-after` sont comparés comme pièces
existantes ; leur égalité ne constitue pas un nouveau hachage physique des bases.
Ils concordent pour les trois copies : respectivement3007,711,711 entrées.
Provenance annoncée par Arbre : portable snapshotG froid avant59ac sous
cold-inspection-02/copy, PC immutable-pc-01/{base} ; copies d'étude sous les
dossiers id-sets. Aucun chemin de base vivante utilisé par cette relecture.

## Recalcul depuis les sorties brutes, pas depuis les seuls verdicts

Pour chaque paire, décodage de toutes les lignes LIVE_ID/ROW, contrôle des
bornes BEGIN_QUERY/END_QUERY, du nombre de documents annoncé et des codes
processus0. Comparaison d'ensembles dans les deux sens et contrôle de doublons
sur les listes brutes. Les résultats sont ensuite confrontés aux JSON de
comparaison d'Arbre, qui concordent intégralement.

| Copie privée | Table | Identifiants FTS | Identifiants graphe | Doublons | Absents | Orphelins |
| --- | --- | ---: | ---: | ---: | ---: | ---: |
| portable | File | 8320 | 8320 | 0 | 0 | 0 |
| portable | Scope | 81347 | 81347 | 0 | 0 | 0 |
| PC principal | File | 6313 | 6313 | 0 | 0 | 0 |
| PC principal | Scope | 65913 | 65913 | 0 | 0 | 0 |
| PC lourd | File | 7275 | 7275 | 0 | 0 | 0 |
| PC lourd | Scope | 82848 | 82848 | 0 | 0 | 0 |

Les12 couples de générations correspondent entre sorties natives enregistrées,
generation-comparison.json et generation_file des inventaires FTS :

| Copie | File | Library | Scope | Symbol |
| --- | ---: | ---: | ---: | ---: |
| portable | 4526 | 142 | 5502 | 3573 |
| PC principal | 13 | 8 | 18 | 11 |
| PC lourd | 13 | 8 | 17 | 11 |

Aucune différence réelle trouvée dans les contrôles ci-dessus. Les verdicts
existants précisent live_qualified=false et maintenance_qualified=false : cette
borne est vérifiée et préservée. La mention initiale de provenance portable
LOCAL_ISA_NOT_YET_QUALIFIED ne vaut pas résultat d'exécution ; ses preuves de
fixtures locales et décodages réels postérieurs sont distinctes.

## Preuves indépendantes et portée

Script local : `~/.cache/rag3db-banc-notes/idsets-review-16nov/review.py`.
Sorties à la même racine : portable/review.json au portable, pc/review.json à
luciepc. SHA256 respectifs :

- `160fd7ba6c238db6f8ada35a0b4441ad3211fa8ed37863ffd2d4e1b89d968d3a` ;
- `e1b953b1a74982bfafc671b79bf66e530a7d69e273a58835bda946987d3e1458`.

C'est une corroboration indépendante des ensembles d'identifiants de ces
**copies froides privées**. Ce n'est ni une preuve d'état vivant, ni une preuve
de pertinence des réponses, de dette vectorielle nulle ou d'équivalence du corpus
source figé. Un futur NOP de purge exige une fermeture fraîche par G et le
snapshot rejoué correspondant. Les limites `.env.example`/YAML de l'oracle
restent NONMATCHED tant que les artefacts finaux contemporains ne les lèvent pas.
Les copies, captures02 et preuves précédentes sont conservées ; aucun nettoyage.

## Complément : métadonnées de chemins, relecture indépendante bornée

Source des relevés annoncée par Arbre : d5de903d30. Aucun nouveau relevé DB
joué par Banc. Pièces reçues et intégralement rehachées :

- portable : `vector-debt-geometry-portable-01/proof-manifest.json`,12 fichiers,
  SHA256 `b431988159eb5a306b9e90b874059ca3b3a9365ca0474400dcfbf50c097ba55a` ;
- PC : `vector-debt-geometry-pc-01/proof-manifest.json`,42 fichiers,
  SHA256 `dd5157b51fac6d4ed1877585655039ac30ca41b4f46f3841a8192f37088c2568`.

Les54 tailles et SHA concordent. Les huit requêtes de chemins ont code0,
observations=[] et sorties brutes strictes BEGIN_QUERY0/END_QUERY0. Les champs
retournés sont UUID, chemins/repo_path, langue, taille ou bornes et absent_since,
sans contenu. Inventaires enregistrés avant/après strictement identiques.

| Copie effectivement interrogée | Cible du suffixe | File | Scope |
| --- | --- | ---: | ---: |
| portable, id-sets-portable-01/copy | /extension/llm/test/.env.example | 0 | 0 |
| PC principal, fresh-sets-01/code-principal/copy | /extension/llm/test/.env.example | 0 | 0 |
| PC lourd, id-sets-pc-01/code-lourd | /extension/llm/test/.env.example | 0 | 0 |
| même copie PC lourd | /extension/lucivy/ld-lucivy/docker-compose.test.yml | 0 | 0 |

**Limite décisive** : les WHERE utilisent uniquement File.path ou Scope.file_path
ENDS WITH '/extension/…'. repo_path est retourné, pas filtré. Le chemin relatif
exact `extension/…`, sans slash initial, n'est pas testé par cette clause.
Aucune visibilité estampillée n'est qualifiée. Ces zéros sont donc vrais pour
les prédicats enregistrés, sans prouver l'absence générale du fichier ni son
absence dans une future ingestion. L'oracle reste NONMATCHED. Le paquet contient
aussi le diagnostic géométrique469 : ses empreintes sont vérifiées ici, pas sa
conclusion métier, qui reste à Arbre et au ticket fils.

Sorties indépendantes :
`~/.cache/rag3db-banc-notes/path-metadata-review-16nov/{portable,pc}/review.json`
aux postes concernés, SHA256 :
`825c57181b8e811306a7b200c8bb89bc668336d33a274b09bb6105943878cb01` et
`f401fd633bfae073f7e3d26531249f0de1b8ebd7ddbefc9b9787fc87b9c86485`.
Aucun DB/HTTP/rustc/service/contenu.env lu pour cette relecture. Toute lecture
complémentaire reste coordonnée par Arbre sur copie autorisée, sans initiative
d'ouverture de Banc et sans modifier les captures02.
