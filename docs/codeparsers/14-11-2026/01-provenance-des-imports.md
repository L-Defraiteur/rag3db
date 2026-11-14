# La provenance des noms importés (14 novembre 2026)

*Session codeparsers. La décision de Lucie, relayée par l'orchestration : pas de scope par nom lié ; la
provenance du nom lié se distingue sur le nœud `Library` (une par bibliothèque) et sur l'arête `liaison`
(les provenances distinctes des mentions qu'elle réunit) — `builtin`, `dependency`, `project`,
`inconnue`, jamais une devinette.*

## Deux lots

1. **La provenance à l'analyse** (codeparsers `import_resolution::provenance`) : `builtin` par des listes
   fixes par langage (std/core/alloc, les modules de Node avec et sans `node:`, les en-têtes standard C et
   C++, la BCL .NET, `sys.stdlib_module_names` de Python 3.14.9 figé ; pour Go, la règle de la chaîne
   d'outils : un premier élément de chemin sans point), `project` pour un import local, `inconnue` sinon.
   Témoin : `codeparsers/tests/provenance_imports.rs`.
2. **La provenance par les manifestes du dépôt** (rag3weaver `src/manifestes.rs`) : la synchronisation
   lit `Cargo.toml`, `Cargo.lock`, `package.json`, `pyproject.toml`, `requirements*.txt` et `vcpkg.json`
   sur la liste complète de la source, et passe à codeparsers les dépendances déclarées
   (`declared_dependencies` → `dependency`) et les paquets du dépôt (`project_packages`, les workspaces →
   `project`). Les noms se comparent par leur tête (la crate, `@scope/pkg`, le module racine), en
   minuscules, `-` et `_` confondus ; un nom que les manifestes ne disent pas reste `inconnue` (PyYAML,
   qui fournit `yaml`). Témoins : `codeparsers/tests/provenance_manifestes.rs`,
   `extension/rag3weaver/tests/e2e_code_sync.rs`
   (`les_manifestes_du_depot_disent_la_provenance_des_bibliotheques`).

## Ce que l'état d'index en dit

`/etat` d'un backend de code dit `manifests_read` (les chemins des manifestes lus à la dernière
synchronisation) et, à côté, `manifests_refresh` : **la limite admise**. Les manifestes sont relus à
chaque synchronisation de la source ; **un manifeste édité seul n'est relu qu'à la synchronisation
suivante** (la réindexation d'un fichier relit la note, pas les manifestes). Ticket :
`docs/codeparsers/tickets/2026-11-14-relire-les-manifestes-quand-l-un-d-eux-change.md`.

## Ce qui reste

- La liste des builtins en fichiers de données par langage :
  `docs/codeparsers/tickets/2026-11-14-les-listes-de-builtins-par-langage-sont-figees-dans-le-code.md`.
- Un import Python pointé ne produit aucune bibliothèque :
  `docs/codeparsers/tickets/2026-11-14-un-import-python-pointe-ne-produit-aucune-bibliotheque.md`.
- L'étiquette de la provenance dans les usages de l'IDE : un lot d'archi, plus tard.
