# Passation Banc — 13 novembre 2026

Reprise différée au week-end à la demande de Lucie : aucun nouveau GO, calcul,
réveil de modèle ou push master. Les anciens créneaux et délais ne se réactivent
pas. C7 rassemble les rapports. Cette passation conserve les travaux et preuves,
sans nettoyer les worktrees ni les archives.

## Livré et état du travail

Le banc Qwen3-4B k5/k10 sur card2 est terminé : 440/880 paires, médianes
540/787 ms, top1 67/88 et 58/88. Rapport
`docs/banc/12-11-2026/01-le-juge-sur-card2.md`, publication master historique
`ee40dbcd638bc488f9dddc0b1086ea113000dad9`. Ne pas refaire ce banc par défaut.
Ticket fermé : `docs/tickets/closed/2026-11-11-mesurer-le-juge-local-avec-dix-fiches.md`.

Le soutien Banc au lot résidence est terminé : harness CPU puis preuve GPU
réelle locale, annulation d’un demandeur vivant, partage, maintien explicite,
réacquisition, nettoyage. Rapport détaillé
[03-resultats-residence-privee.md](../12-11-2026/03-resultats-residence-privee.md),
commit documentaire isolé `f2d9bfbbb2a180e9d29f2b24e4c4482fe57c08bb`.
Ticket commun toujours ouvert :
`docs/embarquement/tickets/2026-11-12-gerer-activation-et-residence-des-modeles.md`.
Embarquement possède le runtime et son intégration ; Banc ne doit pas fusionner
sa branche entière, qui contient un ancien runtime de préparation. Le commit
documentaire isolé peut être repris après vérification de ce qui a déjà atterri.

Attente sans mesure :
`docs/banc/tickets/2026-11-12-qualifier-gemini-decision-1-apres-son-arrivee-sur-vertex.md`.
Attendre la disponibilité Vertex ; ni AI Studio ni API de substitution.
Suivi externe :
`docs/optimiseur/tickets/2026-11-12-attendre-la-disparition-du-port-apres-l-arret-du-prive-7882.md`.

## Branches, remotes et worktrees

Remote de publication : `origin`, `git@github.com:L-Defraiteur/rag3db.git`.
Vérification locale avant cette passation : les quatre worktrees ci-dessous sont
propres, HEAD identique à leur référence origin. Le présent rapport ajoute un
commit à la branche résidence-appels ; son SHA est donné dans la notification
finale et se retrouve avec `git log -1`.

| Branche / worktree sous `/home/lucied/git_workspaces/` | SHA avant passation |
| --- | --- |
| cx-banc-residence-appels | f2d9bfbbb2a180e9d29f2b24e4c4482fe57c08bb |
| cx-banc-residence-nocturne | c932e0622ea276f68f5d47126192e4cf92ea4d34 |
| cx-banc-suivi-k10 | 96d4f9a9e56399c22f7eced519a0eef7a30e6cd4 |
| cx-banc-placement-explicite | 0495833e9f0a24be8fa7a0a0bf583b2c4eddd02e |

Deuxième poste : `ssh luciepc`, hostname LuciePC. La liste des worktrees de son
dépôt `/home/lucied/git_workspaces/rag3db` ne contient pas de worktree Banc au
contrôle de passation ; les paquets et preuves sont hors dépôt, pas des WIP git.
Le dépôt principal local de Lucie reste intouché. Aucun WIP Banc non poussé
identifié dans les quatre worktrees contrôlés. Les preuves volumineuses restent
volontairement hors git et préservées aux emplacements ci-dessous.

## Preuves, tests et limites

Racine aux deux postes : `~/.cache/rag3db-banc-notes/residence-nuit-12nov/`.
Conserver `cpu-01`, `demandeur-714fb8a65`, `gpu-01-sources` (ancien paquet NON
EXÉCUTÉ), `gpu-01-sources-v2` (paquet exécuté), `gpu-01`, les états des services
avant/après et `validations-gpu-01/validation-bvdguacm/`. Le consolidé local
`analyse-gpu01.json` et son script sont conservés. Ne pas publier le reçu privé
d’autorité ni ses jetons. Sources et rouges antérieurs restent conservés.

- CPU : 8 tests fournisseur, 4 tests entrée privée, 10 tests contrôleur passés ;
  les 10 derniers rejoués après adaptation à l’autorité commune. Deux scénarios
  interprocessus CPU verts. Aucun nouveau test lancé pour cette passation.
- GPU réel : 12 novembre 23:54:31–37 CET, 5,866447 s, verdict commun VERT,
  aucun résidu, espace privé nettoyé. Deux fournisseurs réels granite107m/384,
  chauffe complète 16/16 en 1226 puis 1170 ms. PCI04 confirmé par fdinfo et gfx
  positif ; pas de FD PCI07 observé. VRAM +2,766 Gio puis baseline exacte.
  Services partagés identiques avant/après. Échantillons uniques, pas p95.
- Runtime utilisé : `345a402d099ae2c345e4b37a96159df970a7ee83` ; copies immuables
  sur luciepc `~/.cache/embarquement-notes/activation/bin-345a402d0/` avec SOURCE,
  SHA256SUMS et dépendances. Le protocole et le rapport portent les empreintes.
- Le target Embarquement avait dépassé 32 Gio avant NAV : arrêt rouge conservé,
  puis requalification complète annoncée verte `4866e6fee` ; ce ne sont pas des
  tests Banc supplémentaires et leur dossier de référence appartient au lot
  Embarquement. Ne pas transformer le premier arrêt en réussite.
- NON JOUÉS : crash du demandeur, annulation pendant inférence, TTL, éviction,
  activation distante, baux distribués ; aucune preuve de toute la vision.
  Gemini Decision1 non joué. Les e2e supplémentaires d’Embarquement sont à
  retrouver dans sa propre passation, pas présumés terminés ici.

## Ressources et décisions pour la reprise

Le contrôle de passation sur luciepc retrouve absents les anciens PID privés
1097621/1097628/1097629/1097632/1098327/1098381/1098403/1098406 et aucun listener
sur le port privé 48329. Le verrou de notre mesure avait été libéré après le
banc ; aucune réservation Banc n’est maintenue. Cela ne prouve pas l’absence de
verrous ou de nouveaux processus des autres sessions. Aucun arrêt de service
partagé effectué pour cette passation.

Card2 PCI04, ROCm0 : carte normale des sessions, capacité à relire et coordonner
avant usage. `ROCR_VISIBLE_DEVICES=0` explicite ; l’ordre Vulkan est indépendant.
Card0 PCI07/seat1 : préserver les jeux ; contexte minuscule stable acceptable,
aucun poids/cache/calcul lourd sans autorisation actuelle. L’autorisation de
nuit a expiré et la clôture documentaire est annoncée `fa4dc3500`, sans preuve
de déchargement réel puisqu’aucun modèle n’y avait été enregistré cette nuit.
Les anciens états des ports 7878/7879 actifs et 7880/7882 froids sont historiques,
à relire avant toute nouvelle utilisation, jamais une autorisation de les arrêter.

Toute nouvelle validation autorisée doit passer par la primitive commune
`confiner_validation.py lancer`, sans poste extérieur, scope et target privés
plafonnés à 32 Gio, j8 maximum et preuves neuves. Pas de relance implicite dans
cette passation. Pas de secrets, force-push, prune, GC ou suppression forcée.
Avant suppression ultérieure d’un worktree : archive fidèle de son administration
Git hors cible et garde_worktree VERT avec quiescence ; conserver le rapport.

## Trois prochains pas, après reprise explicite

1. Lire les passations C7/Embarquement, vérifier l’intégration réelle et les
   branches, sans calcul :
   `git fetch origin` puis `git log -1 origin/cx-banc-residence-appels` et
   `git log origin/master -- docs/banc/12-11-2026/03-resultats-residence-privee.md`.
2. Faire reprendre par le propriétaire le seul delta encore utile, après revue :
   `git show f2d9bfbbb2a180e9d29f2b24e4c4482fe57c08bb --stat` pour le rapport ;
   examiner séparément les fichiers harness de la branche avant tout cherry-pick.
   Ne pas fusionner l’ancien runtime ni fermer le ticket commun au seul titre du banc.
3. Choisir avec l’orchestration le prochain besoin effectif. Relire d’abord les
   preuves, sans rejouer :
   `cat ~/.cache/rag3db-banc-notes/residence-nuit-12nov/analyse-gpu01.json`.
   Un futur calcul exige une nouvelle coordination et des ressources fraîches ;
   aucun ancien GO, horaire de nuit ou délai ne vaut pour le week-end.
