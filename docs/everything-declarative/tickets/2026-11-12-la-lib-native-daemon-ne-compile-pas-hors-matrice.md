# La lib native+daemon ne compile pas hors matrice

- **État** : ouvert ; compilation rouge observée, aucun correctif dans le lot BGE.
- **Famille** : produit
- **Gravité** : combinaison de features hors matrice empêchée de compiler.
- **Atteignable en service** : non qualifié ; échec à la compilation des tests.
- **Touche rag3weaver** : oui, chat daemon et témoin livraison Codex.
- **Pour** : ED.
- **Signalé par** : préfixe BGE du 12 novembre 2026.
- **Ouvert le** : 12 novembre 2026.

Sur source `5eef3fe01` (base `41497ba4e`), à luciepc :
`bash scripts/cargo_magasin.sh test -j8 --lib --features rag3db-native,daemon -- model_source::tests::gabarits_bge_ --nocapture`
s'arrête avec code 101 avant tout témoin. Le journal conservé est
`~/.cache/rag3weaver-build/I-luciepc/bge-routage-20261112/rouge.log`.

Deux E0433 dans `src/daemon_host/chat.rs:835-836` : accès à
`dataflow::fichier_nodes::LIGNES_MAX`, alors que fichier_nodes est gardé par
la feature `code`. Troisième erreur : import inutilisé `crate::mail::Boite`
dans `src/daemon_host/livraison_codex.rs:92`, refusé par les warnings stricts.
Ces deux fichiers sont inchangés par le lot BGE. La combinaison native+daemon
n'est pas un jeu de check_features ; aucune régression BGE n'est établie.

Suite : qualifier le contrat des features puis corriger gardes/dépendances et
import au bon périmètre, sans masquer les warnings ni supprimer de témoins.
Fin : compilation et témoins concernés verts pour la combinaison annoncée,
ou refus explicite de cette combinaison documenté dans le contrat.
