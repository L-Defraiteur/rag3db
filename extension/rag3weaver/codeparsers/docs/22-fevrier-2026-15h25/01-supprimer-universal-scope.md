# 01 — Supprimer UniversalScope, exposer ScopeInfo directement

## Contexte

Apres le refactor body extraction (22 fev), on a constate que la conversion ScopeInfo -> UniversalScope **detruit des donnees** qu'on vient d'ajouter (body_start_line, signature_end_line) et qu'elle detruisait deja avant (members, children, enum_members, variables).

Question posee : pourquoi garder UniversalScope ?

## Analyse : ce que la conversion ajoute vs efface

### Ce qu'UniversalScope ajoute (quasi rien)

| Champ ajoute | Valeur |
|---|---|
| `uuid` | `String::new()` — **vide**, jamais genere |
| `language` | `self.language.clone()` — un enum trivial |
| `parent_uuid` | `None` — jamais resolu |

3 champs, dont 2 sont vides/None.

### Ce qu'UniversalScope efface (beaucoup)

**Perdu definitivement (aucun langage ne les mappe) :**

| Champ ScopeInfo | Utilite |
|---|---|
| `members: Option<Vec<ClassMemberInfo>>` | Membres structures d'une classe (nom, type, accessibilite, static, readonly) |
| `children: Vec<Box<ScopeInfo>>` | Scopes enfants imbriques |
| `variables: Option<Vec<VariableInfo>>` | Variables locales |
| `signature_start_line` / `signature_end_line` | Lignes de la signature (nouveau, refactor 22 fev) |
| `body_start_line` / `body_end_line` | Lignes du body (nouveau, refactor 22 fev) |
| `lines_of_code` | Nombre de lignes du scope |

**Perdu pour certains langages seulement :**

| Champ | Mappe dans lang_specific pour... | Perdu pour... |
|---|---|---|
| `enum_members` | TS seulement | Python, Rust, Go, C, C++, C# |
| `decorator_details` | TS seulement | tous les autres |
| `heritage_clauses` | TS, Rust | Python, Go, C, C++, C# |
| `ast_valid/issues/notes` | TS, Python, Rust, Go | C, C++, C# |
| `exports/dependencies` | TS, Python, Rust, Go | C, C++, C# |

### Le JSON language_specific est incoherent

| Langage | Nb champs dans lang_specific |
|---|---|
| TypeScript | 12 |
| Rust | 10 |
| Go | 9 |
| Python | 6 |
| C++ | 3 |
| C# | 3 |
| C | 2 |

Les memes donnees (modifiers, complexity, contentDedented) sont exposees pour certains langages mais pas d'autres, sans raison technique.

### La "normalisation" est triviale

Ce que fait reellement la conversion :

1. Renommer des champs : `content` -> `source`, `parent` -> `parent_name`, `scope_start_line` -> `start_line`
2. Convertir `IdentifierReference` -> `UniversalReference` (quasi identique)
3. Convertir `ImportReference` -> `UniversalImport` (quasi identique)
4. Ajouter uuid vide + language enum

C'est du code boilerplate (7 wrappers x ~80 lignes = ~560 lignes) qui ne fait que perdre des donnees.

## Decision : Option C — exposer ScopeInfo directement

### Pourquoi

- `ScopeInfo` EST deja le type universel — les 7 parsers produisent tous des `ScopeInfo`
- La conversion ne normalise rien de significatif, elle detruit
- On a 0 consommateur externe de `FileAnalysis` (code neuf, pas encore publie)
- Ajouter les champs manquants a UniversalScope reviendrait a dupliquer ScopeInfo

### Alternatives ecartees

**Option A — Supprimer UniversalScope, exposer ScopeInfo tel quel :**
Probleme : ScopeInfo n'a pas `language` ni `uuid`.

**Option B — Garder UniversalScope, y ajouter tous les champs manquants :**
On finirait avec 2 structs quasi identiques + 560 lignes de mapping boilerplate. Absurde.

### Ce qu'on fait (Option C)

1. **Ajouter 2 champs a ScopeInfo** : `language: Language` et `uuid: String`
2. **FileAnalysis.scopes** passe de `Vec<UniversalScope>` a `Vec<ScopeInfo>`
3. **Supprimer** : UniversalScope, les 7 fonctions `convert_to_universal_scope()`, les types UniversalReference / UniversalImport (ou les garder comme alias si utile)
4. **Renommer FileAnalysis** en ajustant les types

### Avant / Apres pour le consommateur

**Avant (avec perte de donnees) :**
```javascript
// Le consommateur n'a PAS acces aux members
const scope = fileAnalysis.scopes[0];
// scope.source = body text
// scope.signature = "class Foo extends Bar"
// scope.language_specific?.typescript?.members = undefined (perdu pour la plupart)
```

**Apres (zero perte) :**
```javascript
const scope = fileAnalysis.scopes[0];
// scope.content = body text
// scope.signature = "class Foo extends Bar"
// scope.members = [{name: "bar", member_type: "Property", accessibility: "Private", ...}]
// scope.children = [... scopes enfants ...]
// scope.body_start_line = 2
// scope.signature_end_line = 1
// scope.enum_members = [...]
// scope.variables = [...]
```

## Impact sur les fichiers

### Fichiers a modifier

| Fichier | Action |
|---|---|
| `src/scope_extraction/types.rs` | Ajouter `language: Language`, `uuid: String` a ScopeInfo |
| `src/base/universal_types.rs` | Supprimer UniversalScope, UniversalReference, UniversalImport. Garder FileAnalysis avec `Vec<ScopeInfo>` |
| `src/base/language_parser.rs` | Trait : `analyze_file()` retourne FileAnalysis avec ScopeInfo |
| `src/typescript/type_script_language_parser.rs` | Supprimer `convert_to_universal_scope()`, simplifier `analyze_file()` |
| `src/python/python_language_parser.rs` | idem |
| `src/rust/rust_language_parser.rs` | idem |
| `src/go/go_language_parser.rs` | idem |
| `src/cpp/cpp_language_parser.rs` | idem |
| `src/c/c_language_parser.rs` | idem |
| `src/csharp/c_sharp_language_parser.rs` | idem |
| `tests/relationships.rs` | Adapter si les tests utilisent UniversalScope |

### Fichiers NON impactes

- Les 7 scope_extraction parsers (ils produisent deja ScopeInfo)
- `relationship_resolution/` (travaille avec ScopeInfo en interne)
- CSS/Vue/Svelte/Markdown/Generic/SCSS parsers (types separes)

## Sous-question : que faire de language_specific ?

Avec ScopeInfo expose directement, **on n'a plus besoin de language_specific**. Tous les champs qui etaient fourres dans ce JSON bag (modifiers, complexity, contentDedented, genericParameters, heritageClauses, etc.) sont des champs propres de ScopeInfo.

On peut :
- Supprimer le champ `language_specific` de la sortie publique (il n'existe pas dans ScopeInfo)
- Ou ne rien faire — il n'existait que dans UniversalScope, qui disparait

## Sous-question : que faire de UniversalReference / UniversalImport ?

Ces types sont des copies quasi identiques de `IdentifierReference` / `ImportReference` (memes champs, noms differents).

Options :
- **Simple** : Garder les types internes (`IdentifierReference`, `ImportReference`) directement dans `FileAnalysis`. Supprimer les types "Universal".
- **Si compatibilite necessaire** : Type alias `pub type UniversalReference = IdentifierReference;`

Comme on n'a pas de consommateur externe, on fait le simple.

## Resume des actions

| Action | Status |
|---|---|
| Analyser ce qu'UniversalScope ajoute vs efface | FAIT |
| Decider : Option C (exposer ScopeInfo + 2 champs) | FAIT |
| Documenter la decision | FAIT (ce doc) |
| Implementer le changement | A FAIRE |

## Etapes d'implementation

1. Ajouter `language: Option<Language>` et `uuid: String` a ScopeInfo (avec Default)
2. Modifier FileAnalysis : `scopes: Vec<ScopeInfo>` (etait Vec<UniversalScope>)
3. Modifier les 7 wrappers : `analyze_file()` retourne ScopeInfo directement (supprimer convert_to_universal_scope)
4. Supprimer UniversalScope, UniversalReference, UniversalImport de universal_types.rs
5. Adapter le trait LanguageParser si necessaire
6. Verifier compilation : `cargo check`
7. Verifier tests : `cargo test --tests`
