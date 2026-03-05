# Rapport 08 — Tests Playwright/IDBFS browser : progression en cours

## Résumé

L'infrastructure de tests browser avec Playwright est en place. Le build WASM standard (browser) est OK. Les tests échouent à cause d'un bug dans le serveur de test (query string non parsée). Fix identifié mais pas encore appliqué.

## Ce qui a été fait

### 1. Build WASM standard (browser) recompilé

Le build NODEFS avait écrasé la sortie. Rebuild forcé du standard :

```
tools/wasm/build/rag3db/rag3db_wasm.js   17MB (single-file, WASM inline)
```

Flags : `-sSINGLE_FILE=1 -lidbfs.js -lworkerfs.js -pthread -sPTHREAD_POOL_SIZE=8`

Pas de `.wasm` séparé, pas de `.worker.js` — tout est embarqué dans le .js.

### 2. Playwright installé

```json
// package.json devDependencies (ajouté)
"@playwright/test": "^1.51.0"
```

Chromium headless installé : `~/.cache/ms-playwright/chromium-1208/`

### 3. Fichiers de test créés

| Fichier | Rôle |
|---------|------|
| `test/browser/serve.js` | Serveur HTTP avec headers COOP/COEP (requis pour SharedArrayBuffer/pthreads) |
| `test/browser/index.html` | Page de test : 2 phases (create+persist, reload+verify) |
| `test/browser/idbfs.spec.js` | Spec Playwright : 2 tests E2E |
| `playwright.config.js` | Config Playwright : Chromium headless, webServer auto |

### 4. Architecture des tests

```
Phase 1 (/?phase=1) :
  - Init WASM module
  - Mount IDBFS at /database
  - Create DB at /database/mydb (lucivy indexes iront sous /database/)
  - Create table docs (id, title, body, embedding FLOAT[4])
  - Insert 4 documents
  - CREATE_LUCIVY_INDEX (title, body)
  - CREATE_VECTOR_INDEX (embedding, cosine)
  - Test contains → vérifier résultats
  - Test fuzzy → vérifier résultats
  - Test phrase → vérifier résultats
  - Test vector cosine top 3 → vérifier résultats
  - Close DB
  - syncfs(false) → sauvegarder dans IndexedDB
  - Unmount IDBFS

Phase 2 (/?phase=2) :
  - Init WASM module
  - Mount IDBFS at /database
  - syncfs(true) → charger depuis IndexedDB
  - Ouvrir DB /database/mydb
  - Re-test contains → mêmes résultats
  - Re-test fuzzy → mêmes résultats
  - Re-test vector → mêmes résultats + mêmes IDs
  - Vérifier 4 documents toujours là
  - Close + unmount
```

Le spec Playwright fait :
1. Test 1 : Ouvrir phase 1, attendre completion, vérifier résultats
2. Test 2 : Ouvrir phase 1 puis naviguer vers phase 2 (même contexte browser = même IndexedDB), vérifier persistance

### 5. Serveur HTTP avec COOP/COEP

```javascript
// Headers requis pour SharedArrayBuffer (pthreads WASM)
res.setHeader("Cross-Origin-Opener-Policy", "same-origin");
res.setHeader("Cross-Origin-Embedder-Policy", "require-corp");
```

Sans ces headers, `SharedArrayBuffer` n'est pas disponible et les pthreads WASM échouent.

## Bug identifié (non fixé)

### serve.js ne parse pas les query strings

**Symptôme** : 404 pour `/?phase=1`

**Cause** : le serveur utilise `req.url` directement comme chemin de fichier, sans séparer le pathname des query parameters.

```javascript
// Actuel (bugué) :
const url = req.url === "/" ? "/index.html" : req.url;
// req.url = "/?phase=1" → cherche un fichier "/?phase=1" → 404

// Fix à appliquer :
const parsedUrl = new URL(req.url, `http://localhost`);
const url = parsedUrl.pathname === "/" ? "/index.html" : parsedUrl.pathname;
```

**Vérification** : le serveur répond correctement pour `/index.html` (200, 7437B) et `/rag3db_wasm.js` (200, 17MB). Seules les URLs avec query string échouent.

## Pour reprendre

1. **Fixer serve.js** : parser la query string (2 lignes à changer)
2. **Relancer** : `npx playwright test --reporter=line`
3. **Si ça passe** : les 8 tests (4 en phase 1 + 4 en phase 2) valident :
   - lucivy_fts (contains, fuzzy, phrase) dans le browser
   - vector HNSW (cosine) dans le browser
   - Persistance IDBFS (create → save → reload → re-query)
4. **Si ça échoue** : possibles problèmes avec :
   - SharedArrayBuffer (COOP/COEP headers)
   - pthreads dans Chromium headless
   - IDBFS mount/sync dans le contexte de test

## Commandes utiles

```bash
# Lancer les tests Playwright
cd packages/rag3db/tools/wasm
npx playwright test --reporter=line

# Lancer avec debug visible
npx playwright test --reporter=line --headed

# Serveur seul (pour test manuel)
node test/browser/serve.js
# → http://localhost:3333/?phase=1
# → http://localhost:3333/?phase=2

# Rebuild WASM standard si nécessaire
cd packages/rag3db/build/wasm
source ~/emsdk/emsdk_env.sh
rm -f tools/wasm/build/rag3db/rag3db_wasm.*
emmake cmake --build . --target rag3db_wasm -j$(nproc)
```

## Builds WASM — attention au conflit de sortie

Les deux builds WASM (standard et NODEFS) écrivent dans le MÊME répertoire :

```
tools/wasm/build/rag3db/rag3db_wasm.js
```

Pour éviter que l'un écrase l'autre :
- Toujours supprimer les anciens fichiers avant de rebuilder (`rm -f tools/wasm/build/rag3db/rag3db_wasm.*`)
- Le standard produit : 1 fichier `.js` de 17MB (WASM inline)
- Le NODEFS produit : `.js` 239K + `.wasm` 15MB (séparés)

Pour savoir quel build est en place : vérifier la taille du `.js`. Si ~17MB = standard. Si ~240K + un `.wasm` = NODEFS.
