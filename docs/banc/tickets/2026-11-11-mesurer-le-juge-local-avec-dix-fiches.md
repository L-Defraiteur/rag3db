# Mesurer le juge local avec dix fiches

- **État** : ouvert — protocole préparé ; coordination card2 en cours.
- **Famille** : produit
- **Gravité** : coût — latence et qualité du classement à dix fiches non mesurées.
- **Atteignable en service** : non pour cette mesure ; le branchement du juge revient à la session mémoire.
- **Touche rag3weaver** : oui, choix du juge du rappel de mémoire.
- **Pour** : banc
- **Ouvert le** : 11 novembre 2026

## Travail et preuves

Le rapport `docs/banc/11-11-2026/01-rapport-de-session.md`, sections 6 et 7,
demande la mesure k = 10. La page `docs/banc/11-11-2026/03-le-juge-en-gguf.md`
ne donne pour dix fiches qu’une estimation de 1,5 s. À cinq fiches, elle rapporte
0,76 de bonnes premières et 1 076 ms par cas en Q8_0 avec cache ; ces résultats
historiques n’ont pas été rejoués lors de cette reprise.

Lecture locale vérifiée le 11 novembre : `paires2-2000.jsonl` contient 440 paires ;
`paires2.json` décrit 88 cas et 88 fiches distinctes, avec 88 positifs,
176 négatifs proches et 176 négatifs aléatoires. Ces fichiers vivent dans
`~/.cache/rag3db-banc-notes/decision/` du portable.

## Protocole à exécuter

Conserver les cinq candidats de chaque cas et ajouter cinq négatifs distincts
d’autres sessions, avec la graine 20261108. Conserver contextes à 2 000 jetons
et partitions de test ; figer le manifeste et l’ordre avant toute mesure.
Cette extension mesure un groupe construit, pas la distribution d’un rappel réel.

Comparer cinq et dix candidats sous Q8_0 avec `cache_prompt`, groupes contigus :
bonne fiche première, présence dans les deux premières, AUC intra-cas,
latences par cas p50/p95, cache effectivement réutilisé et VRAM avant/après.
Garder les résultats par cas et les commandes dans des fichiers neufs.

Utiliser le llama.cpp b9590 déjà bâti à luciepc, sans le rebâtir, sous `poste lourd`.
Vérifier que le port 8099 est libre. Consigne de Lucie du 12 novembre :
card0/seat1 (PCI 0000:07:00.0, ROCm GPU[1]) est interdite aux sessions.
Utiliser exclusivement card2 (PCI 0000:04:00.0, ROCm GPU[0]) avec
`ROCR_VISIBLE_DEVICES=0` explicite. Lire sa VRAM avant le banc et coordonner
la charge avec les autres sessions. Les quatre services 7878/7879/7880/7882
occupent environ 25,9 Go selon le relevé transmis par Lucie. Un cinquième
processus GPU a provoqué un reset de ring le 9 novembre : ne pas ajouter
le banc aux quatre services. Coordonner avec le porteur du service privé 7882
son arrêt temporaire et sa remise en route, puis relire la VRAM avant chargement.
Ne jamais utiliser card0 comme débordement. Arrêter les seuls processus lancés
par ce banc. Aucun calcul GPU n’a été lancé lors de cette reprise.

## Attente et reste à faire

Lucie autorise le banc sur card2 le 12 novembre sous réserve de coordination
avec les autres sessions et de capacité disponible. Cette consigne remplace
l’ancienne attente d’accord card0. La disponibilité de 7882 et la fenêtre de
charge restent à convenir avec l’orchestration ; aucun tour de push de code accordé.

Construire et contrôler les 880 paires, puis, dans la fenêtre convenue, jouer les mesures,
analyser les résultats et compléter la page GGUF pour la session mémoire.

## Pour fermer

Résultats k = 5 et k = 10 réellement joués, commandes et preuves conservées,
page GGUF actualisée avec les limites du jeu. Déplacer ce ticket vers
`docs/tickets/closed/` et inscrire son état de clôture avec la preuve dans le même commit.
