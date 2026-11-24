# Changer de modèle dans une conversation déjà ouverte

Ouvert le 24 novembre 2026, 14 h 45. Lucie : « chiant qu'on ne puisse pas repasser sur une ancienne session alors
qu'on change de modèle ».

## Le constat

Une tâche lit ses déclarations au commit du manifeste, au moment où elle s'ouvre. Qwen3-Coder-Next déclaré dans
`models.llm` du manifeste de `tisserande` (14 h 37) n'apparaît donc qu'aux conversations ouvertes ensuite :
`/model qwen3-coder-next` dans la conversation de midi (`--continue`) ne le trouve pas. Contournement du jour :
la ligne ajoutée à la main au manifeste figé de la tâche (`taches/8ff97cef50b6/24-11-12h07/arbre/…`, commit 7c04545).

## Ce qui est attendu

La liste des modèles est une préférence de l'utilisatrice, pas une déclaration du travail de la tâche : `/model` et
`GET /ide/teammate/models` lisent `models.llm` du manifeste **vivant** du dossier (et la surcouche, quand elle existera),
pas la copie figée de la tâche. Une conversation reprise par `--continue` voit les modèles déclarés depuis. Le reste
des déclarations de la tâche reste figé à son ouverture.

## Le témoin

Par un vrai hôte : une tâche ouverte, un modèle ajouté au manifeste du dossier et commité, puis `/ide/teammate/models`
de cette tâche le liste, et `/ide/teammate/model` le prend.
