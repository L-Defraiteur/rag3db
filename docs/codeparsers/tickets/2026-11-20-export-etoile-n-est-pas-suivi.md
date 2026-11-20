# `export * from './x'` n'est pas suivi

- **État** : ouvert (20 novembre 2026)
- **Famille** : produit
- **Gravité** : basse : un nom réexporté par `export * from './x'` se relie par le nom (deviné), pas par l'import
- **Atteignable en service** : oui, dans un paquet TypeScript dont l'entrée réexporte tout un module
- **Touche rag3weaver** : oui (`code.rs`, les réexportations)
- **Pour** : session codeparsers
- **Signalé par** : session codeparsers, au lot des paquets importés par leur nom

## Constat

codeparsers marque `export * from './x'` (et `export * as ns from './x'`) comme une réexportation
(`tests/reexports_python_ts.rs`), mais rag3weaver ne suit que les réexportations **nommées** : elles s'attachent au
rendez-vous d'un nom (`pending_reexport`). Une réexportation large n'a pas de nom ; c7 a demandé qu'elle se suive sans
deviner un nom qui n'y est pas.

## Ce qu'il faudrait

Garder, par fichier, ses réexportations larges (le module source) ; au choix d'une cible pour un nom importé d'un
module qui désigne ce fichier, suivre aussi ces réexportations et ne retenir qu'un définisseur du nom dans le module
réexporté. Témoin rouge d'abord : `export * from './catalog'` dans l'entrée, `Catalog` défini dans `catalog.ts`,
l'arête marquée « import ».
