# 09 — Réflexion : Utiliser codeparsers depuis Rust ?

## Le problème

`@luciformresearch/codeparsers` est un package Node.js/TypeScript. On voudrait l'appeler depuis Rust (dans rag3weaver) pour avoir un pipeline 100% natif : parse → ingest → search → explore.

## Wasmer (runtime WASM embedable en Rust)

- **Wasmer v7.0.1** — runtime WASM mature (7400+ projets, 220+ contributeurs)
- Exécute des modules WASM/WASI depuis Rust à near-native speed
- Crate Rust `wasmer` — instancier un module, appeler des fonctions, passer des données
- Supporte WASI (filesystem, env vars, etc.)

**Mais** : Wasmer exécute des modules WASM purs, pas du JavaScript arbitraire.

## Pourquoi codeparsers ne peut pas tourner directement en WASM

Codeparsers dépend de :
- **tree-sitter** — addon natif C/C++ (ou WASM via tree-sitter-wasm)
- **Node.js APIs** — `fs`, `path`, `worker_threads`
- **Le runtime JS** — V8/Node.js

Ce n'est pas compilable tel quel en un module WASM.

## Options étudiées

### Option 1 : Garder codeparsers en JS (statu quo)

```
[Node.js]                          [WASM]
codeparsers.parseProject()
  → codeparsersToEntities()    →   weaver.create() × N
  → codeparsersRelationships() →   weaver.link() × M
                                    weaver.drainAsyncStart()
                                    weaver.searchAsyncStart()
```

- **Pro** : Marche déjà, codeparsers est testé et mature
- **Con** : Deux runtimes (Node.js + WASM), pas de pipeline unifié
- **Verdict** : Solution pragmatique, c'est ce que fait test-l5-full.mjs

### Option 2 : Réécrire en Rust avec le crate `tree-sitter`

tree-sitter a des **bindings Rust natifs** (`tree-sitter` crate sur crates.io). C'est le même moteur C sous le capot. On réécrirait :
- Scope extraction (parcours AST → ScopeInfo)
- Relationship resolution (CONSUMES, INHERITS_FROM, PARENT_OF...)
- UUID mapping

- **Pro** : Pipeline 100% Rust, performances maximales, pas de Node.js
- **Con** : Gros chantier, duplication de logique, maintenance double
- **Verdict** : Le graal à long terme, mais pas prioritaire maintenant

### Option 3 : Javy (JS → WASM compiler, par Shopify)

Javy compile du JavaScript en module WASM exécutable. Théoriquement on pourrait compiler codeparsers.

- **Pro** : Réutilise le code JS existant
- **Con** : Javy ne supporte pas les Node.js APIs (fs, path), pas de native addons (tree-sitter), limitations sévères sur l'écosystème
- **Verdict** : Non viable pour codeparsers

### Option 4 : IPC (Node.js process ↔ Rust)

Codeparsers tourne dans un process Node.js séparé, communique avec Rust via stdin/stdout JSON ou socket.

- **Pro** : Codeparsers tel quel, pas de réécriture
- **Con** : Overhead IPC, complexité de gestion de process, pas browser-compatible
- **Verdict** : Possible pour le natif (pas WASM), mais pas élégant

### Option 5 : tree-sitter WASM dans le browser

tree-sitter a un build WASM (`web-tree-sitter`). Codeparsers pourrait potentiellement tourner dans le browser avec tree-sitter WASM au lieu du natif.

- **Pro** : Tout dans le browser, JS pur
- **Con** : C'est déjà ce qui se passe côté client — la question est de l'intégrer côté Rust/WASM
- **Verdict** : Pertinent pour le flow browser-only (codeparsers JS → weaver WASM, tout dans le même worker)

## Conclusion provisoire

**Court terme** : Option 1 (statu quo). codeparsersToEntities/codeparsersRelationships restent en JS, appellent l'API WASM de rag3weaver pour l'ingestion. C'est le chemin de moindre résistance.

**Moyen terme** : Option 5 — faire tourner codeparsers + tree-sitter WASM dans le même worker que rag3db WASM. Tout-en-browser, pas de serveur. Le JS orchestre les deux.

**Long terme** : Option 2 — si le parsing de code devient un bottleneck ou qu'on veut un CLI Rust autonome, réécrire avec le crate tree-sitter. Mais c'est un projet en soi.
