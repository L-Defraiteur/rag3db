# Une annulation de plus de 64 Ko pointe un fichier effacé après la fin du graphe

- **État** : ouvert (8 novembre 2026, relecture du lot de l'équipier `agent/journaux-annulation-tmp-3-3`)
- **Famille** : produit
- **Gravité** : coût (confort) — rien de faux n'est rendu, mais la ligne du nœud promet une
  annulation qui n'existe plus
- **Atteignable en service** : oui, toute exécution non `migration-…` dont un nœud pose une
  annulation de plus de `EN_LIGNE_MAX`
- **Touche rag3weaver** : oui (`src/dataflow/checkpoint_store.rs`)

## Ce que c'est

`poser_l_undo` range une annulation de plus de `EN_LIGNE_MAX` (64 Ko) dans un fichier et écrit son
chemin dans `n.undo_json`. Depuis le lot de l'équipier, `nettoyer_apres_fin` efface tout le dossier
d'une exécution qui n'est pas une migration : la ligne garde le chemin d'un fichier effacé. À la
relecture, `Spiller::lire_texte` échoue et l'échec se tait (`.ok()`) : `undo_context` vaut `None`,
comme pour un nœud sans annulation.

## Le témoin

Pas encore : une exécution non migration dont un nœud pose plus de 64 Ko, `nettoyer_apres_fin`,
puis la relecture de la ligne du nœud — `undo_json` doit être vide (ou dire « effacée à la fin »)
au lieu d'un chemin mort.

## Ce qu'il faut pour le fermer

À la fin, vider `n.undo_json` des nœuds de l'exécution dont le fichier part, ou ne pas écrire de
chemin pour une exécution dont l'annulation ne survit pas.

## Aussi, en passant

- `PreparedBackend::workspace_root()` (lot `agent/racine-disparue`) refait la résolution de
  `workspace.root` déjà écrite trois fois dans `backend.rs` (au chargement, pour les montres, pour
  le bac à sable) : les trois pourraient l'appeler. Décision de c7 du 8 novembre : plus tard, pas
  dans ce lot-là.
