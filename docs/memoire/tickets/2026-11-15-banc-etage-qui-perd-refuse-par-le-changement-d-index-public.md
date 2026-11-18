# `banc_etage_qui_perd` tombe à son installation : changement d'index public refusé

- **État** : ouvert, le correctif est en branche depuis le 18 novembre 2026 (`cx-memoire-banc-etage`, 65960ae7a), vert sur la lib 3d706f709 ; il se ferme à son atterrissage. **Le diagnostic d'origine était faux**, voir « Diagnostic corrigé ».
- **Famille** : produit. Le défaut est dans un banc e2e du crate, pas dans le catalogue : c'est la
  règle du catalogue qui a raison, et le banc qui ne la suit pas.
- **Gravité** : un banc de la batterie est rouge sur master à chaque passe qui le joue. Il est rouge
  depuis le 11 novembre 2026 au plus tard.
- **Atteignable en service** : non. Le banc réenregistre lui-même une entité peuplée avec d'autres
  signaux.
- **Touche rag3weaver** : oui, `tests/e2e_banc_etage.rs` (`banc_etage_qui_perd`, l. 411 ; le
  réenregistrement l. 795-801).
- **Pour** : la session qui tient les bancs d'étage (banc), ou la session mémoire (96) à son mot.
- **Signalé par** : la session mémoire (96), dans la passe du lot `usage` du 15 novembre 2026.

## Ce qu'on voit

```
e2e_banc_etage::banc_etage_qui_perd ... FAILED
Scope au titre pesé: SchemaError("Scope : changement d'index public interdit sur une entité peuplée sans migration déclarée")
```

Le banc indexe un dépôt, puis, pour chaque réglage essayé, réenregistre `Scope` avec un autre
`search_index` : les signaux `HYBRID | SPARSE`, et un `boost` sur `name`. Il le fait sur l'entité
déjà peuplée (`cat.register_entity(SCOPE, config).expect("Scope au titre pesé")`, l. 801).

## Pourquoi

8a931f78d (11 novembre 2026, « Garder la représentation publique et le rattrapage dans leur
cellule ») a ajouté au catalogue (`src/catalog.rs`, vers la l. 2154) un refus : un `search_index`
changé sur une entité qui a des lignes exige une migration déclarée. C'est voulu, pour qu'une
réouverture avec un ancien schéma ne change pas en silence la représentation publique d'une base.
Le banc, plus ancien, change le sien à chaque tour de réglage.

## Le départage

- Rouge sur la tête du lot `usage` (1c85fa06e), en 613 s : 1 passé, 1 rouge.
- Rouge aussi sur la base du lot, c733b81dd, qui n'a pas `usage` : même erreur, au même endroit.
  Ce n'est donc pas le lot.
- Lib courant 373b5477b, `RAG3WEAVER_MOTEUR_ANCIEN=1` dit. Le refus est du Rust, et ne dépend pas
  du moteur.

## Ce qui serait juste

Deux voies, au choix de qui tient le banc :

1. Le banc ouvre **une base par réglage**, ou copie la base indexée avant chaque
   réenregistrement : chaque réglage part d'une entité neuve, et la règle tient.
2. Le banc déclare la migration que la règle attend, s'il en existe une forme pour un changement
   de signaux ou de `boost`. C'est à lire dans `catalog.rs` près du refus.

Rien n'est à relâcher dans la règle du catalogue.


## Diagnostic corrigé, le 18 novembre 2026

Le boost du titre n'était pas en cause. `search_index` (`EntityConfig.search_index`,
`src/config.rs`) est le **nom** de l'entité d'index dérivée, et non les poids.

- `register_code_schema` enregistre `Scope` avec `search_index = Some(SCOPE_CONTENT)`
  (`src/code.rs`, vers la ligne 791) : ce sont les versions.
- Le banc reconstruisait la config de `Scope` par `scope_config(default_scope_chunking())`, qui ne
  pose pas ce champ. Le réenregistrement passait donc `search_index` de `Some(…)` à `None`, et c'est
  ce changement que le catalogue refuse à raison, sur une entité peuplée.

## Le correctif

Il est dans le test seul, sans nouvelle ingestion : `tests/e2e_banc_etage.rs`. Le banc part
maintenant de la config enregistrée (`cat.entity_config(SCOPE).cloned()`) et ne change que le boost
du titre, et les signaux dans le cas du creux. Rien n'est relâché dans le catalogue.

Ni l'une ni l'autre des deux voies proposées plus haut (une base par réglage, une migration
déclarée) n'est nécessaire.

## La preuve

- Le rouge est celui de master : vu deux fois, le 15 novembre, sur la tête du lot `usage` et sur
  c733b81dd, au même endroit et par le même refus.
- Le vert : `e2e_banc_etage` 2/2 (`banc_etage_qui_perd` et `banc_cohesion_produit`), en 1 354 s, sur
  65960ae7a, lib 3d706f709 sans drapeau, validation VERT
  (`~/.cache/rag3weaver-build/validations/validation-o2i9rkg9`).
