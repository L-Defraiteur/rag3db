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
