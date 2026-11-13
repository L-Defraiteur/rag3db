# Passation week-end — optimiseur, 13 novembre 2026

Passation demandée par Lucie pour limite d’usage. Veille : aucun nouveau GO, calcul,
bâti, test lourd, pose ou nettoyage. Les anciens délais et autorisations ne se
réactivent pas. C7 rassemble ce rapport depuis une branche NON master.

## Livré et en cours

- Binaire posé aux deux postes : `68b9ede17c182487d3f823c2e5cc5906b62a4a46`,
  SHA256 `6af3ec67450863021735359132b8d68e2afb97a8e0c50f2b182fd94a0bc44e2e`,
  contre lib `29a7bc52aaf11e6204f9beafaffb154105417e13`. Dernière pose documentée
  le 11 novembre vers 22 h ; aucune nouvelle pose depuis par cette session.
- Attente PID/port privé : correctif `62640bd42`, deadline unique avant SIGTERM,
  refus des changements d’identité ; livré. Aucun service réel actionné pour ce test.
- Provenance CI vector : `567d2de50`, fixture workflow `d03ef6fc2`, livré ; sélection
  OUT_DIR par messages cargo liés à la source et au CMake, refus absence/ambiguïté.
- Garde fenêtre card0 : `24f3603cf7295f4def9c58cb68ac96718a77494b` ; clôture
  `fa4dc35001c4b86494ab0b9954cb6e2d2d619f0d`, intégrée avant cette consigne.
  Cutoff réel du 13 novembre 07:54 CET, Result success / rc0 ; lecture indépendante
  07:58 : registre fermé sans réouverture, avant/après et descendants vides.
  Aucune unité modèle privée enregistrée : aucun déchargement GPU réel démontré.
- Pas de code WIP identifié dans les worktrees possédés contrôlés ci-dessous.
  Seule cette passation reste à rassembler ; ne pas reprendre les anciens « prochains pas »
  du rapport du 11 novembre comme des ordres actuels.

## Branches, remotes et worktrees

Remote `origin` du dépôt rag3db. Branche de ce rapport :
`cx-optimiseur-passation-weekend`, poussée sur `origin` ; son SHA est le commit qui
ajoute ce fichier (`git log -1 --format=%H -- docs/optimiseur/13-11-2026/99-passation-week-end.md`).
Worktree conservé : `/home/lucied/git_workspaces/cx-optimiseur-passation-weekend`.
Base de préparation : `3a5080d93bdaeba05fd333b25bdd0565b49ad18b`.

| Poste et worktree conservé | HEAD relue à la passation | Branche | État |
|---|---|---|---|
| portable `/home/lucied/git_workspaces/rag3db-docs-G` | `50a37c80bd3a7ee1daa096d25316abd936e13eba` | `HEAD détachée` | propre |
| portable `/home/lucied/git_workspaces/rag3db-plafond-G` | `d845744a2b901621879843280d6b6c2c8c9b5f00` | `plafond-target-32` | propre |
| portable `/home/lucied/git_workspaces/rag3db-poste-G` | `e006f4cffaa38fa493d6fd3d5854aefa667f3d56` | `poste-n-detenteurs` | propre |
| portable `/home/lucied/git_workspaces/cx-optimiseur-banc-7882` | `96d80615d554079cd192f6a8d043d60753bd35db` | `cx-optimiseur-banc-7882-2` | propre |
| luciepc `/home/lucied/git_workspaces/rag3db-controle-G` | `d1345a115a8044c5b5f5947e26d5aaba8d340053` | HEAD détachée | propre |
| luciepc `/home/lucied/.cache/rag3weaver-build/wt-lib-arbre` | `29a7bc52aaf11e6204f9beafaffb154105417e13` | HEAD détachée | propre |

La branche `origin/cx-optimiseur-banc-7882-2` conserve exactement
`96d80615d554079cd192f6a8d043d60753bd35db` (attestation historique des deux séries).
Elle n’est pas une nouvelle activation. Les autres worktrees historiques ci-dessus
portent des lots livrés, sans WIP ; aucun retrait ici. Aucun worktree `cx-optimiseur-*`
actif à luciepc dans l’inventaire relu. Le micro-worktree card0-restitution a été
retiré avant cette passation après archive administrative et garde VERT ; ses
branches remotes et preuves sont conservées. Les arbres principaux de Lucie
restent hors périmètre d’écriture.

## Tickets et preuves

Ticket ouvert conservé :
`docs/optimiseur/tickets/2026-11-08-le-plein-texte-de-code-lourd-compte-plus-de-documents-que-de-lignes.md`.
Constat historique : Scope 82 958 documents / 82 855 lignes après index complet.
Suite attendue recherche/lucivy ; pas de réindexation autorisée par cette passation.

Tickets livrés dans `docs/tickets/closed/` :
- `2026-11-11-preparer-et-poser-le-binaire-regroupe-aux-deux-postes.md` ;
- `2026-11-12-attendre-la-disparition-du-port-apres-l-arret-du-prive-7882.md` ;
- `2026-11-12-la-ci-cherche-le-bati-vectoriel-dans-un-dossier-absent.md` ;
- `2026-11-12-borner-la-fenetre-nocturne-de-card0.md`.

Preuves hors dépôt, conservées aux deux postes sous `~/.cache/optimiseur-notes/` :
`pose-68/`, `attestation-68-*`, `attente-port-12nov/` (rouge historique et 14 filets
CPU verts), `ci-vector-12nov/` (rouge historique et 20 fixtures CPU vertes),
`audit-residence-12nov/`. Les séries CPU ont été jouées aux deux postes sous
poste lourd ; aucune CI distante réelle rejouée. Aucun test relancé pendant cette passation.

À luciepc, `card0-20261113/` conserve source, config, reçus `restitution.json`,
`ferme.json`, et `preparation/verification-restitution-13nov.json`.
Source déployée SHA256 :
`0e11e0a3e6d0d47bee04b9cda3ce1b97db4c722f96e1852ac97d1b5d6ef67739`.
Reçu restitution SHA256 :
`406dc82e8e274138542096967481601738f4b40870946950b8006e9c7f0dd8a9`.
22 fixtures CPU vertes aux deux postes ; vraie entrée CPU systemd contrôlée,
hors portée et portée étrangère refusées avant sonde ; descendant setsid éliminé.
Ce sont des preuves CPU, pas une qualification de placement des poids GPU réels.
Au portable, `card0-20261113/restitution-13nov/` conserve preuve indépendante,
contrôles du push, archive administrative, `garde-retrait.json` VERT et nettoyage.

NON JOUÉS : chargement réel d’un modèle sur card0, déchargement réel de ses poids,
CI distante, nouvelle pose, recompte FTS à froid. Les rouges historiques restent
consultables ; ne pas les remplacer par une réussite supposée.

## Ressources, processus, verrous et contraintes

Dernier état documenté (pas une nouvelle mesure GPU) : card2 PCI04 pour les
services normaux ; card0 PCI07 revient à Lucie. Exception du 13 novembre expirée.
Timer privé `cx-card0-20261113-cutoff.timer` active/elapsed sans prochaine échéance,
service cutoff inactive/dead ; état lu à07:58. Configs et reçus gardés, aucun replay.

7878 granite-278m et 7879 BGE/relecteur/OCR à garder chauds. Derniers PID connus
966256 et3415586, non revalidés ici. 7880 granite-107m arrêté/désactivé, non masqué ;
7882 privé arrêté après audit, sans restauration automatique. Ne pas substituer
un modèle ni démarrer une charge pour « vérifier ». Aucun processus de calcul
possédé par cette session à reprendre. À la passation, `poste --qui` à luciepc
indique libre, 0/4 lourds ; aucun verrou tenu par cette session.

Conserver magasins, sauvegardes, instantanés, targets et worktrees historiques.
Le target de préparation `target-G/hote-lucie` avait été retiré après la pose ;
ne pas supposer que les chemins historiques du 11 novembre existent encore.
Pas de nettoyage ici. `.env` et `.vault` ne se lisent/copient pas. Identité Git
Lucie Defraiteur <luciedefraiteur@gmail.com>. Aucun push master pour cette
passation, aucun force push. Tout futur lot attend sa consigne et les règles
relues dans `docs/README.md`, `docs/procedures/push-d-un-lot.md` et le dump.

## Trois prochains pas à la reprise

1. Reprendre le contexte et la consigne actuelle, sans ancien délai automatique :
   `git fetch origin` puis
   `git show origin/cx-optimiseur-passation-weekend:docs/optimiseur/13-11-2026/99-passation-week-end.md`.
   Lire `docs/README.md`, le rapport/dump du 11 novembre (leurs mises à jour en tête)
   et le ticket FTS ouvert. C7 rassemble cette branche ; aucune intégration autonome.
2. Si un état de poste est demandé, lectures seules :
   `~/.cache/rag3weaver-build/poste --qui` et
   `ssh luciepc '~/.cache/rag3weaver-build/poste --qui'` ;
   `ssh luciepc 'cat ~/.cache/optimiseur-notes/card0-20261113/restitution.json'`.
   Ne pas rejouer le cutoff ni l’entrée modèle. Relire statuts/PID avant de les citer
   comme actuels ; ne pas employer les PID historiques pour envoyer un signal.
3. Attendre le prochain périmètre explicite. Pour toute pose/calcul futur, relire
   les procédures et réserver les ressources adaptées ; ne lancer maintenant
   aucune commande de bâti/pose/GPU. Le FTS reste un constat à transmettre avec
   son ticket, pas une permission de modifier les bases.
