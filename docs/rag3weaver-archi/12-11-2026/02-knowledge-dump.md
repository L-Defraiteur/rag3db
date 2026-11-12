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
