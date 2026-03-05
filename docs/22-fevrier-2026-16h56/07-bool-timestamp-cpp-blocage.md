# 07 — Blocage C++ : Boolean/Timestamp filter fields

## Ce qui est fait (Rust — tout vert, 307 tests)

- `schema.rs` : Boolean ET Timestamp retirés du skip → inclus dans filter_fields
- `search.rs` : `filter_condition: Option<FilterCondition>` ajouté à SearchOptions
- `catalog.rs` : `filter_condition` prioritaire sur `filters` HashMap

## Ce qui est fait (C++ — compile OK)

### create_lucivy_index.cpp
- `mapLogicalTypeToLucivy()` : ajout `LogicalTypeID::BOOL → "i64"`, `LogicalTypeID::TIMESTAMP → "i64"`
- `FilterFieldInfo` : ajout `LogicalTypeID originalTypeID` pour savoir comment lire la valeur
- `resolveFilterFields()` : stocke `originalTypeID`, message d'erreur mis à jour
- Bulk indexing : si `originalTypeID == BOOL` → `getValue<bool>(0) ? 1 : 0`, sinon `getValue<int64_t>(0)` (TIMESTAMP = microseconds epoch, physical type INT64 → pas de conversion nécessaire)

### lucivy_fts_test.cpp
- Nouveau test `LucivyBoolTimestampFilterTest` : table avec BOOLEAN + TIMESTAMP, filtres eq/gt/lt

## Le problème : lucivy_index.cpp (insert / update / finalize)

`lucivy_index.cpp` a 3 endroits qui lisent les valeurs des filter fields :
1. `insert()` (ligne ~220) — appelé par les hooks INSERT
2. `finalize()` (ligne ~360) — checkpoint
3. `update()` appelle `insert()` en interne → couvert

Le problème : quand `lucivyType == "i64"`, le code fait `getValue<int64_t>(pos)`. Mais si la colonne est BOOLEAN (PhysicalTypeID::BOOL, 1 byte), il faut `getValue<bool>(pos)` puis convertir en 0/1.

On doit savoir le PhysicalTypeID original pour chaque colonne indexée. Or `IndexInfo` dans rag3db (Kuzu fork) n'a **PAS** de champ `columnTypes` — seulement `columnIDs`.

## 3 options

### Option A : Ajouter `columnTypes` à `IndexInfo` (~20 lignes)

**Fichier :** `src/include/storage/index/index.h`

On maîtrise toute la stack. Ajouter :
```cpp
struct IndexInfo {
    // ... existant ...
    std::vector<column_id_t> columnIDs;
    std::vector<PhysicalTypeID> columnTypes;  // NOUVEAU
    // ...
};
```

- Modifier le constructeur de `IndexInfo` pour accepter `columnTypes`
- `create_lucivy_index.cpp` (ligne ~415) : déjà collecte les types, les passer au constructeur
- `lucivy_index.cpp` : utiliser `indexInfo.columnTypes[f]` pour BOOL check

**Impact :** IndexInfo est sérialisé/désérialisé. Il faudrait ajouter `columnTypes` à la sérialisation aussi, sinon les index existants ne pourront plus être chargés. Ou alors gérer la rétrocompat (vecteur vide = pas de types).

**+ :** Solution propre, générique, bénéficie à tout le monde
**- :** Touche au core de rag3db, sérialisation, rétrocompat

### Option B : Cache `physicalTypes_` dans LucivyIndex (~15 lignes)

**Fichier :** `lucivy_index.h` + `lucivy_index.cpp`

Ajouter un `std::vector<PhysicalTypeID> physicalTypes_` dans LucivyIndex. Le remplir dans le constructeur ET dans `load()` en relisant les types depuis le table catalog entry.

```cpp
// Dans le constructeur LucivyIndex (après les fieldIds_/fieldTypes_ existants) :
// On ne peut PAS le faire car on n'a pas le ClientContext dans le constructeur actuel.

// Dans load() :
auto tableEntry = catalog->getTableCatalogEntry(transaction, indexInfo.tableID);
for (auto& colID : indexInfo.columnIDs) {
    auto propID = tableEntry->getPropertyIDFromColumnID(colID); // si cette API existe
    auto& propType = tableEntry->getProperty(propID).getType();
    physicalTypes.push_back(propType.getPhysicalType());
}
```

Le problème : dans `load()`, on a `ClientContext*` et le catalog. Mais dans le constructeur normal (appelé depuis `create_lucivy_index.cpp`), on n'a pas le contexte. Il faut soit :
- Passer les types en paramètre du constructeur
- Résoudre dans `load()` uniquement (à la création, `create_lucivy_index.cpp` passe les types)

**Constructeur modifié :**
```cpp
LucivyIndex(IndexInfo indexInfo, std::unique_ptr<IndexStorageInfo> storageInfo,
    rust::Box<::LucivyHandle> handle, std::vector<PhysicalTypeID> physicalTypes = {});
```

`create_lucivy_index.cpp` passe les types collectés. `load()` les résout depuis le catalog.

**+ :** Pas de modif du core rag3db, auto-contenu dans l'extension
**- :** Duplique l'info types, résolution dans load() nécessite API catalog

### Option C : Encoder le type original dans le schema JSON Lucivy (~10 lignes)

**Fichier :** `create_lucivy_index.cpp` + `lucivy_index.cpp`

Stocker un hint dans le schema JSON envoyé à Lucivy :
```json
{"name":"published","type":"i64","stored":true,"indexed":true,"fast":true,"kuzu_type":"bool"}
```

Puis dans `LucivyIndex` constructeur, quand on lit les field infos depuis `get_field_ids()`, on parse aussi le `kuzu_type` hint.

Problème : `get_field_ids()` retourne `name`, `field_id`, `field_type` depuis Rust. Il n'a pas accès à un champ custom JSON. Il faudrait :
- Modifier le bridge Rust (FieldInfo struct) pour inclure un champ custom
- OU stocker le schema JSON dans LucivyIndexAuxInfo (déjà fait !) et le parser dans le constructeur C++

Le `LucivyIndexAuxInfo` stocke déjà `schemaJson`. On peut le parser dans `load()` pour extraire les `kuzu_type` hints.

**+ :** Zéro modif du core, info persistée dans le catalog
**- :** Parsing JSON dans le constructeur, un peu hacky

## Resolution : aucune des 3 options n'etait necessaire

`IndexInfo` avait deja un champ `keyDataTypes: Vec<PhysicalTypeID>` herite de Kuzu. Le "blocage" etait dans notre tete — on n'avait pas vu que le type existait deja.

**Ce qui a ete fait :**
- `create_lucivy_index.cpp` : `mapLogicalTypeToLucivy()` gere BOOL et TIMESTAMP → "i64", `originalTypeID` pour bulk indexing
- `lucivy_index.cpp` : `indexInfo.keyDataTypes[f] == PhysicalTypeID::BOOL` → `getValue<bool>() ? 1 : 0` dans insert() et finalize()
- Test E2E `LucivyBoolTimestampFilterTest` : 5 sous-tests — PASSED
- TIMESTAMP : PhysicalTypeID::INT64 → `getValue<int64_t>()` directement, filtrage gt/lt epoch us

**Status : ✅ RESOLU — 15/15 tests E2E PASSED**
