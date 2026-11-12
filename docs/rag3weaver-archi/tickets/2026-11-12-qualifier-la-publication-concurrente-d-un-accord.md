# Qualifier la publication concurrente d'un accord

- **État** : ouvert ; hypothèse statique relevée pendant la conception du mode auto, non reproduite.
- **Famille** : produit
- **Gravité** : portée d'accord potentiellement incorrecte à qualifier ; aucun défaut reproduit ni effet de commande démontré.
- **Atteignable en service** : potentiellement via deux réponses à la même carte ; scénario concurrent non exécuté.
- **Touche rag3weaver** : oui, daemon_host/atelier_fichiers.rs::decider_accord et la publication vers la Garde.
- **Pour** : ED pour la qualification, archi pour le parcours ; aucun correctif engagé par l'étude.
- **Signalé par** : revue ED/archi de la conception du mode auto le 12 novembre, relecture statique confirmée par orchestration.

## Observation et limite

Lecture sur adcf01128527f4cb24027364489bd44e322ca089,
`src/daemon_host/atelier_fichiers.rs` : decider_accord lit une demande non décidée,
relâche le verrou du registre, installe l'accord dans la Garde, puis reprend le
registre pour vérifier et écrire la décision. Deux réponses pourraient franchir
la première vérification ; une installation tardive pourrait intervenir après
une consommation CetteFois, avant que la seconde réponse soit refusée.

Ce raisonnement n'est pas une reproduction. Le verrou de consommation de la
Garde ne suffit pas à démontrer l'atomicité de la publication amont. Le correctif
de reprise d'accord déjà livré n'est pas rouvert par cette hypothèse distincte.

## Qualification et fin

Après autorisation séparée, construire un témoin déterministe non destructif de
deux réponses au même identifiant, de leur publication et du nombre d'usages
effectivement disponibles. Couvrir reçu en erreur, révocation, annulation et
reprise avant de conclure à un défaut et de choisir le contrat commun.
Conserver les preuves ; fermer comme corrigé ou non reproduit selon le résultat,
sans annoncer de course établie sur la seule lecture. Aucun test ou appel de
commande exécuté pour cette note.

[Conception liée](../../orchestration/visions/2026-11-12-mode-auto-contextuel-et-accords.md).

## Reprise parallèle autorisée le 12 novembre

Lucie demande que les sessions libres prennent des chantiers utiles en parallèle
des couches. Orchestration confie à ED la qualification bornée de cette hypothèse,
avec archi en revue seule du parcours. Cette reprise couvre le témoin déterministe
sur registre et Garde synthétiques, sans exécuter de commande réelle ni ouvrir
les données ou accords de Lucie. Aucun modèle de décision nouveau n'est activé.

Premier résultat attendu : établir par barrières contrôlées si deux réponses
peuvent produire plus d'un usage CetteFois, puis qualifier erreur, révocation,
annulation et reprise. Conserver aussi le résultat négatif si le contrat tient.
Si le défaut est reproduit, une correction bornée doit préserver l'unicité de
publication et de consommation déjà attendue, sans changer la politique d'accord.
Publier source, diagnostic et recette avant le ciblé ; arrêt au premier rouge
inattendu, aucun rejeu inchangé. Le ticket reste ouvert jusqu'au verdict relu et,
si nécessaire, à l'intégration du correctif.

Propriété ED : atelier_fichiers et témoins de publication d'accord. Archi relit,
sans édition parallèle ; Arbre garde backend_schema et migrations. Les worktrees,
targets et fixtures sont distincts. Le créneau de push sera coordonné après la
passe des couches ; aucune réservation de master pendant le développement.
