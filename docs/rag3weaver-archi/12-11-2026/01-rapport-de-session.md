# Archi — 12 novembre 2026

## Lot autorisé en cours : accord de commande hors domaine

Ticket canonique :
`docs/everything-declarative/tickets/2026-11-11-un-accord-de-commande-hors-domaine-redemande-le-meme-accord.md`.
Autorisation/répartition désormais dans
`docs/tickets/closed/2026-11-12-paralleliser-les-suites-autorisees-sans-chevauchement.md` :
ce ticket clos atteste seulement l'affectation, pas la réalisation fonctionnelle.

ED possède commande.rs, run_nodes.rs et leurs gardes/tests backend. Archi ne modifie que
le témoin navigateur `src/daemon_host/chat_contextuel.rs` (et ses propres docs) ; aucun
changement runtime/React. Le script est réellement hors racine atelier ; le nominal intérieur
livré reste inchangé. Nouveau témoin :
`un_accord_exterieur_reprend_la_ligne_exacte_et_n_accorde_pas_sa_variante`.

Après clic réel sur la carte : reprise de la même ligne, vrai run en fond, vrai wait_output
sur sortie encore absente, signal libéré après annonce de l'attente, sortie et cwd réels
vérifiés. Ensuite même script avec un argument ajouté : nouvel accord requis, jamais donné,
aucun effet du mode variante. Ni décision ni résultat de commande simulés.

## État des preuves

- Rouge historique produit `b57313175` conservé sur les deux postes dans
  `cx-ui-contextuel-b57313175-coupure-attente-20261111/navigateur.log` du cache de build.
- Préparation/ticket avant code `59a55ced3`, témoin `5611fc98e`, base master `6e5f9458c`.
- Premier lancement navigateur `5611fc98e` compile, puis Chromium ferme le transport dès
  l'ouverture : **0 réussi, 1 échec, 0 ignoré, 0,49 s**. Aucun accord ni appel d'outil atteint ;
  ne pas compter ce défaut de montage comme nouveau rouge fonctionnel de l'accord.
  Preuves PC : `cx-accord-exterieur-5611fc98e-baseline-20261112/`.
- Diagnostic instrumenté (stderr Chrome auparavant jeté) : premier lancement retiré de la
  file avant entrée du script, mesure exclusive prioritaire ; **NON JOUÉ**, pas un échec produit.
  Dossier `cx-accord-exterieur-5611fc98e-chrome-diagnostic-20261112/`.
- Diagnostic suivant en attente sous poste lourd, dossier
  `cx-accord-exterieur-5611fc98e-chrome-diagnostic-2-20261112/`. Son unité de test bornée sera
  créée **après acquisition** du poste ; aucune compilation ou exécution hors mesure.

ED annonce son correctif et ses ciblés : 47 commande réussis (helper enfant
`role_hote_qui_sort` ignoré explicitement), 18 run_nodes réussis. Source ED actuelle annoncée
`2e459d9d4`, dont garde Standard ; test navigateur 561 repris inchangé chez ED. Aucun résultat
UI corrigé ni vert global revendiqué ici.

## Reprendre

Worktree isolé sur les deux postes : `~/.cache/rag3weaver-build/cx-wt-accord-exterieur` ;
branche archi `cx-archi-accord-exterieur-1`. Code navigateur figé `5611fc98e` sur PC ; les
compléments locaux suivants sont documentaires. Target unique PC `cx-archi-target`, scratch
`cx-accord-exterieur-scratch`, lib épinglée complète 29a7bc52a. Pas de base réelle ni service GPU.

Lire d'abord le diagnostic Chrome et son stderr, corriger seulement le montage prouvé, puis
obtenir le rouge produit sur la base non corrigée. Combiner ensuite source ED figée et même
fixture, annoncer SHA, jouer un seul ciblé corrigé borné. Aucun replay inchangé aveugle,
aucune passe globale ni push master avant revue. Le ticket fonctionnel reste ouvert.

Aucun nettoyage avant fin du lot. Le sous-module privé a été initialisé : son administration
est relevée dans le dossier baseline. Pour tout retrait utiliser garde_worktree.py courant,
archives administratives fidèles hors cible, rapport neuf et quiescence ; refuser en cas de
worktree dépendant ou doute. L'ancien script de nettoyage du lot UI du 11 ne convient pas.
