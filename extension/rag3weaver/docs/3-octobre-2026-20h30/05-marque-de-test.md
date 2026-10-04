# Savoir qu'un scope est un test

3 octobre 2026, codeparsers `137b9d2`. La pièce qui manque à la première
vision du produit code (`extension/rag3weaver/visions/2026-10-03-20h37-le-produit-code.md`, idée 1 : « 23
appelants, 4 tests la traversent », puis « quels tests rejouer après cette
édition »).

## Ce que sort codeparsers

`ScopeInfo.test : Option<TestMark>`, posé en fin d'analyse sur ce que chaque
scope porte déjà ; aucun scope n'est créé ni retiré.

- `role` : `case` (un test, ce qu'un lanceur exécute), `suite` (ce qui en
  regroupe), `support` (du code qui n'existe que pour les tests) ;
- `certainty` : `certain` quand la syntaxe ou l'outil le dit, `convention`
  quand seul le nom le dit ;
- `marker` : ce qui l'a dit (`#[test]`, `#[cfg(test)]`, `unittest.TestCase`,
  `test_*`, `TEST_F`, `*testing.T`…) ;
- `name` : le nom du test quand ce n'est pas celui du scope (`CalcTest.Adds`
  pour `TEST(CalcTest, Adds)`, dont le scope s'appelle `TEST`).

## Sûr ou convention, par langage

| Langage | Sûr | Convention |
|---|---|---|
| Rust | `#[test]`, `#[tokio::test]` et tout `…::test`, `#[rstest]`, `#[test_case]` ; un `mod` sous `#[cfg(test)]` et ce qu'il contient ; un fichier sous `tests/` | — |
| Python | `@pytest.fixture` ; une sous-classe de `…TestCase` (unittest) et ses méthodes `test*` | pytest : `test_*` et `Test*` dans un fichier `test_*.py` / `*_test.py`, le reste de ce fichier, `conftest.py` (les motifs de pytest se règlent : c'est une convention) |
| C++ | gtest `TEST`, `TEST_F`, `TEST_P`, `TYPED_TEST`, `TYPED_TEST_P` ; Catch2 `TEST_CASE` | — |
| Go | `TestX(t *testing.T)`, `BenchmarkX(b *testing.B)`, `FuzzX(f *testing.F)`, `ExampleX()` dans un `_test.go` ; le reste de ce fichier (la chaîne d'outils l'exclut des builds) | — |
| TS / JS | `describe`, `context`, `suite` (suites), `it`, `test`, `specify` (cas), et leurs `.only` / `.skip`, appelés avec un titre littéral et une fonction ; chacun devient un scope nommé par son titre, avec son chemin (`calc > adds`) — codeparsers `594fc61`, pointeur séparé puisqu'il crée des scopes | — |

Une fonction nommée `test_x` hors d'un fichier de test, ou `TestLike` hors
d'un `_test.go`, n'est pas marquée : le nom seul ne suffit jamais hors de
la convention de l'outil.

## Mesure

| Corpus | Marques |
|---|---|
| `src/dataflow` (Rust) | 350 cas — autant que d'attributs de test dans la source —, 33 suites, 281 supports |
| 40 fichiers gtest de `test/` du moteur | 336 cas, tous les `TEST` / `TEST_F` lus, chacun avec son nom gtest |
| 21 fichiers de test JS (`tools/nodejs_api/test`, `tools/wasm/test`) | 148 cas pour 148 `it(` / `test(` dans la source, 81 suites |
| 11 scripts `scripts/test_*.py` | 2 suites, 5 cas et 3 supports unittest (sûrs) ; 42 fonctions en support par convention (ces scripts ne suivent pas pytest) |

Cela suppose les fonctions de module Rust devenues des scopes
(`1f9d653`, `04-fonctions-de-module-rust.md`) : sans elles, les 348 tests
de `src/dataflow` n'existaient pas comme scopes.

## Proposition pour rag3weaver (à la session de l'arbre principal)

Faite et acceptée : branche `codeparsers-champs-test` (`b6e7db858`), en
attente de reprise après les pointeurs 4 et 5. Ce qu'elle contient :

Deux champs ordinaires de l'entité `Scope`, rien d'orienté test dans le
moteur :

- `test_role` (`STRING`, vide pour un scope qui n'est pas de test) ;
- `test_certainty` (`STRING`).

Et le nom gtest (`TestMark.name`) dans un troisième, `test_name`, ou dans le
texte du scope. Ce qu'ils permettent sans rien coder : peser moins les tests
dans la recherche par `FieldWeightNode` (valeur de champ), filtrer, et pour
l'idée 1, compter les tests parmi les appelants (`CONSUMED_BY` vers un
`Scope` dont `test_role = 'case'`).
