# Préparer et poser le binaire regroupé aux deux postes

- **État** : fait, pose vérifiée de 68b9ede17c182487d3f823c2e5cc5906b62a4a46 aux deux postes le 11 novembre 2026 ; preuves et heures ci-dessous, clôture portée par le commit qui déplace ce ticket
- **Famille** : produit
- **Gravité** : planification — rendre les lots intégrés accessibles aux deux postes
- **Atteignable en service** : oui, le binaire installé détermine les fonctions disponibles
- **Touche rag3weaver** : oui, préparation et pose de l’hôte
- **Pour** : optimiseur ; orchestration pour la cible et le créneau
- **Signalé par** : reprise de session, `docs/optimiseur/11-11-2026/01-rapport-de-session.md`, et consignes d’orchestration du 11 novembre

## Demande et périmètre

Préparer une seule cible regroupée, puis poser le binaire à luciepc et au portable selon
`docs/optimiseur/scripts/` et la coordination de l’orchestration. Ce ticket ne donne
aucune autorisation de coupure ni de modification des ressources d’autres sessions.

## Preuves déjà obtenues

Le binaire historique d1345a115a8044c5b5f5947e26d5aaba8d340053 a été bâti en release
et essayé sur les deux postes : hôtes d’essai VERT, check-lib à zéro, deux manifestes
acceptés à luciepc et six au portable. SHA256 du binaire identique :
`597f6a29de486a9161996e8e4f460731802e090c71f0859a7ad80808ad7da7ee`.
Preuve portable : `~/.cache/optimiseur-notes/preparation-d1345a115.log`.
Artefacts luciepc : `~/.cache/rag3weaver-build/nuit-d1345a115a8044c5b5f5947e26d5aaba8d340053/`.
Aucune passe e2e supplémentaire ni pose effectuée par cette session.

Cette cible est dépassée : l’orchestration interdit sa pose et attend une cible
regroupant notamment les lots suivants. Les artefacts restent conservés pour référence.
La lib annoncée reste
`rag3db-build-29a7bc52aaf11e6204f9beafaffb154105417e13-Release-p12-v11-g17-s18-rcOFF-stOFF-asanOFF-tsanOFF-ubsanOFF`.

## Attestation des ressources embarquées

Le défaut de target repris appartient à recherche :
`docs/recherche/tickets/2026-11-11-un-target-repris-embarque-les-gabarits-d-un-autre-worktree.md`.
Ne pas dupliquer son correctif. Les preuves R5 ont été lues sans toucher au target actif :
`~/.cache/rag3weaver-build/journaux-cx-recherche-arret-11nov-r5/diagnostic-bundle.txt`
à luciepc montre les deux ressources UI d’un autre worktree dans le binaire navigateur.

Pour la prochaine préparation, comparer les octets réellement servis par GET
`/ide/app.js` à `git show <cible>:extension/rag3weaver/templates/ide/app/app.js` ;
attester aussi `templates/ide/app/bundle.json` embarqué et vérifier la provenance
des chemins `include_bytes!` du fichier généré dans OUT_DIR. Le hash du binaire et
check-lib seuls ne prouvent pas que les gabarits appartiennent au commit annoncé.
Ces contrôles restent à réaliser sur la prochaine cible.

## Reste à faire et critères de fin

1. Recevoir le hash complet cible et la coordination de l’orchestration ; ne pas rebâtir chaque arrivée documentaire.
2. Vérifier les ressources, préparer sous poste lourd avec huit jobs, conserver les journaux et les empreintes.
3. Jouer les vérifications prescrites, dont hôte d’essai, manifestes et attestation UI servie/embarquée sur les deux postes.
4. Après coordination, préavis d’une minute pour binaire seul, poser luciepc puis portable ; contrôler réveil, bases, IDE et outils, avec retour conservé.
5. Rapporter hash, empreintes, heures et preuves. Fermer et déplacer ce ticket seulement après la pose vérifiée ou un abandon explicite documenté.

## Préparation regroupée du 11 novembre, 21 h 55–21 h 58

Cible convenue : `68b9ede17c182487d3f823c2e5cc5906b62a4a46`, contenant les lots
arrêt, lucivy 4.3.3, outils d’atelier et UI contextuelle. Bâti release sous poste lourd,
huit jobs, en 1 min 01 s, worktree détaché `~/git_workspaces/cx-optimiseur-controle`
à luciepc. L’ancien worktree `rag3db-controle-G` reste intact : git refuse de déplacer
un worktree contenant des sous-modules. Target incrémental : `target-G/hote-lucie`.

SHA256 identiques aux deux postes :

- hôte : `6af3ec67450863021735359132b8d68e2afb97a8e0c50f2b182fd94a0bc44e2e` ;
- check-lib : `888679312c87f6d5b78785a7c5de202671df6f68679b121c416dabe5f8f4853d`.

Artefacts : `~/.cache/rag3weaver-build/nuit-68b9ede17c182487d3f823c2e5cc5906b62a4a46/`
à luciepc ; `~/.local/bin/rag3weaver-backend.nouveau` et `rag3db-check-lib.nouveau`
au portable. Ils ne sont pas installés dans les chemins actifs.

Contrôles effectivement joués aux deux postes : hôte isolé 7991, `/sante`, check-lib
à zéro, aucune bibliothèque manquante, lucivy-core 4.3.3, lib complète 29a7 lue
dans les maps, arrêt du seul PID d’essai. Deux manifestes acceptés à luciepc,
six au portable, zéro refus. Pas de nouvelle passe e2e globale jouée par cette session.

Attestation UI : octets GET `/ide/app.js` et `/ide/app.css` strictement égaux au
commit cible ; `bundle.json` de ce commit présent dans l’ELF et ses empreintes
cohérentes avec les deux ressources servies. `bundle.json` n’a pas été présenté
comme une route HTTP testée.

- app.js : `fff528ffdcbfb08c022c1249d0ec5ecc82a4caf4a010eaad4447ca7c3081bc00` ;
- app.css : `f401d17501eb227e317ca8a40e83c0a41ff2eb4516528ae3ce29cec17968be50` ;
- bundle.json : `6d317a42f06655af8d59d934bd6bb4ac4aba7be82794365e1c6a6e027f3fba43`.

Le nouveau fichier généré `release/build/rag3weaver-6d2079498d1ec81f/out/builtin_templates.rs`
contient 220 inclusions, toutes issues de `cx-optimiseur-controle`. Les anciens
fichiers générés restent présents pour les anciens fingerprints ; ils ne constituent
pas l’attestation du nouveau binaire.

Preuves : `~/.cache/rag3weaver-build/cx-bati-68b9ede17.log` à luciepc ;
`~/.cache/optimiseur-notes/attestation-68-82ly9ely/resultat.json` à luciepc ;
`~/.cache/optimiseur-notes/attestation-68-aewd4xt6/resultat.json`,
`essai-68-portable.log` et `manifestes-68-portable.log` dans le même dossier de notes
au portable. Script d’attestation conservé `attester-68.py` dans ces notes.

## Plan de pose à relire avant autorisation

Aucune coupure, aucun instantané neuf, aucune réouverture de base réelle et aucune
modification des manifestes réels effectués pendant cette préparation.

La lib et son format ne changent pas. La procédure existante de binaire seul ne
demande pas un nouvel instantané de bases ; les instantanés antérieurs restent
conservés. Avant une pose, confirmer avec l’orchestration si un nouvel instantané
est requis : dans ce cas, arrêt propre puis reflink de chaque base avec tous ses
fichiers associés, recette RETOUR.sh, avant tout réveil. Ne pas prendre une copie
à chaud pour un instantané cohérent et ne pas restaurer une ancienne base sans
décision explicite : cela peut retirer des écritures récentes.

Retour binaire : conserver les deux exécutables actifs sous `.avant-68b9ede1`,
ainsi que les réglages et le filet, sans en afficher les secrets. Conserver aussi
avant réveil les manifestes concernés et leurs notes de livraison : un rafraîchissement
des gabarits peut survivre à un simple retour du binaire. Si écart, arrêter proprement,
remettre anciens exécutables/réglages/manifestes et notes sauvegardés, puis réveiller
et contrôler ; aucune restauration de base automatique.

Atelier principal au portable : note de livraison présente, manifeste actuel avec
11 outils (SHA256 `b06e821f92c4bf5a5940bde558d58c37f4b7004aefbf6350b87fe92b8adb1a6c`).
Le gabarit cible en livre 20, dont search_code, outline, usages, callees, impact,
grep_files, list_files, schema et wait_output. Vérifier après réveil le manifeste
effectivement rafraîchi ET les outils exposés, puis le parcours IDE. L’atelier
d’essai reste à trois outils ; son ancien refus de réinstallation reste réservé
à Lucie et ne sera pas contourné par cette pose.

Ordre proposé : revue des attestations et du retour, coordination et préavis
d’une minute, luciepc puis portable, contrôle des backends, UI servie et outils
réels, bilan avec heures et empreintes. La préparation verte ne vaut pas accord de pose.

## Réalisation : pose autorisée et contrôlée

L’orchestration a relu les deux attestations et autorisé la pose de binaire seul
après 22 h 03 Paris, préavis donné à Lucie à 22 h 01. Pas d’instantané de base neuf
requis : lib et format inchangés. Les étapes ci-dessous ont été réalisées, distinctes
du plan historique conservé plus haut.

| Poste | Début arrêt socket/service | Réveil contrôlé | MainPID |
|---|---|---|---|
| luciepc | 22:03:08.940 | 22:03:11.708 | 3788236 |
| portable | 22:03:56.736 | 22:04:04.529 | 4167580 |

Les trois services d’embarquement de chaque poste ont répondu avec leurs dimensions
avant coupure. Activité 7970 relue juste avant chaque arrêt : aucun chat en cours
ni run de fond. Au portable, la tâche `8fa380f6a09c` était en attente sans attente de
réponse ; l’orchestration a confirmé que cet état seul n’interdisait pas la pose.
Arrêt systemd propre, aucun autre processus tué ni force-kill.

Sauvegardes réalisées hôte arrêté, avant tout rafraîchissement de gabarit : anciens
exécutables `.avant-68b9ede1`, fichiers JSON/MMD de configuration, notes de livraison
et copie du filet. Sur chaque poste : `~/.cache/optimiseur-notes/pose-68/config/`,
`notes-manifestes/` et `retour.json` (chemins et SHA des anciens exécutables).
Env et filet actifs restent inchangés : aucun besoin de lire/réécrire l’env ni de
réinstaller l’unité avec la même lib. Aucun fichier de base restauré ou remplacé.

Contrôles après réveil : SHA256 de `/proc/<MainPID>/exe` égal à
`6af3ec67450863021735359132b8d68e2afb97a8e0c50f2b182fd94a0bc44e2e` aux deux postes,
PID de `/sante` égal au MainPID, lib complète 29a7 dans les maps et check-lib vert.
Deux bases de luciepc et cinq backends du portable rouverts avec `ok=true`.
Les GET authentifiés de l’UI active correspondent aux SHA app.js/css attestés.

L’atelier principal expose exactement les 20 outils du gabarit cible, dont
`search_code`, `outline`, `usages`, `callees`, `impact`, `grep_files`, `list_files`,
`schema` et `wait_output`. Cela prouve leur exposition, pas l’exécution de chaque
outil sur chaque base. L’atelier d’essai n’a pas été réinstallé : trois outils et
SHA `f563123a5bc5ab47d83ab032d95e95ffd6158d2212e913d7fb298d7e7b7b61f7` inchangés.
Les sept tâches gardent les mêmes identifiants et états avant/après ; la tâche
`8fa380f6a09c` reste en attente, sans consommation manuelle de son travail.

Rendu Chrome sans tête au portable : page `/ide/` rendue, 431 éléments sous la
racine React, aucune erreur ni avertissement de console. Pas de preuve de clics
ou d’exécution de tous les outils ajoutée à cette vérification de pose.

Preuves conservées :

- luciepc : `~/.cache/rag3weaver-build/cx-pose-68.log` et `cx-controle-pose-68.log` ;
- portable : `~/.cache/optimiseur-notes/pose-68-portable.log` et `controle-pose-68-portable.log` ;
- aux deux postes : `~/.cache/optimiseur-notes/pose-68/resultat-reveil.json` et `controle-fonctionnel.json` ;
- portable : même dossier `etat-avant.json`, `etat-apres.json`, `rendu-ide.log` ;
- scripts exacts conservés dans les notes : `pose-68.py`, `controler-pose-68.py` au portable,
  copies `cx-pose-68.py`, `cx-controler-pose-68.py` dans le cache de bâtis de luciepc.

Bilan heures, SHA et 20 outils transmis à l’orchestration avant clôture. Retour
non nécessaire ; les anciens exécutables, sauvegardes et artefacts restent conservés.

## Nettoyage après publication des preuves

Clôture et rapport poussés en `a83fa83e22beaa15e7be1306962402605e2382e9` ;
contrôles lus : worktree propre, aucun rebase, HEAD égal à origin/master.
Sur instruction d’orchestration, worktree de préparation `cx-optimiseur-controle`
et target `target-G/hote-lucie` retirés à luciepc entre 22:06:58 et 22:07:00,
après vérification d’absence de processus utilisant le target. Le worktree de
documentation `cx-optimiseur-suivi-pose` a également été retiré, sans supprimer
de branche distante. L’ancien `rag3db-controle-G` reste intact.

Avant retrait : source exacte des gabarits archivée dans
`~/.cache/optimiseur-notes/pose-68/templates-source-68.tar`, fichier généré copié
dans `builtin_templates.rs` au même endroit. Journal : `nettoyage.log`.
Artefacts nuit, sauvegardes privées, anciens exécutables et preuves conservés ;
aucune base ni instantané supprimé. Les chemins de préparation mentionnés plus
haut décrivent le bâti réalisé et ne sont plus des ressources actives à reprendre.
