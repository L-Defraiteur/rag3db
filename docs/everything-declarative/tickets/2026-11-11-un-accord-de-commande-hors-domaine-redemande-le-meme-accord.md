# Un accord de commande hors domaine redemande le même accord

- **État** : ouvert (11 novembre 2026), relevé par archi, transmis à ED.
- **Pour** : ED (décision et reprise de commande), prise en charge confirmée ; transfert de dossier effectué après la passe figée a92.
- **Signalé par** : archi, navigateur `b57313175` et journal ci-dessous.
- **Famille** : produit
- **Gravité** : l'accord annoncé dans la page ne permet pas de reprendre la commande.
- **Atteignable en service** : oui, carte d'accord de commande du chat atelier.
- **Touche rag3weaver** : oui, décision de commande et reprise après accord.

## Preuve

Sur `b57313175`, le témoin navigateur de commande différée place son script Python dans le
parent de la racine de l'atelier. Le premier appel `run_command` demande l'accord. Le clic réel
sur « accorder » affiche « accordé » et relance le tour avec exactement la même ligne. Le second
appel redemande l'accord ; aucun `wait_output` n'est atteint. Aucun contenu privé n'est impliqué.

Journal luciepc :
`~/.cache/rag3weaver-build/cx-ui-contextuel-b57313175-coupure-attente-20261111/navigateur.log`.
Témoin : `le_chat_attend_la_sortie_differee_d_une_commande_apres_accord` avant déplacement de
son script sous la racine. Rouge conservé ; la fixture suivante sépare ce défaut de l'attente.

La lecture de `commande.rs` montre que `accorder_ligne` enregistre la commande dans les acquis,
mais `juger` contrôle les arguments hors domaine avant ces acquis. Ce mécanisme explique le
comportement observé ; ne pas supprimer la frontière du domaine pour corriger la reprise.

## Pour fermer

Définir et prouver la portée de l'accord explicite sur cette ligne et ce domaine, puis une
reprise qui l'honore sans accorder d'autres chemins ni variantes de commande. Le refus doit
rester observable si cette autorisation n'est pas prise en charge. Tester la carte et le tour
réel, pas seulement l'enregistrement de la décision. Ce correctif est distinct du lot UI
contextuelle et ne doit pas être annoncé livré par le succès de wait_output dans le domaine.

## Transfert à ED après la passe figée a92

Porteur : everything declarative, confirmé avec archi. Le ticket reste ouvert : le transfert
ne corrige ni ne ferme le défaut. Preuve b573 conservée ; reste définir et prouver la portée
de l'accord extérieur et la reprise de la commande. Aucun correctif dans le lot relais/UI.

## Reprise du 12 novembre 2026

ED reprend sur `cx-ed-accord-hors-domaine`, depuis master `6e5f9458c`, avec
`src/commande.rs`, `src/dataflow/run_nodes.rs` et leurs tests Rust comme périmètre.
Archi possède le témoin navigateur et ne modifie pas ces fichiers Rust.

Contrat visé : une décision explicite retient la ligne complète exacte et le domaine
canonique, sans en faire un acquis de famille ni une autorisation de chaque sous-commande
isolée. La même ligne dans le même domaine reprend ; changement de ligne, d'arguments,
d'enchaînement ou de domaine ne bénéficie pas de cet accord. Les refus structurels
restent prioritaires et le bac à sable conserve ses propres garanties.

Prévoir rouge de reprise hors domaine, refus des variantes/domaines et ambiguïtés argv,
puis vert au niveau de la porte et du nœud d'exécution ; archi prouve clic réel et
`wait_output`. Aucun Gemini, service ou base réelle supplémentaire ; ciblés sous poste
lourd avec target privé, lib29, -j8 et plafond 32 Gio. Ticket toujours ouvert.

## Reprise autorisée le 12 novembre — contrat partagé ED/archi

Même ticket, sans nouveau lot fonctionnel. Autorisation Lucie et répartition :
`docs/tickets/closed/2026-11-12-paralleliser-les-suites-autorisees-sans-chevauchement.md`.
Base de départ archi : `6e5f9458cab5140ff2652bab9481be34500628ec`.

- ED possède `commande.rs`, les tests backend et le raccord de décision nécessaire : accord
  associé à la **ligne exacte et au domaine exact**, sans accorder les variantes ni d'autres domaines.
- Archi possède uniquement le témoin navigateur dans
  `extension/rag3weaver/src/daemon_host/chat_contextuel.rs` : nouveau cas extérieur ; le témoin
  nominal intérieur déjà livré reste inchangé. Aucun fichier runtime/React d'ED modifié.
- Worktree archi isolé `~/.cache/rag3weaver-build/cx-wt-accord-exterieur`, branche
  `cx-archi-accord-exterieur-1`. Target unique de session sur luciepc `cx-archi-target`, après
  suppression du target du lot précédent ; aucun cargo concurrent sur ce target.

Le script synthétique restera **hors de la racine atelier**, comme dans le rouge historique
b573 conservé aux deux postes ; aucune adaptation dans le domaine pour faire passer le témoin.
Le navigateur envoie la demande, clique réellement sur la carte d'accord, observe le tour
relancé et le même `run_command` extérieur, puis le vrai `wait_output` sur un journal sans
sortie. Un signal de fixture est libéré seulement après l'annonce du vrai appel d'attente.
La sortie attendue et le résultat de wait_output doivent être vus par le modèle et la page.
La même commande avec un argument supplémentaire, non accordée doit rester en attente d'accord et ne produire
aucun effet ; ni les args du modèle ni le témoin ne donnent de droit en dehors du clic initial.

Les identifiants, résultats et journaux viennent des outils réels. La commande, son domaine
et ses arguments sont enregistrés pour vérifier l'identité de reprise. Modèle simulé pour
la décision d'outil uniquement ; aucune sortie de commande ni décision d'accord simulée.

Préparation et code autorisés ; premier ciblé borné sous poste lourd quand la mesure prioritaire
le permet. Aucun rejeu identique sans diagnostic, aucune passe globale ni push master avant
revue coordonnée. Ce ticket reste ouvert jusqu'aux critères réels ; le succès du témoin dans
le domaine ou un simple enregistrement d'accord ne ferme pas ce défaut extérieur.

## Preuves backend du 12 novembre

Rouge déterministe `6cbbb22a9` : les deux témoins de reprise extérieure et de collision
entre argv partageant un affichage échouent (0/2). Un premier lancement avait refusé le
montage sans sous-module avant compilation ; ce défaut de montage a été corrigé avant
le rouge fonctionnel. Correctif `7e95d5b7a` : 47 tests commande verts, zéro échec, un helper
existant ignoré (`role_hote_qui_sort`) ; 18 tests run_nodes verts, zéro échec/ignoré.
Les nouveaux témoins couvrent la ligne complète, les arguments, les blancs, le domaine,
les sous-commandes isolées, les enchaînements modifiés et le refus structurel sans
accord partiel ; un vrai `cat` extérieur est exécuté après accord.

La garde produit désormais les laissez-passer par `autoriser_ligne`, puis run_nodes
compose cette primitive en exécution synchrone et en fond. Aucun changement du protocole
de carte, aucun élargissement du bac à sable ; un refus du bac à sable reste distinct
de l'accord de la porte. Le témoin navigateur archi `5611fc98e` est repris sans changement
en `b95732394`. Sa preuve et la passe globale restent à consigner avant fermeture.

Journaux luciepc : `~/.cache/rag3weaver-build/I-luciepc/accord-20261112/`
(`rouge.log`, `verts.log`, `commit-verts`, `verdict-cibles`). Jeux ciblés
`rag3db-native,code`, lib29, target privé, poste lourd -j8/32G. Aucun essai Gemini.
