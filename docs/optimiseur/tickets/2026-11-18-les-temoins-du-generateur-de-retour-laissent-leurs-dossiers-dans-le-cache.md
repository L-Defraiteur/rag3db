# Les témoins du générateur de retour laissent leurs dossiers dans `~/.cache`

- **État** : ouvert (18 novembre 2026) — à G
- **Famille** : produit (outillage : `docs/procedures/scripts/test_generated_return.py`)
- **Gravité** : encombrement. Chaque passage laisse un dossier, et rien ne les efface.
- **Atteignable en service** : non
- **Touche rag3weaver** : non
- **Pour** : G (optimiseur)
- **Signalé par** : G, à l'évaluation du travail de Codex (`docs/optimiseur/18-11-2026/01-evaluation-du-travail-de-codex.md`)

## Ce qu'on voit

Le 18 novembre : 32 dossiers `~/.cache/generated-return-integrity-*` au portable, 23 à luciepc, plus des
`generated-return-*`. Le témoin les crée par `tempfile.mkdtemp(prefix=…, dir=Path.home()/'.cache')` (lignes 35 et
83) et ne les efface pas. Le dossier est sous `~/.cache` parce que le témoin a besoin de `cp --reflink=always`, que
le tmpfs de `/tmp` refuse.

## Ce qu'il faudrait

Chaque cas efface son dossier à la fin (`addCleanup(shutil.rmtree, …)`), sauf sous une variable explicite de
conservation. Un témoin de plus vérifie qu'après un passage, aucun dossier `generated-return-*` neuf ne reste. Les
restes actuels s'effacent ensuite, en ne prenant que ceux de ce préfixe, au mot de c7.
