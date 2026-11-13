# La garde du push compte un outil de mesure comme du code

- **État** : ouvert (13 novembre 2026), demandé par c7 après un push arrêté ; codeparsers le code
  (il touche `scripts/`), recherche relit.
- **Famille** : produit
- **Gravité** : une passe verte à rejouer pour rien (environ 16 minutes, et une fenêtre de master
  tenue pendant ce temps). Pas de réponse fausse : la garde refuse trop, jamais trop peu.
- **Atteignable en service** : non. C'est l'outillage du push.
- **Touche rag3weaver** : non. La garde est `push_garde.sh`, sous `~/.cache/rag3weaver-build`, et
  sa règle est écrite dans `docs/procedures/push-d-un-lot.md` (§ « Le push »).

## Ce que c'est

Avant de pousser, la garde compare la base testée par la passe à la tête de master. Si master a reçu
« du code » entre les deux, elle s'arrête : il faut rejouer la passe. Ce qui compte comme du code, ce
sont les chemins sous `extension/rag3weaver`, `tools/rust_api`, `extension/vector` et `Cargo.lock`,
hors `.md`. Le C++ du moteur (`src/`, `test/`) en est exclu, parce que la lib épinglée ne le lie pas.

Le 13 novembre à 21 h 51, le lot `lecture-pendant-un-lot` a été arrêté par
`extension/rag3weaver/scripts/oracle_lsp.py` (c76e4a482, codeparsers). C'est un outil de mesure :
aucun fichier de `src/`, de `tests/`, ni aucun script de passe ne l'appelle. Il ne touche pas ce que
la passe a jugé, et la passe a pourtant dû être rejouée.

## Ce qu'il faut

La garde doit apprendre la distinction **par la provenance**, et non par une exception tapée pour un
push :

- un fichier sous `extension/rag3weaver` qui n'est ni du Rust, ni un manifeste ou un verrou Cargo, ni
  un gabarit (`templates/`), ni appelé par un script de passe (`run_e2e.sh`, `check_features.sh`,
  `scripts/temoins_navigateur.sh` et ce qu'ils appellent) compte comme de la documentation ;
- tout le reste garde la règle d'aujourd'hui ;
- **un témoin rouge d'abord**, à jouer sans réseau dans un dépôt jetable :
  - un master qui reçoit seulement un outil Python que rien n'appelle : la garde pousse ;
  - un master qui reçoit un `.rs`, un gabarit, ou un script appelé par `run_e2e.sh` : la garde
    s'arrête, comme aujourd'hui.

c7 (13 novembre) : pas de retrait de chemin à la main, même à son mot. Une exception tapée pour un
push est la graine d'une règle floue.

## Lot

À part, après le push de `lecture-pendant-un-lot` (5bc5fc87c). La garde n'est pas dans le dépôt : il
faudra décider si elle y entre (sous `extension/rag3weaver/scripts/`, avec son témoin), ou si son
témoin vit à côté d'elle, et dire où dans la procédure.
