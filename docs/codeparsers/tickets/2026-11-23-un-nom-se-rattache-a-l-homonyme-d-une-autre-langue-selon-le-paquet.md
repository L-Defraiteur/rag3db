# Un nom se rattache à l'homonyme d'une autre langue, choisi selon le paquet

- **État** : ouvert (23 novembre 2026)
- **Famille** : produit
- **Gravité** : moyenne. Un usage en TypeScript se rattache (`CONSUMES`) à une classe ou une méthode homonyme d'un
  fichier Rust, et laquelle dépend des fichiers indexés dans le même paquet. `usages` et `impact` rendent alors un
  usager d'une autre langue, faux, qui change d'un index à l'autre.
- **Atteignable en service** : oui. Tout dépôt à plusieurs langues qui partagent des noms (`Search`, `call`, `route`).
- **Touche rag3weaver** : oui (la résolution par le nom ; codeparsers, le rattachement au premier homonyme)
- **Pour** : session codeparsers, avec le lot 20 des relations fabriquées (n° 9 de
  `docs/codeparsers/22-11-2026/01-ce-que-codeparsers-fabrique.md`)
- **Signalé par** : la session codeparsers (la sonde du graphe selon la taille des paquets, 23 novembre 2026)

## Constat

Même instantané de rag3db (a47ab7f5bb), même tête (le lot `batch-independence`, où la provenance d'une bibliothèque
se décide par le fichier seul), indexé en paquets de 2 048 puis de 128. Il reste une trentaine de `CONSUMES` qui
diffèrent, tous entre langues :

- `ui/ide/src/main.tsx#View:function` → `src/dataflow/search_chain.rs#Search:class` (en 2 048) au lieu de
  `ui/ide/src/views/Recherches.tsx#Search:type_alias` (en 128) ;
- `main.tsx#App:function` → `src/backend.rs#Backend.route:method` (en 2 048) ;
- `ui/ide/src/views/Editor.tsx#Editor:function` → `tests/openai_llm_retry.rs#Aucun.call:method` (en 128).

Un nom écrit en TypeScript ne désigne jamais un scope Rust.

## Ce qu'il faut

- Une résolution par le nom ne rattache qu'à un scope de la même famille de langues (TypeScript et JavaScript
  ensemble ; C et C++ ensemble ; chaque autre langue seule).
- Le choix entre homonymes ne dépend pas des fichiers du paquet (le n° 9 de la page du 22 : le premier homonyme, sans
  marque).
- Témoin : un fichier TypeScript et un fichier Rust qui définissent chacun `Search`, un usage TypeScript ; indexés en un
  paquet ou un par fichier, l'usage va au `Search` TypeScript, et jamais au Rust.
