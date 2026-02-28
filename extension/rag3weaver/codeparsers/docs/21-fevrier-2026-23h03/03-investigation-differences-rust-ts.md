# 03 - Investigation des differences Rust vs TS

## Ecarts initiaux (avant fix)

- **Scopes** : Rust 1127 vs TS 1125 (+2) — negligeable
- **Relations** : Rust 21321 vs TS 21336 (-15)
  - 25 only in Rust (dont 8 du scope supplementaire)
  - 36 only in TS

## Bugs identifies et corriges

### Bug 1 : qualifier `this`/`self` filtre comme scope inconnu

**Symptome** : `this.extractSections()` dans une classe produit `qualifier = "this"`. Comme `"this"` n'est pas dans le scope_index/scope_mapping, la ref est soit classifiee `Unknown` (dans `classify_scope_references`), soit skippee (dans `resolve_unknown_references`). Resultat : les appels intra-classe via `this.method()` ne generent aucune relation CONSUMES.

**Fix applique** (2 endroits) :

1. `src/scope_extraction/base_scope_extraction_parser.rs` — `classify_scope_references` (~L2400) :
   - `this` traite comme pas de qualifier (tous langages : JS/TS/C#/C++/Java)
   - `self` traite comme pas de qualifier uniquement pour Python/Rust (verifie via `self.language`)
   - Les refs tombent dans le cas "no qualifier" → classifiees `LocalScope` au lieu de `Unknown`

2. `src/relationship_resolution/relationship_resolver.rs` — `resolve_unknown_references` (~L370) :
   - Meme logique, mais le langage est deduit de l'extension du `file_path` (`.py`, `.rs`)
   - Les refs avec qualifier `this`/`self` ne sont plus skippees

**Pourquoi `self` est conditionne sur le langage** : en JS/TS, `const self = someObject` est un alias utilisateur courant, pas un keyword. Le traiter comme `this` donnerait des faux positifs. En Python/Rust, `self` est toujours le parametre d'instance.

**Impact** : +~1900 relations CONSUMES/CONSUMED_BY (appels `this.method()` intra et cross-classe).

### Bug 2 : regex C++/C# matchait les annotations de type TypeScript

**Symptome** : `detect_relationship_type` appliquait la regex `:\s*(public|private|protected)?\s*` (pour `class X : public Y` en C++/C#) sur **tous les fichiers**. Sur du TypeScript, cette regex matchait les annotations de type (`const x: Type`), produisant des faux INHERITS_FROM/IMPLEMENTS.

Exemples de faux positifs :
```
const globalRegistry: ParserRegistry → INHERITS_FROM (devrait etre CONSUMES)
const CPP_NODE_TYPES: NodeTypeConfig → IMPLEMENTS (devrait etre CONSUMES)
const EXTENSION_TO_LANGUAGE: SupportedLanguage → INHERITS_FROM (devrait etre CONSUMES)
```

**Fix applique** : guards de langage sur chaque bloc de `detect_relationship_type` :
- `extends`/`implements` keywords → tous langages (OK, pas de guard)
- `impl Trait for` → `.rs` uniquement
- `class X : Y` → `.cpp`, `.cc`, `.h`, `.hpp`, `.cs`, `.c` uniquement
- `class X(Y):` → `.py` uniquement
- `struct { embed }` → `.go` uniquement
- Heritage clauses → pas de guard (structurees, pas de regex)

Aussi factorise le check `sig.contains(&target.name)` en amont du bloc pour eviter de le repeter.

**Impact** : ~927 relations reclassees de IMPLEMENTS/INHERITS_FROM vers CONSUMES (correction de faux positifs).

## Ecarts finaux (apres fix)

- **Scopes** : Rust 1127 vs TS 1125 (+2) — inchange
- **Relations** : Rust 26083 vs TS 21336 (+4747)

| Type | Rust | TS | Diff | Explication |
|---|---|---|---|---|
| CONSUMED_BY | 11618 | 8783 | +2835 | Fix this + reclassement |
| CONSUMES | 11618 | 8783 | +2835 | idem |
| DEFINED_IN | 1127 | 1125 | +2 | Scopes supplementaires |
| HAS_PARENT | 749 | 748 | +1 | idem |
| IMPLEMENTS | 6 | 585 | -579 | TS a 579 faux positifs (regex C++ sur TS) |
| INHERITS_FROM | 21 | 369 | -348 | TS a 348 faux positifs (idem) |
| PARENT_OF | 749 | 748 | +1 | Scopes supplementaires |
| USES_LIBRARY | 195 | 195 | = | |

### Reste "Only in TS" (24 → expliques)

- **14 `isLocalImport`** : faux positifs TS. Le qualifier est `"ref"` ou `"importMatch"` (variables locales, pas des scopes). Le Rust les skip correctement.
- **4 `BaseLanguageParser -> initialize/parseFile`** : faux positifs TS. Ce sont des property declarations dans l'interface LanguageParser, pas des appels de methode.
- **6 directions inversees** : les faux INHERITS_FROM/CONSUMES corrigees par le fix regex. Le TS dit encore CONSUMES (par chance, car sa regex matchait aussi mais produisait CONSUMES dans certains cas).

### "Only in Rust" (~2976)

Majoritairement des relations CONSUMES legitimes via `this.method()` :
- Appels intra-classe : `parseFile -> extractSections` (meme fichier)
- Appels cross-classe : `CScopeExtractionParser -> calculateComplexity` (methode heritee de BaseScopeExtractionParser)
- References de type : `CSSAtRule -> parseAtRule` etc.

**Conclusion : le Rust est maintenant plus correct que le TS** sur tous les points. Le TS a ~927 faux IMPLEMENTS/INHERITS_FROM et manque ~1900 relations CONSUMES intra-classe.

## Fichiers modifies

### Fix this/self
- `src/scope_extraction/base_scope_extraction_parser.rs` — classify_scope_references (~L2400)
- `src/relationship_resolution/relationship_resolver.rs` — resolve_unknown_references (~L370)

### Fix detect_relationship_type
- `src/relationship_resolution/relationship_resolver.rs` — detect_relationship_type (~L725)
