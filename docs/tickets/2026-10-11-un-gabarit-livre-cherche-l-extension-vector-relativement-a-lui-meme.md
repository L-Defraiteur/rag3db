# Un gabarit livré cherche l'extension vector relativement à lui-même, sans repli sur le bâti

- **état** : ouvert (11 octobre 2026, banc des fiches classées)
- **gravité** : blocage (le backend ne s'ouvre pas)
- **atteignable en service** : oui, pour qui charge un gabarit sur place depuis un arbre où le moteur n'est pas bâti (un worktree, un paquet installé)
- **touche rag3weaver** : oui (et seulement)

## Ce que c'est

Les quatre gabarits livrés (`templates/backends/{code,memory,notebook,validated-result}/backend.json`)
déclarent `"vector_extension": "../../../../vector/build/libvector.rag3db_extension"`, et
`PreparedBackend::open` le résout **relativement au dossier du manifeste**
(`self.path(&self.manifest.vector_extension)`, `src/backend.rs`, avant `LOAD EXTENSION`).
Depuis un worktree, ce chemin vise l'`extension/vector/build/` **du worktree**, qui
n'existe pas : le moteur et son extension sont bâtis dans l'arbre principal et se
trouvent par `RAG3DB_ROOT` / `RAG3DB_LIBRARY_DIR`. L'ouverture tombe :

```
Binder exception: The extension …/rag3db-96/extension/rag3weaver/templates/backends/memory/../../../../vector/build/libvector.rag3db_extension
is neither an official extension, nor does the extension path: '…' exists.
```

Les tests unitaires du crate n'ont pas ce défaut : `extension_vector_de_test()` prend
`RAG3DB_ROOT`, sinon l'arbre du crate. Le produit, lui, n'a que le chemin relatif. La
recette MCP le contourne en réécrivant `vector_extension` en absolu dans la copie du
manifeste — à la main.

## Recette minimale

Charger `templates/backends/memory/backend.json` **sur place** depuis un worktree
(`PreparedBackend::load` puis `open`), `RAG3DB_ROOT` posé sur l'arbre principal :
l'ouverture refuse. Rencontré par `e2e_banc_fiches_classees` (branche
`banc-fiches-classees`, `e961e3dd8`).

## Ce qu'il faut pour le fermer

L'extension se trouve comme la lib : par le bâti déclaré (`RAG3DB_ROOT`, ou un chemin
posé au bâti), jamais par un chemin relatif au gabarit. Un `vector_extension` explicite
dans un manifeste reste un forçage ; son absence (ou une valeur réservée) veut dire
« celle du bâti », et l'échec le dit en nommant la variable à poser. Témoin : le banc
des fiches classées ouvre le gabarit `memory` sur place depuis un worktree.
