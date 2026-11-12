# Un accord de commande hors domaine redemande le même accord

- **État** : corrigé le 12 novembre 2026, source éprouvée `b42a4ada90ab7f2b80f1137e75692b3d00e26fcc`, verdict composé vert ; fermeture portée par le lot d’intégration ED/archi.
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

## Portée à usage unique et parcours noyau — 12 novembre

Archi a relevé une antériorité dans l'accord exact : une commande bornée `CetteFois`
restait acquise. Rouge `b4c1d13ef` : 0/2 ; second jeu de laissez-passer délivré à tort,
et deux autorisations concurrentes réussies au lieu d'une. Aucun jeton destructeur exécuté.
Correctif `aed5ef115` : portée conservée dans l'accord de ligne, jugement non consommateur,
consommation atomique à `autoriser_ligne` ; 50 tests commande verts / zéro échec / un helper
ignoré exercé par son parent, et 18 run_nodes verts / zéro échec ou ignoré. Le mode standard
reste sans accord différé. Revue du delta par archi sans objection concrète.

Le témoin navigateur a d'abord refusé le montage Chromium (`Socket path too long`) ; seule
la racine TMPDIR sur disque a été raccourcie. Rouge produit sur `5611fc98e` : 0/1/0 en 1,13 s,
après clic réel et deux lignes identiques ; état fini plutôt que wait_output. Sur `aed5ef115`,
le même témoin atteint effectivement wait_output, mais reste rouge (0/1/0 en 30,73 s) :
`python3: can't open file .../attendre-exterieur.py: [Errno 13] Permission denied`, stdout vide.
Le bac à sable par défaut ne permet pas de lire ce script extérieur. Le ciblé run_nodes sans
bac ne constitue donc pas une preuve de réussite du parcours atelier complet.

Preuves luciepc :
`~/.cache/rag3weaver-build/cx-accord-exterieur-5611fc98e-rouge-produit-20261112/navigateur.log`,
`~/.cache/rag3weaver-build/cx-accord-exterieur-aed5ef115-vert-20261112/`
(`navigateur.log`, `commande-reelle.stderr`, `commande-reelle.stdout`) et dossier ED
`I-luciepc/accord-20261112/` (`rouge-portee.log`, `verts-aed5.log`). Le nom du dossier
« vert » décrit l'intention du lancement, pas son verdict : il est rouge.

Choix soumis à orchestration avant toute suite : isoler la reprise de garde avec une lecture
préalablement déclarée et limitée au seul fichier script (script toujours extérieur, baseline
à reprouver), ou définir une capacité extérieure portée par l'accord et le laissez-passer.
Cette seconde voie exige de préciser les droits et les refus sans ouvrir globalement le bac.
Aucune de ces options n'est livrée par ce constat. Aucun script déplacé dans le domaine,
aucune assertion/durée relâchée, aucune désactivation de bac ni nouvelle tentative inchangée.
La passe globale n'a pas commencé ; ticket ouvert, parcours complet non validé.

### Frontière retenue avant la modification de fixture

Orchestration autorise de composer `workspace.sandbox.extra_read` existant, limité au
**seul fichier script**, sans son parent. Le script reste extérieur au domaine de Garde :
la baseline doit toujours demander à nouveau l'accord après clic, même avec ce droit noyau
préalablement déclaré. Le correctif doit ensuite atteindre sortie réelle et refus de variante.
L'accord de ligne ne confère aucun droit noyau supplémentaire dans ce lot.

Archi possède cette modification de fixture navigateur et ses nouveaux rouge/vert. ED possède
un témoin runtime distinct avec le vrai bac : fichier synthétique lisible par l'hôte, lecture
refusée et erreur rendue sans extra_read, succès avec le seul fichier déclaré, frère extérieur
encore illisible. Ce diagnostic comparatif doit départager les droits ordinaires et Landlock ;
le stderr seul n'est pas une preuve causale suffisante. Aucun sandbox off, aucune ouverture du
parent ou du home, aucun accès extérieur implicite via les laissez-passer. Source à publier
propre avant les ciblés ; passe globale seulement après ces preuves. Le rouge original sans
extra_read reste conservé et la précondition devra rester explicite dans README et bilan.

Diagnostic causal `157a2f272`, ciblé réel Linux : **1 réussi, 0 échec, 0 ignoré en 0,08 s**.
Le même script et son frère sont lisibles par l'hôte ; le témoin exige un ruleset Landlock
actif (aucun saut), puis fait varier seulement la déclaration extra_read du script. Sans
elle, le processus réellement lancé après accord rend Permission denied et aucune sortie ;
avec elle, le script rend sa sortie. La variante attend son accord ; même explicitement
accordée ensuite, sa lecture du frère reste refusée. Aucune règle sur le dossier parent.
Ce comparatif étaye la cause noyau plutôt que de l'inférer du seul stderr navigateur.
Preuve : `I-luciepc/accord-20261112/bac-reel.log` et `commit-bac-reel`.
Les README explicitent désormais la lecture déclarée préalable et la consommation CetteFois.

### Compatibilité des lignes shell littérales

Revue ED : le premier correctif ajoutait à tort un refus de tout faits.shell, alors que
l'ancien accorder_ligne excluait seulement les décisions Refuse. Rouge a2db2abc3 : 0/1,
sans exécuter de shell. Correctif9a1a8539c : les deux conditions ajoutées sont retirées ;
la ligne littérale explicitement accordée reste à usage unique, variantes/élévation et
formes non réduites refusées. Ciblés : commande51/0/+1helper, run_nodes19/0/0.
Première globale4df arrêtée avant la lib large, puis quiescence vérifiée ; ses cinq jeux
de features et1594 tests natifs restent des preuves partielles de cette seule source.
La nouvelle passe inclura le master3b2eab7ec intégré entre-temps et le correctif final.

## Verdict de fermeture du lot

Source de code figée b42, lib29 : features **5/5**, native **1609/0/15**, lib large
**2176/0/65**, navigateur **40/0/0**. Le 65e ignoré ajouté est exactement
`daemon_host::chat_contextuel::un_accord_exterieur_reprend_la_ligne_exacte_et_n_accorde_pas_sa_variante`,
marqué « témoin navigateur : étape séparée » sans la feature navigateur ; il est réellement
joué et vert dans les40. L'unique helper ignoré de commande est exercé par son test parent.

Verdict composé explicite : les étapes features/lib/nav de la globale b42-r2 sont conservées ;
les seules e2e ont été revalidées pour prouver le confinement des vrais enfants, après
autorisation orchestration : **lectures24/0/0 en9,49s**, **démon13/0/0 en12,26s**,
**2 suites sur2, zéro sauté**. PIDs2697195 et2702388 réellement relevés dans
`cx-ed-accord-e2e-confines-20261112.service`, memory.max34359738368 et KillModecontrol-group.
Budget600s après acquisition réelle de poste lourd ; durée23,482s, pic11,4Gio ; quiescence
exeTarget/cwdWT vide. Aucun droit noyau implicite, aucune variante exécutée sans accord.

ELF navigateur SHA256 `98bc3103ec21dffeca418a9cb27f2e5f37565fb7369fab2217e379a86f19ed94`,
empreinte source b42a4ada9 et220 gabarits présents, aucun absent. Preuve finale composée :
`~/.cache/rag3weaver-build/I-luciepc/accord-20261112/verdict-compose.json`.

Les rouges initiaux restent conservés. La dette des fixtures de journaux est séparée sous
`docs/recherche/tickets/2026-11-12-les-temoins-de-lecture-imposent-le-nom-du-dossier-des-journaux.md` ;
le suivi commun des scopes imbriquées reste ouvert à orchestration. Ce lot ne corrige ni
ces fixtures ni le runner produit. Aucun nouvel essai Gemini, activation ou base réelle.

Intégration effective : master `adcf01128527f4cb24027364489bd44e322ca089`. Les contrôles
post-push et le retrait gardé des seules ressources du lot sont consignés dans le
[rapport ED final](../../everything-declarative/12-11-2026/01-rapport-de-session.md).
Les nouveaux tickets de dette restent ouverts ; leur travail n'est pas déclaré livré ici.
