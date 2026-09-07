# Le banc complet du moteur : le compte réel, pour la première fois

**7 septembre 2026, soir.** Tout le monde croyait ce chiffre connu ; il n'existait
nulle part. Le voici, mesuré sur `build/lecteurs-csv` — donc avec
`ESCAPED_NEWLINES` — un répertoire à la fois, en séquence, treize minutes.

## Ce qu'il a fallu faire avant que le banc dise quelque chose

Le renommage global `kuzu` → `rag3db` (`c647fbb33`) a laissé les tests derrière
lui de quatre façons, toutes silencieuses :

| forme | où | effet | corrigé |
|---|---|---|---|
| `${KUZU_ROOT_DIRECTORY}`, `${KUZU_EXPORT_DB_DIRECTORY}` | 86 fichiers, 872 occurrences | « fichier introuvable », 148 tests aveugles sur les seuls répertoires du CSV | `0d650e3c5` |
| `-DATASET KUZU` | 5 fichiers de `binary_demo/` | jeu de données jamais chargé | `70fd03564` |
| un message attendu disant encore « Kuzu » | `compressed_csv.test` | faux rouge | `0d650e3c5` |
| **des données prises pour du code** | `over-large-string/nodes.csv` et sa réponse ; `md5('kuzu')` ; `['Duckdb','Alice','kuzu',…]` | deux octets de trop, des hachages faux, un filtre sur la taille qui change | `b0b78ea49`, `1cd38487a` |

La dernière forme est la plus insidieuse : un renommage qui touche une chaîne
aléatoire ou un littéral de test ne casse rien à la compilation et rend le test
faux sans qu'aucun message ne le dise. **Si la passe de renommage est un jour
rejouée pour reprendre l'amont, les données doivent en être exclues.**

## Le tableau

Quarante-neuf répertoires, 1 861 cas exécutés. Après les corrections ci-dessus :

| | cas | verts | rouges |
|---|---:|---:|---:|
| **total** | **1 861** | **1 857** | **4** |

Les quarante-cinq répertoires non listés ci-dessous sont **entièrement verts**,
dont les plus lourds : `transaction/` 513, `tck/` 332, `dml_node/` 150,
`copy/` 118, `function/` 103, `exceptions/` 91, `ddl/` 72, `dml_rel/` 56,
`issue/` 53.

| répertoire | cas | verts | rouges | ce que sont les rouges |
|---|---:|---:|---:|---|
| `extension/` | 4 | 1 | 3 | téléchargement depuis `extension.rag3db.com`, domaine inexistant |
| `dml_node/` | 150 | 149 | 1 | une attente probabiliste, fausse neuf fois sur dix |
| `parquet/` | 0 | 0 | 0 | groupe **désactivé en amont** (`-SKIP`, export de listes fixes non pris en charge) |

### Les quatre rouges, un par un

**`extension/` — trois tests, un seul défaut, et il est dans le produit.**
`ForceInstallExtension`, `LoadNotInstalledExtension`, `UninstallExtensionError`
tentent de télécharger une extension depuis
`http://extension.rag3db.com/`. Le renommage a transformé `extension.kuzudb.com`
en un domaine qui n'existe pas (`src/include/extension/extension.h:61`,
`OFFICIAL_EXTENSION_REPO`). Ce n'est pas un faux rouge de test : **installer une
extension ne peut pas fonctionner** dans ce fork tant que cette constante
pointe dans le vide. Non corrigé ici — choisir une URL est une décision, pas
une mesure. Vela a résolu la même question par une variable d'environnement de
surcharge (`KUZU_EXTENSION_REPO`, document 06 de la mission du 3 septembre) ;
c'est une forme qui mérite d'être reprise.

**`dml_node/set/set_empty.test` — `RandUpdateInt`, un test qui a tort.**
Deux cent mille nœuds, mille mises à jour sur des identifiants tirés au hasard,
puis un compte de ceux restés intacts : l'attente est `199 000`, ce qui suppose
que les mille tirages sont tous distincts. Le résultat est `199 002` : deux
identifiants tirés deux fois. C'est le paradoxe des anniversaires — mille
tirages parmi deux cent mille produisent en moyenne 2,5 collisions, et la
probabilité qu'il n'y en ait aucune est d'environ 8 %. **Le moteur a raison, le
test se trompe**, et il se trompe identiquement sur le build du 3 septembre,
donc ce n'est pas une régression. Le fichier est identique à l'amont : le
défaut est hérité de Kuzu. À réécrire en comptant les identifiants distincts
réellement tirés, ou à ne pas garder.

**Défauts réels du moteur : zéro.** Sur 1 861 cas.

## Ce que le banc ne couvre pas

- **904 cas dans 138 fichiers portent un `-SKIP` de tête**, et ne sont jamais
  enregistrés. Parmi les 255 directives, 57 sont `-SKIP_IN_MEM`, 55
  `-SKIP_WASM`, 20 `-SKIP_NODE_GROUP_SIZE_TESTS` — conditionnelles, légitimes.
  Les autres sont des désactivations héritées de l'amont, à relire un jour
  pour savoir lesquelles ont encore une raison.
- **Rien n'a tourné en mode mémoire ni en WASM.** Le banc a été lancé une fois,
  sur disque, en natif.

## Refaire la mesure

```sh
scratch=$(mktemp -d)
for d in test/test_files/*/; do
  E2E_TEST_FILES_DIRECTORY="$d" ./build/lecteurs-csv/test/runner/e2e_test --gtest_filter='*' \
    > "$scratch/$(basename "$d").log" 2>&1
done
grep -h '^\[  FAILED  \] [a-z]' "$scratch"/*.log | sort -u
```

Le filtre `'*'` n'est pas décoratif : sans lui, le banc n'enregistre aucun
fichier et rend « 0 tests passés », qui n'est pas une ligne verte.
