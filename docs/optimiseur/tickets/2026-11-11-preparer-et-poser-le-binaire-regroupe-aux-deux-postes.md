# Préparer et poser le binaire regroupé aux deux postes

- **État** : ouvert, en attente de la cible regroupée convenue avec l’orchestration
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
