# Reprise archi — 12 novembre

Lire le rapport voisin et le ticket canonique ED d'accord extérieur. ED possède le runtime,
archi uniquement le témoin navigateur chat_contextuel.rs ; un ticket commun, pas deux lots
fonctionnels concurrents. Le nominal intérieur livré n'est pas une preuve de l'accord extérieur.

Le témoin nouveau 5611fc98e garde le script au parent de rw, avec une garde starts_with sur
les chemins canoniques. La variante ajoute un argument au même script pour vérifier la ligne
exacte, pas seulement la famille ou le chemin. Le signal ne se libère qu'après ToolCallStarted
wait_output visible dans la page. Ledger des args/reçus réels ; aucune sortie inventée.

Première compilation verte, fermeture CDP Chrome à 0,49 s : pas de rouge produit, aucun clic.
Instrumenter stderr via wrapper externe aux sources, ne pas modifier les assertions pour
faire disparaître un défaut de montage. Les traces sont hors target, dossiers nommés par SHA.

La mesure a priorité. Poste peut lancer la commande dans sa propre scope systemd : placer
l'unité bornée **dans la commande après acquisition**, avec KillMode=control-group, afin que
son échéance couvre les vrais descendants et ne se consomme pas dans la file de mesure.
Pas de nouveau lot ou correctif hors du ticket d'accord sans signalement/autorisation.


Diagnostic Chrome confirmé : le socket SingletonSocket dépassait sa longueur permise avec
le scratch long. La seule correction a été TMPDIR=cx-ae ; aucun changement de fixture.
Cela a permis le rouge produit561. Après intégration aed, l'accord/reprise et wait_output sont
atteints, mais Python reçoit Permission denied sur le script extérieur : refus physique non
couvert par le ciblé backend sans sandbox. Garder ce rouge et ses stdout/stderr, ne pas masquer
le défaut en déplaçant le script ni en accordant implicitement des accès supplémentaires.
Source ED/raccord sous revue. La variante non accordée n'a pas encore été éprouvée dans Chrome.
