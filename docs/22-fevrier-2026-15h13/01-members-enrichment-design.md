# 01 — Enrichissement "Members:" des containers : qui le fait, design retenu

## Contexte

Question posee : est-ce que codeparsers met lui-meme les membres dans le `content` des classes/structs/interfaces, ou bien est-ce le consommateur (L5) qui s'en charge ?

## Reponse : c'est L5 qui le fait, pas codeparsers

### Ce que fait codeparsers (donnees brutes structurees)

codeparsers extrait et expose des donnees structurees :

```
ScopeInfo {
    content: String                        → body-only text (depuis le refactor 22 fev)
    content_dedented: String               → version dedentee
    signature: String                      → signature propre ("class Foo extends Bar")
    members: Option<Vec<ClassMemberInfo>>  → liste structuree des membres
    children: Vec<Box<ScopeInfo>>          → scopes enfants imbriques
    parent: Option<String>                 → nom du scope parent
    enum_members: Option<Vec<EnumMemberInfo>> → variantes d'enum
    variables: Option<Vec<VariableInfo>>   → variables locales
}
```

`ClassMemberInfo` contient : name, type, member_type (Property/Method/Getter/Setter/Constructor), accessibility (Public/Private/Protected), is_static, is_readonly, line, signature, value.

codeparsers ne formate RIEN pour l'embedding. Il fournit les briques.

### Ce que fait L5 (enrichissement pour le RAG)

Dans le prototype `kuzu-wasm-exp`, c'est `codeparsersToEntities()` dans `l5/code-rag/index.js` (lignes 124-184) qui fait l'enrichissement :

```javascript
// DEUXIEME PASSE — L5 reconstruit les enfants et formate pour l'embedding
const EXPLICIT_CONTAINER_TYPES = ['class', 'interface', 'enum', 'namespace', 'module', 'struct', 'trait'];

for (const scope of scopes) {
    const children = scopes.filter(s =>
        s.parentName === scope.name &&
        s.absolutePath === scope.absolutePath
    );
    if (children.length === 0) continue;

    const memberLines = [];
    for (const child of children) {
        if (child.scopeType === 'block') continue;
        const sig = child.signature || child.name;
        const lineRange = `L${child.startLine}-${child.endLine}`;
        memberLines.push(`  - ${sig} (${lineRange})`);
        // + body preview 120 chars
    }

    scope.content = `${containerSig}\n\nMembers:\n${memberLines.join('\n')}`;
}
```

L5 **remplace** le content du scope par un texte formate pour l'embedding. C'est un choix de presentation RAG, pas une extraction structurelle.

## Pourquoi c'est mieux comme ca

### 1. Separation des responsabilites

- **codeparsers** = extraction structurelle de l'AST → modele de donnees (scopes, membres, relations, types)
- **L5 / consommateur** = formatage pour un usage specifique (embedding, search, transpilation, linting)

### 2. Differents consommateurs, differents besoins

| Consommateur | Ce qu'il veut | "Members:" utile ? |
|---|---|---|
| RAG (embedding) | Texte formate pour l'embedding | Oui, texte "Members:" |
| Graph DB (Kuzu) | Relations PARENT_OF deja resolues | Non, redondant |
| Transpiler (skeleton Rust) | Membres structures (nom, type, params) | Non, il veut ClassMemberInfo |
| Linter / IDE | Types, signatures, accessibilite | Non, il veut les donnees brutes |
| Documentation gen | Signatures + docstrings | Non, format different |

### 3. Le format est arbitraire

Le "Members:" avec 120 chars de body preview, le format `L${start}-${end}`, le separateur `\n\nMembers:\n` — tout ca depend de la strategie d'embedding (token limit du modele, format attendu, etc.). Ca ne doit pas etre baked dans le parser.

## Probleme identifie : les members sont perdus

### Chaine de conversion actuelle

```
ScopeInfo (interne)  →  UniversalScope (sortie publique)
    .members ✅           .source (= content body)
    .children ✅          .signature
    .parent ✅            .parent_name ✅
    .enum_members ✅      .language_specific (JSON)
```

Quand `ScopeInfo` est converti en `UniversalScope` (dans les 7 language_parser wrappers), les **members et children sont perdus** :

```rust
// Exemple : rust_language_parser.rs, convert_to_universal_scope()
lang_specific.insert("rust".to_string(), json!({
    "modifiers": scope.modifiers,
    "complexity": scope.complexity,
    "contentDedented": scope.content_dedented,
    "genericParameters": scope.generic_parameters,
    "heritageClauses": scope.heritage_clauses,
    // PAS de members
    // PAS de children
    // PAS de enum_members (sauf dans le TS wrapper)
}));
```

Seul le wrapper TypeScript met `enumMembers` dans `lang_specific`. Aucun wrapper ne met `members` ni `children`.

### Consequence

Un consommateur qui utilise `FileAnalysis` (sortie publique avec `Vec<UniversalScope>`) n'a PAS acces aux membres structures. Il ne peut pas faire l'enrichissement "Members:" correctement — il devrait se rabattre sur les relations PARENT_OF + signatures des enfants.

## Solution proposee

### Option retenue : exposer members dans language_specific

La solution la plus simple et non-breaking : ajouter `members` et `enum_members` dans le JSON `language_specific` de chaque wrapper.

```rust
// Dans chaque language_parser wrapper
lang_specific.insert("rust".to_string(), json!({
    "modifiers": scope.modifiers,
    "complexity": scope.complexity,
    "contentDedented": scope.content_dedented,
    // ... existants ...
    "members": scope.members,        // NEW
    "enumMembers": scope.enum_members, // NEW (deja fait pour TS)
    "variables": scope.variables,     // NEW
}));
```

Avantage : pas de changement de struct `UniversalScope`, pas de breaking change dans l'API publique.

### Alternative envisagee mais non retenue : ajouter un champ members a UniversalScope

Ajouterait un champ `members: Option<Vec<...>>` a `UniversalScope`. Plus propre structurellement, mais :
- Breaking change pour les consommateurs existants
- Necessite un type generique (ClassMemberInfo est specifique a scope_extraction)
- UniversalScope est cense etre universel, pas code-specific

### Le consommateur fait ensuite ce qu'il veut

Avec les members exposes dans language_specific, le consommateur peut :

```javascript
// Exemple L5 — enrichissement container
const members = scope.language_specific?.typescript?.members || [];
for (const m of members) {
    // m.name, m.signature, m.member_type, m.accessibility, m.line
    memberLines.push(`  - ${m.signature || m.name} (L${m.line})`);
}
scope.content = `${scope.signature}\n\nMembers:\n${memberLines.join('\n')}`;
```

Ou via les relations PARENT_OF resolues, qui sont deja disponibles dans le RelationshipResolutionResult.

## Resume des actions

| Action | Status |
|---|---|
| Confirmer que codeparsers ne fait PAS l'enrichissement "Members:" | FAIT — confirme, c'est L5 |
| Confirmer que c'est le bon design | FAIT — codeparsers = donnees brutes, L5 = formatage RAG |
| Identifier la perte de donnees (members non exposes dans UniversalScope) | FAIT |
| Corriger : exposer members/enum_members dans language_specific | A FAIRE |

## Fichiers de reference

- **L5 enrichissement** : `packages/ragforge-core-exp-kuzu/kuzu-wasm-exp/l5/code-rag/index.js` (lignes 124-184)
- **L5 hooks** : `packages/ragforge-core-exp-kuzu/kuzu-wasm-exp/l5/code-rag/hooks.js` (enrichClassContent)
- **ScopeInfo struct** : `codeparsers/src/scope_extraction/types.rs`
- **UniversalScope struct** : `codeparsers/src/base/universal_types.rs`
- **Wrappers (7 fichiers)** : `codeparsers/src/{typescript,python,rust,go,cpp,c,csharp}/*_language_parser.rs`
