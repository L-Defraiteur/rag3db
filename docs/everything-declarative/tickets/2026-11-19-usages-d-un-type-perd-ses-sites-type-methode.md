# `usages` d'un type perd ses sites `Type::méthode(…)` depuis un changement de l'analyse

- **État** : ouvert (19 novembre 2026)
- **Famille** : produit (analyse du code)
- **Gravité** : `usages Catalog` perd 41 sites réels sur 774, tous dans `extension/rag3weaver/tests/*.rs`. Le critère de
  la répétition de la bascule est manqué par cet écart.
- **Atteignable en service** : oui. Toute réindexation d'un fichier par l'analyse d'aujourd'hui perd ces usages, avec
  ou sans versions. La bascule le révèle, parce que son premier passage réanalyse tout.
- **Touche rag3weaver** : oui (`src/code.rs`, l'analyse des rendez-vous) ; codeparsers peut-être.
- **Pour** : everything declarative
- **Signalé par** : G, à la répétition complète de la bascule (`mesures/repetition-versions-code-rag3db-depot-20261119-194739`
  au portable) ; cause lue par ED.

## Ce qu'on voit

Comparée à la production, au même arbre (b79878da9b), la copie migrée puis repassée a les mêmes usages pour
`acquireLock`, `FileSource` et `lockRowForWrite`. Pour `Catalog` : 774 sites, 733 sur la copie. Les 41 manquants
sont tous des `MENTIONS` devinées (`guessed`, sans définition, `usage: other`), venues de fonctions de
`tests/*.rs`. Par exemple `e2e_arret_brutal.rs:476` : `let mut catalog = Catalog::new(boxed, …)`.

## La cause, lue sur un reflink de la copie

Sonde `sonde_parite_search_code::releve_les_aretes_d_un_scope` (branche `rewrite-bounded-memory`).
- **Sur la copie** : `jouer_le_relecteur` a 64 arêtes, dont des `MENTIONS` vers `CatalogConfig` et `DbConnection`,
  **aucune vers `Catalog`**.
- **L'analyse d'aujourd'hui du fichier** : 44 rendez-vous en attente (`analysis.pending`) pour la fonction, **aucun
  vers `Catalog`**. `Catalog::new(…)` est retenu comme l'appel `new` qualifié par le type `Catalog`
  (`pending_qualifier_types`), comme `initialize`, `register_entity` et `execute_raw`.
- **La production** avait été indexée par une analyse plus ancienne, qui posait aussi une mention devinée du type
  `Catalog`.
- **Pourquoi `tests/` seulement** : `Catalog` y est importé du crate comme d'une bibliothèque
  (`use rag3weaver::Catalog`, `USES_LIBRARY → rag3weaver`). Le seul lien du site au type était cette mention.

## Les suspects

- `0e1074ca58` (16 nov.), « qualifier le chemin à la position exacte de sa mention » ;
- `05a0ebdfe2` (18 nov.), les mots vides de Rust et `REGLES_DE_L_ANALYSE` à 8 ;
- `bd9a9ed069` et `1055be55e9` (16 nov.), le membre d'un qualificatif de bibliothèque sûre.

## Ce qu'il faut pour le fermer

1. La bisection, avec un témoin d'analyse : `use x::Catalog; fn f() { let c = Catalog::new(); }`, puis vérifier que
   `f` a un rendez-vous vers `Catalog`. Il est rouge aujourd'hui. Le premier commit qui le fait rougir désigne la
   règle.
2. Décider avec c7 : l'analyse garde une mention du type qualifiant, ou `usages` lit `qualifier_types`. Je recommande
   la première : un `Type::méthode(…)` est un usage du type, quel que soit l'outil qui le lit.
3. Le correctif, le témoin vert, puis G rejoue la répétition (usages de `Catalog` égaux à la production).
