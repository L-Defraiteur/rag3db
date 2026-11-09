# La précision du rappel d'aujourd'hui — la ligne de base du classeur

*9 novembre 2026, session mémoire (96), à la demande de l'orchestration. Mesure seule : aucun code
produit, rien ne change chez Lucie. C'est la ligne « juge aucun » que promet la page 03 (le
classeur local) : ce que le rappel des crochets de Claude Code glisse aujourd'hui, et ce que
vaut ce qu'il glisse.*

**En une phrase : le rappel d'aujourd'hui ne glisse presque rien, et ce qu'il glisse ou pourrait
glisser n'est presque jamais pertinent. Le plafond d'un classeur est là : hors la fiche de la
session elle-même, une candidate pertinente n'apparaît que dans 4 rappels sur 104. Un rangeur ne
peut pas ranger ce qui n'est pas dans la liste ; ce qui manque est le contenu de la mémoire, pas
l'ordre.**

## 1. Le jeu

- **Une seule session porte les crochets** : celle de hooks-essai (`494dbbc6…`, du 7 au
  9 novembre). Le journal des crochets (`~/.local/state/rag3weaver/hooks.log`) n'en connaît pas
  d'autre ; les sessions ne posent pas les crochets.
- **171 rappels** (`UserPromptSubmit` avec un `recall_fiches`), hors rejeux. Chacun est apparié à
  son prompt dans la transcription de la session (à 5 s près, l'heure du journal étant locale) :
  **115 retrouvent leur prompt**, **104** ont des candidates et un verdict.
- **Ce que reçoit cette session** : presque uniquement des messages d'autres sessions (c7,
  everything declarative, archi, G) et des notifications de tâches de fond, pas des demandes
  de Lucie.
- **La mémoire** a 19 fiches en fichiers, plus les bilans du juge en base. Les candidates
  rencontrées : la fiche propre (« hooks-essai »), les fiches d'outils du juge (« outil : … »),
  les bilans du juge (« bilan : tâche réelle… ») et une seule fiche de sujet, « régime GPU ».
- **Le juge** : `gemini-3.8-flash` par Vertex (projet `lr-hub-472010`), un appel par rappel, le
  prompt (3 000 caractères au plus) et chaque candidate par son nom et son `about`, la question de
  Lucie : « ce bout de mémoire est-il pertinent avec le contexte de l'agent ? », partager un mot ne
  suffit pas. 103 appels, aucune erreur, 3 verdicts manquants sur 256 paires.

## 2. Ce qui a été glissé

| | |
|---|---|
| rappels | 171 |
| sans aucune fiche glissée | **158** (92 %) |
| dont sans aucune candidate | 13 |
| fiches glissées | 13, dont 12 jugées |
| **fiches glissées jugées pertinentes** | **1 sur 12** |
| tours avec au moins une fiche glissée pertinente | 1 sur 104 |

Les fiches glissées, par type :

| type | glissées | pertinentes |
|---|---|---|
| bilan du juge | 8 | 0 |
| outil (fiche du juge) | 2 | 0 |
| sujet (« régime GPU ») | 2 | 1 |

Les dix fiches du juge glissées l'ont été **avant le 8 novembre à 18 h 17**, quand le crochet a
appris à les écarter par leur nom (« bilan : », « outil : », faute du champ `via` dans la réponse
de `recall_fiches`). **Sous les règles d'aujourd'hui**, sur les mêmes 104 rappels, la seule fiche
qui pourrait glisser est « régime GPU », et Gemini la juge pertinente **2 fois sur 58**.

## 3. Ce que le rappel avait sous la main

Toutes les candidates, glissées ou non (256 paires jugées) :

| type de candidate | pertinentes |
|---|---|
| la fiche propre (« hooks-essai », toujours écartée) | 77 sur 89 |
| bilan du juge | 0 sur 58 |
| outil (fiche du juge) | 2 sur 51 |
| sujet | 2 sur 58 |

- **Le rappel classe bien la fiche propre** : quand elle est là, c'est elle qui est pertinente.
  Elle est écartée à dessein (la session la connaît déjà).
- **Hors elle, 4 candidates pertinentes en 104 rappels** : c'est le plafond de ce qu'un classeur
  pourrait faire glisser. Parmi les candidates que le crochet n'écarte pas, 2 sur 89.
- **Par jour** (candidates non écartées) : 1 sur 13 le 7, 1 sur 74 le 8, 0 sur 2 le 9. **Par
  tranche de quatre heures** : 0 sur 10 (0-4 h), 0 sur 31 (8-12 h), 1 sur 24 (12-16 h), 1 sur 12
  (16-20 h), 0 sur 12 (20-24 h). Une seule session : il n'y a pas de découpage par session.

## 4. Le juge contrôlé

Vingt paires tirées au sort (graine 20261109), relues à la main avec leur prompt et leur fiche :
**20 accords sur 20**, dont deux cas limites où j'aurais pu répondre l'inverse (la fiche propre
face au rapport d'un sous-agent ; la fiche d'outil `list_files` face à un message sur l'usage des
outils de code). Un cas montre la limite du rappel plutôt que du juge : un message sur le service
granite saturé de luciepc avait pour candidate « outil : report », et « régime GPU », qui aurait
été pertinente, n'était pas dans la liste.

## 5. La durée du rappel

Lue dans le journal (`rappel en N ms`), sur 158 rappels :

| p50 | p95 | max |
|---|---|---|
| 161 ms | 397 ms | 1 028 ms |

Le budget du rappel synchrone est d'une seconde ; il a été dépassé une fois.

## 6. Ce que ça dit du classeur

- **Une seconde de plus ne paierait pas aujourd'hui.** Le classeur range des candidates ; ici,
  elles ne sont pertinentes que dans 4 rappels sur 104, et le crochet écarte déjà correctement
  les fiches du juge et la fiche propre. Même parfait, il ferait glisser au plus 4 fiches utiles en
  trois jours.
- **Ce qui manque est le contenu.** Une mémoire de 19 fiches, dont une seule de sujet, ne peut
  pas répondre à des messages d'orchestration sur les passes, les pushes et les services. Le
  rattrapage prévu (l'archivage des procédures, de la page 08, des fiches de session, des visions,
  des nouveautés et des fiches de la mémoire de Claude Code dans la mémoire de Lucie) changerait
  ce chiffre ; c'est lui qui vient d'abord.
- **Le champ `via` manque à la réponse du rappel** dans la mémoire de Lucie, copiée le
  25 octobre : le crochet devine les fiches du juge à leur nom. Le gabarit livré le rend depuis le
  pas 3 de M4 (ff7a18fe2) ; c'est un cas de plus pour M8 (la montée des gabarits).
- **La mesure à refaire** : la même, après l'archivage, et sur plus d'une session (les crochets
  posés dans deux ou trois sessions de travail, pas seulement l'essai). Le classeur se juge alors
  contre ce chiffre, et contre Gemini, sur les mêmes tours.

## 7. Pour rejouer

Les scripts sont dans le dossier de travail de la session (`precision/jeu.py`, qui lit le
journal, la transcription et les fiches ; `precision/juge.py`, Gemini par Vertex, les
identifiants par le chemin de `.vault` ; les verdicts dans `precision/verdicts.json`). Ils ne
lisent rien que le poste n'ait déjà, et n'écrivent rien chez Lucie.
