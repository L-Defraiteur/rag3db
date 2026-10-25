# Fenêtre calme du 25 octobre 2026 : V2-1c, avant et après

Banc de concurrence, luciepc, 21 h 31 à 22 h 19, sous `poste mesure`, fenêtre donnée par
l'orchestration après le vert de l'embarquement.

- **avant** : f92b21d56 (V2-1b, pièce 2), le parent du premier commit de V2-1c. Le lot V2-1c compte
  quatre commits, 98cb797ff..66994dd43 : 66994dd43^ le contient déjà (lecture pendant le
  CHECKPOINT à 2,4 ms). Sur f92b21d56, `CheckpointNoStall.ReadDuringACheckpointDoesNotWait` est
  rouge : CHECKPOINT 1 050 ms, lecture pendant 1 020 ms.
- **après** : cbf0f9975, branche banc-hors-graphe-3 sur master 66994dd43.
- Les deux binaires bâtis dans des dossiers à part (build-avant, build-vingt), en Release.
- Script : `~/.cache/rag3weaver-build/fenetre-jeu.sh` à luciepc ; journaux dans
  `~/.cache/rag3weaver-build/jeu-vingt/fenetre-1025-2131/`.

## Relevé de charge

Avant chaque passe : charge sur 1, 5 et 15 minutes ; nombre de rustc, cc1plus et tests ; 24 cœurs.

| passe | charge | rustc / cc1plus / tests |
|---|---|---|
| pente avant 1, 2, 3 | 5,06 · 7,33 · 5,75 | 0 / 0 / 0 |
| pente après 1, 2, 3 | 4,69 · 4,73 · 4,04 | 0 / 0 / 0 |
| scénario avant | 3,82 | 0 / 0 / 0 |
| scénario après | 5,21 | 0 / 0 / 0 |
| hors du graphe | 4,04 | 0 / 0 / 0 |

Une passe de recherche (lib entière, sans bâti) tournait encore au départ ; sa fin ne se lit pas
dans ce relevé.

## 1. La pente de l'insertion sous HNSW

`VectorIndexInsertSlope.CostPerBatchSettles`, trois fois par binaire.

| | derniers / premiers (sans point de reprise) | plateau / premiers (avec) |
|---|---|---|
| avant | ×2,22 · ×2,39 · ×2,33 | ×1,41 · ×1,51 · ×1,45 |
| après | ×2,38 · ×2,23 · ×2,32 | ×1,35 · ×1,41 · ×1,44 |

Les six passes sont rouges sur `insert-cost-without-checkpoint-stays-close`, et les deux colonnes
sont confondues : V2-1c ne touche pas l'écriture. La ligne reste dans `probabilistic.txt`.

## 2. Le scénario des vingt clients

`TwentyWriters.DdlEveryMinute` avec `RAG3DB_PROFILE_CHECKPOINT=1` : 20 écrivains Doc, 2 écrivains
Note, 4 lecteurs, tampon de 2 Gio, point de reprise automatique au seuil par défaut. Vert des deux
côtés.

### Les lecteurs

| fenêtre | avant | après |
|---|---|---|
| hors DDL | n = 101 070, médiane 9,8 ms, p99 30,2 ms | n = 150 766, médiane 2,1 ms, p99 28,5 ms |
| pendant le CREATE_VECTOR_INDEX (4) | n = 829, médiane 12,8 ms | n = 8 838, médiane 0,61 ms |
| pendant le CHECKPOINT (5) | n = 7, toutes bloquées jusqu'à 1,1e6 ms | n = 2 308, médiane 0,46 ms, max 139 ms |

C'est l'effet attendu de V2-1c : une lecture ne s'arrête plus devant un point de reprise.

### Les points de reprise

| | avant | après |
|---|---|---|
| durée (n = 16 profilés) | médiane 866 ms, p90 1 326 ms, max 1 424 ms | médiane 669 ms, p90 1 146 ms, max 1 390 ms |
| pic du tampon | 1 508 Mio | 1 497 Mio |

### Trois choses à part, aucune conclusion

**(1) Les attentes jusqu'au bout du CREATE_VECTOR_INDEX, la prochaine cible.** Le CREATE dure
environ 18 min des deux côtés (1,106e6 ms avant, 1,072e6 ms après). Pendant ce temps, 40 attentes
d'écrivains Doc, 4 de Note et 8 de lecteurs commencent dans ses six premières secondes et ne
finissent qu'avec lui. V2-1c ne les touche pas : elles ne viennent pas du point de reprise mais du
rattrapage du CREATE. Ajoutées au ticket
`2026-10-19-un-create-vector-index-sous-ecritures-retient-les-autres-le-temps-de-son-rattrapage.md`,
dont la correction est (A), le CREATE concurrent.

**(2) Les validations « au seuil » : 1 062 avant, 110 après.** Chaque ligne est une validation
qui a trouvé le seuil franchi (`[commit-profile] … (au seuil)`). Il y en a 1 052 avant et 101
après où la part du point de reprise dure moins d'1 ms : la validation a trouvé le seuil franchi
mais n'a pas écrit de point de reprise. Il reste 10 vrais points de reprise avant et 9 après, et la
part totale du point de reprise dans les validations est de 9,7 s avant et 6,6 s après. L'écart
des lignes vient donc de validations qui trouvent un point de reprise demandé et pas encore fait.
Une hypothèse, non vérifiée : après V2-1c, plus aucun lecteur ne retient le point de reprise
demandé, si bien que moins de validations le trouvent en attente. Pour la vérifier, il faut dater
la demande et l'écriture de chaque point de reprise.

**(3) Les erreurs des écrivains Doc : 32 avant, 46 après, plus 1 pendant le CREATE après.**
Qualifiées : avant, 30 conflits de sérialisation (« could not serialize access due to concurrent
update ») et 2 clés primaires en double ; après, 44 conflits et 2 clés en double hors DDL, plus
1 conflit pendant le CREATE. Ce sont des conflits entre écrivains, que le scénario tolère. Un seul
passage par colonne ne dit pas si l'écart de 30 à 44 est un signal ; il n'en fait pas une ligne.

## 3. Les lignes hors du graphe : pas de mesure

`VectorIndexUpdate.MeasureSearchOverRowsOutsideTheGraph` n'a rien mesuré, et le défaut était dans
le témoin.
- Le COPY des 120 000 vecteurs de dimension 768 a rempli le tampon d'1 Gio de la fixture : le
  point de reprise a échoué et la base a été condamnée.
- Le moteur a ensuite refusé proprement le CREATE, le CHECKPOINT et le BEGIN READ ONLY.
- L'assistant `indexedRowCount` n'a pas lu le refus du BEGIN. Il a lu le catalogue sous une
  transaction nulle : SIGSEGV dans `CatalogSet::traverseVersionChainsForTransactionNoLock`, appelé
  depuis `indexedRowCount`.

Correctif sur la branche banc-hors-graphe-4 (b032cbfe4), pas encore joué :
- une fixture `VectorIndexMeasure` à 4 Gio de tampon ;
- `beginReadOnly`, qui fait dire aux trois assistants des internes un BEGIN refusé et rendre 0 ;
- l'arrêt de la mesure après un échec de la préparation.

Il sera bâti, la suite `VectorIndexUpdate` rejouée et la mesure prise à la fenêtre calme que
l'orchestration donnera le 26 octobre.
