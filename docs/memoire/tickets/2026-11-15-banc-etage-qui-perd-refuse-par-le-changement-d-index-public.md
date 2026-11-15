# `banc_etage_qui_perd` tombe à son installation : changement d'index public refusé

- **État** : ouvert, le 15 novembre 2026.
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
