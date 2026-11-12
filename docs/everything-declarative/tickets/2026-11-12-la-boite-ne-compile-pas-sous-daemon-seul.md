# La boîte ne compile pas sous la feature daemon seule

- **État** : ouvert ; relevé pendant la qualification CPU de résidence.
- **Famille** : produit
- **Gravité** : blocage ; compilation de la bibliothèque avec daemon seul impossible.
- **Atteignable en service** : non ; refus de compilation.
- **Touche rag3weaver** : oui, src/mail.rs et garde de feature daemon_lifecycle dans lib.rs.
- **Pour** : everything declarative, propriétaire boîte/hôte.
- **Signalé par** : embarquement, qualification du ticket d'activation des modèles.
- **Ouvert le** : 12 novembre 2026

## Preuve

Sur f3f8dc80f, luciepc, `cargo test --lib --features daemon
model_source::activation::tests -- --nocapture` échoue avant exécution :
`error[E0433]: cannot find daemon_lifecycle in crate` à `src/mail.rs:120:28`.
`daemon_lifecycle` est gardé par `all(daemon, rag3db-native)` dans lib.rs:247.
Le fichier mail.rs n'est pas modifié par le lot d'embarquement.
Log : `~/.cache/embarquement-notes/activation/cpu-b.log` à luciepc.

## Reste à faire

Qualifier le contrat de features de la boîte : rendre le minimum daemon
compilable ou refuser explicitement cette combinaison si elle n'est pas supportée,
sans importer le moteur natif pour masquer une dépendance de déclaration.
Vérifier les jeux supportés ; ne pas déclarer les témoins d'activation verts
sur cette tentative qui n'a joué aucun test. La suite d'embarquement utilisera
le jeu natif/daemon supporté, avec lib épinglée.

## Fin attendue

Correction et compilation du jeu minimal retenu, puis preuve du comportement
de découverte de la boîte sous ses features réellement nécessaires.

## Deuxième combinaison relevée, même frontière de features

Sur 3520ec019, `rag3db-native,daemon` échoue dans
`daemon_host/chat.rs:835-836` (fichier_nodes exclu sans code) et import Boite
inutilisé sous -Dwarnings dans `daemon_host/livraison_codex.rs:92`.
Log luciepc `~/.cache/embarquement-notes/activation/cpu-c.log`.
Ces fichiers ne sont pas modifiés par la tranche d'embarquement.
La prochaine qualification utilise `rag3db-native,code,daemon`, jeu supporté
par les passes usuelles, avec la lib épinglée et aucun GPU requis.
