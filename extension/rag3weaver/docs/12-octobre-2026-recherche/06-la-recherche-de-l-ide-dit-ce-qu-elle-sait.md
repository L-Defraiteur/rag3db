# La recherche de l'IDE dit ce qu'elle sait

*25 octobre 2026, session recherche, à la demande de l'orchestration (priorité de Lucie :
l'IDE). Forme retenue par c7 le même jour : trois pièces, trois pushes, chacune avec son
témoin rouge d'abord.*

## 1. Ce qui manque (relevé sur master `f472e75d4`)

**La page jette ce que l'hôte sait.** La vue de recherche (`src/ide/page.html`, la route
`recherche`) ne garde que `result.result`. Elle perd `presentation`, donc les avertissements
(le repli dense du 25 octobre y compris) et la ligne d'état de l'index, et `freshness`. Elle
n'affiche ni score ni extrait, seulement l'étiquette « bm25+vector ».

**L'hôte ne rend pas la structure.**
- `search_code` ne déclare pas de port `metadata`, à la différence de `recall` et des outils
  du carnet : sa `SearchMeta` (compteurs, avertissements) n'existe que réécrite en markdown.
- `FuseResultsNode` remplace le score de chaque résultat par le score fusionné : les scores bruts
  par signal, encore là pendant la fusion (`originals`), sont perdus.

**Le reclassement n'est pas branché, et sa demande est ignorée en silence.** Le relecteur
existe (`bge-reranker-v2-m3`, `POST /rerank` du démon d'embarquement, `DaemonReranker`,
`models.rerank` dans un manifeste), mais `search_workspace.mmd` n'a pas de `RerankNode`. Un
`options.rerank` passé à `search_code` grossit seulement le sur-fetch.

Les visions ne disent rien de précis de cette vue. Leurs règles suffisent : « voir sans lire,
où en est l'index » (vision générale, 11 octobre) et « la page n'a pas d'état que l'hôte n'ait
pas » (l'atelier comme IDE, 18 octobre). L'hôte rend la structure, la page l'affiche, rien
n'est relu dans du markdown.

## 2. Pièce 1 : l'hôte rend la structure

- `UnifiedResult.signals` : pour chaque signal qui a trouvé le résultat, `{label, score, rank}`
  (le score brut, le rang dans la liste du signal, 1 pour le premier). `FuseResultsNode` le
  remplit au moment où chaque identité entre dans la liste de son signal ; vide hors fusion.
  Toute recherche fusionnée en profite, pas seulement `search_code`.
- `search_code` déclare son port `metadata` (`render.meta`) : la réponse porte
  `metadata["render.meta"]`, la `SearchMeta` en JSON (compteurs, avertissements, état d'index).
  L'agent ne reçoit que `presentation` (`mcp.rs`, `texte_de`) : rien de plus en jetons.
- **Témoins** :
  - `e2e_lectures::la_recherche_rend_chaque_signal_et_sa_meta`, par `call_in` : `alpha`, trouvé
    par les deux signaux, porte le score et le rang de `bm25` et de `vector` ; la méta en JSON a
    ses compteurs ; l'index vectoriel retiré, `metadata["render.meta"].warnings` dit le repli.
    Rouge sur master : pas de champ `signals`, `metadata` vide.
  - `generic_search_nodes::tests::fuse_keeps_each_signal_score_and_rank` : les scores bruts et
    les rangs exacts sur trois résultats.

## 3. Pièce 2 : le reclassement branché, jamais ignoré en silence

`RerankNode(keep_signal=true)` entre la fusion et la pagination de `search_workspace.mmd`, actif
seulement à la demande (`options.rerank`). Sans relecteur, la demande se dit (« ordre de fusion
conservé »). Sa ligne s'ajoute à `signals`. **À la demande, jamais par défaut** tant que son gain
n'est pas mesuré sur notre code : la pièce mesure, sur des requêtes réelles de `code-lourd`,
l'ordre avant et après et la latence ; Lucie tranchera le défaut sur les chiffres. Le manifeste
de l'hôte de Lucie n'est pas touché (G le tient).

### 3.1 La mesure (25 octobre 2026, luciepc)

`tests/sonde_reclassement.rs`, sur une copie par reflink de `code-lourd` (la base du 19 octobre,
lib `04063ee06`), les gabarits de cette branche, le relecteur `bge-reranker-v2-m3` du démon de
7879 sur 20 candidats. Huit questions en langue naturelle (six en français, deux en anglais), trois
appels chacune, la médiane ; sous `poste mesure`.

| question | fusion | avec relecture | ce que la relecture change dans les cinq premiers |
|---|---|---|---|
| où un run par niveaux vérifie-t-il qu'il a été annulé | 732 ms | 1256 ms | `node.rs:cancellation` et `CancelToken` montent ; `run_level_auto` sort — mieux, en partie |
| comment la recherche se replie quand l'index vectoriel est retiré | 440 ms | 935 ms | `rebuild_detached_vector_indexes` et des tests de l'index vectoriel montent — mitigé |
| qui borne le nombre de runs lancés en fond | 656 ms | 1185 ms | la page de la commande en fond et `RunCommandNode` montent, lucivy sort — mieux |
| comment le point de reprise écrit les pages modifiées | 568 ms | 1027 ms | des tests d'ouverture en lecture seule montent — mitigé |
| où le binder refuse un index qui n'existe pas | 402 ms | 838 ms | `ensure_vector_index` et `validateIndexExistence` montent — mieux |
| comment la montre réindexe un fichier modifié | 263 ms | 739 ms | `reindex_on_change.mmd` passe premier — mieux |
| how does the hybrid search fuse bm25 and vector scores | 257 ms | 731 ms | des plans de février passent devant le gabarit hybride — moins bien |
| where is the read-only snapshot of a tool call opened | 744 ms | 1213 ms | `backend_code.rs:monte` et `ReadOnlyOpenPhase` montent — mitigé |

**Ce que disent les chiffres** : la relecture coûte de 436 à 529 ms par recherche (la latence
multipliée par 1,6 à 2,8), et change l'ordre des cinq premiers sur les huit questions. Le gain est
inégal : meilleur sur quatre, moins bon sur une, mitigé sur trois. Le jugement est le mien, sans
vérité de référence ; la base date du 19 octobre, et plusieurs bonnes réponses d'aujourd'hui n'y
sont pas. **À la demande, donc, pas par défaut** : c'est la recommandation, Lucie tranche.

## 4. Pièce 3 : la page affiche ce que l'hôte rend

Par résultat : le score, les scores par signal, l'extrait. En tête : l'état de l'index et la
fraîcheur. En pied : les avertissements, repli compris. Une case « reclasser ». Témoin : la vue
recherche de `les_vues_de_l_ide_se_rendent_dans_un_vrai_navigateur`, qui indexe sans service
(le dense y est absent) : la page montre l'avertissement et un score.
