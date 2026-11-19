# Un accord peut être appliqué malgré une erreur de journal

- **État** : fermé (19 novembre 2026, ED) — le contrat de reçu est posé depuis 0f4207fa59 (13 novembre) : `decider_accord` écrit le reçu **avant** de publier l'accord ; sans journal, rien n'est publié et l'erreur le dit (témoin `erreur_du_journal_apres_commit_ne_signifie_pas_absence_d_effet`, attente changée) ; un reçu dont la publication échoue ensuite est marqué par une seconde ligne « non posé », avec sa cause, la demande reste ouverte (témoin `a_receipt_whose_publication_fails_is_marked_not_posed`, éprouvé par mutant : rouge sans la ligne « non posé », vert avec) ; un accord ne se rejoue jamais au redémarrage (`recharger_la_carte_conserve_la_decision_redemarrer_ne_la_rejoue_pas`).
- **Famille** : produit
- **Gravité** : reçu en erreur alors que l'accord est déjà appliqué en mémoire.
- **Atteignable en service** : si le journal decisions.jsonl ne peut pas être écrit.
- **Touche rag3weaver** : oui, daemon_host/atelier_fichiers.rs::decider_accord.
- **Pour** : ED ; contrat de reçu à qualifier avec orchestration.
- **Signalé par** : qualification déterministe publication d'accord, 12 novembre2026.
- **Ouvert le** : 12 novembre2026.

Source b708f7c34, témoin erreur_du_journal_apres_commit_ne_signifie_pas_absence_d_effet
(module publication_accord_tests) : hôte sans backend, chemin de fixture et journal
strictement privés. decisions.jsonl est un dossier : append échoue, decider_accord
rend une erreur mais etat_accord dit accordé et accorde(path,read) est vrai. Un retry
est refusé comme déjà décidé. Aucun accès réel, commande ou accord utilisateur utilisé.

Ce défaut est distinct de la republication concurrente : publication/commit précèdent
l'écriture du reçu. Pas de rollback ni changement de politique dans le lot concurrence.
Fin attendue : préciser le contrat de reçu en échec et la reprise, puis choisir et
éprouver un comportement qui ne laisse pas croire absence d'effet ; journaliser
les limites de durabilité sans rejouer un accord implicitement au redémarrage.
