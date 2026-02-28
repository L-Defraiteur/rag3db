# 03 — Couverture : Go interface embedding + TS decorators

## Ce qui a ete fait

### 1. Go interface embedding — FIX

**Probleme** : `type ReadWriter interface { Reader; Writer }` ne creait aucune relation INHERITS_FROM. Les interfaces embarquees etaient invisibles.

**Cause racine** (2 bugs combines) :

1. **AST tree-sitter-go 0.23** : les interfaces embarquees sont wrappees dans un noeud `type_elem`, pas directement `type_identifier`. L'AST :
```
interface_type
  type_elem          ← wrapper node, pas gere
    type_identifier "Reader"
  type_elem
    type_identifier "Writer"
```
Le parser cherchait `type_identifier` comme enfant direct de `interface_type` — ne trouvait rien.

2. **Filtre heritage_clauses** : le filtre exigeait `m.r#type.as_deref() == Some(&m.name)` (condition des struct embeddings ou `r#type = Some("Base")`). Mais les interfaces embarquees ont `r#type: None` → filtre ne matchait jamais.

**Fix applique** :

Fichier : `src/scope_extraction/go_scope_extraction_parser.rs`

- `extract_go_interface_methods()` : ajout gestion du noeud `type_elem` — itere ses enfants pour trouver `type_identifier` ou `qualified_type`
- Heritage clauses : filtre elargi a `m.r#type.as_deref() == Some(&m.name) || m.r#type.is_none()`

**Test ajoute** : `go_interface_embedding_is_inherits`
```go
type Reader interface { Read(p []byte) (int, error) }
type Writer interface { Write(p []byte) (int, error) }
type ReadWriter interface { Reader; Writer }
```
Asserte : ReadWriter INHERITS_FROM Reader + ReadWriter INHERITS_FROM Writer ✅

### 2. TS class decorator relations — TEST

**Constat** : les relations DECORATES/DECORATEDBY pour les decorateurs de **classe** TypeScript etaient deja implementees. Le resolver `resolve_decorator_relations()` gere `decorator_details` (TS) et `decorators` (Python). Manquait juste un test.

**Test ajoute** : `ts_decorator_relationship`
```typescript
@Injectable()
class UserService { ... }
```
Asserte : UserService DECORATEDBY Injectable ✅

**Gap identifie** : les decorateurs de **methode** (`@Log getUser()`) ne creent pas de relation DECORATES. Le `@Log` est traite comme un identifier reference (CONSUMES) au lieu d'un decorateur. C'est un gap plus profond dans l'extraction de decorateurs pour les methodes — pas traite dans cette session.

## Verification

```
cargo test --tests → 60/60 OK (etait 58 avant)
```

2 nouveaux tests :
- `go_interface_embedding_is_inherits`
- `ts_decorator_relationship`

## Fichiers modifies

| Fichier | Changement |
|---|---|
| `src/scope_extraction/go_scope_extraction_parser.rs` | `extract_go_interface_methods` : gestion `type_elem` + filtre heritage_clauses elargi |
| `tests/relationships.rs` | 2 tests ajoutes |

## Bilan couverture doc 09

| Item (doc 09 Partie A) | Status | Effort |
|---|---|---|
| Go interface embedding | **FAIT** | Fix AST + filtre |
| TS class decorator relations | **FAIT** (test) | Deja impl, test ajoute |
| TS method decorator relations | **GAP** identifie | A investiguer |
| C++ lambdas comme scopes | A FAIRE | Nouvelle extraction |
| Python comprehensions comme scopes | A FAIRE | Nouvelle extraction |
| Partie B (content body-only) | **FAIT** (session precedente) | 7 parsers + 9 tests |

## Enchainement des refactors du 22 fevrier

1. Rename ScopeInfo fields (scope_start_line, body_start_line, etc.)
2. Body extraction (content = body-only) — 7 parsers, 9 tests
3. Suppression UniversalScope — FileAnalysis.scopes = Vec<ScopeInfo> (doc 02)
4. Go interface embedding + TS decorators (ce doc)
