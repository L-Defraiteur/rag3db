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

## Reprise autorisée après la livraison du parcours IDE — 13 novembre

Les preuves initiales viennent de l'ancienne source ; vérifier d'abord les
frontières de features sur le master qui intègre ED43e4f185b et Embar23ceed87b.
ED reprend ce défaut existant après son nettoyage terminé, dans un worktree
neuf. Le périmètre est boîte, lifecycle et usages d'outils sous leurs features
réellement nécessaires ; ni nouveau comportement d'agent ni feature native
ajoutée par défaut pour cacher la dépendance.

Publier le diagnostic actuel, le contrat de features retenu et la recette CPU
bornée avant compilation. Corriger les gardes/imports si les combinaisons sont
bien supportées, puis prouver le jeu minimal sans natif, daemon seul et
daemon+natif sans code, ainsi que le parcours de découverte concerné. Un refus
volontaire doit être explicite et cohérent avec le contrat publié, pas un E0433
accidentel ni une exclusion silencieuse des témoins. Pas de modèle/service/PG
pour ce ciblé ; source figée, poste et plafond mémoire réels, lib épinglée pour
la combinaison native. Premier rouge inattendu arrêté et diagnostiqué.

Coordonner les rares fichiers partagés avec Archi ; son lot grep garde ses
outils/Plans. Aucun ancien worktree nettoyé n'est rouvert, aucune pose du daemon
personnel. Revue et passe adaptée avant tour master ; clôture seulement après
correction et preuves des combinaisons réellement annoncées.
