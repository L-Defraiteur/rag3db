# Mesurer le juge local avec dix fiches

- **État** : ouvert — protocole préparé, accord card0 attendu.
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
Vérifier que le port 8099 est libre. Card0 seule avec `ROCR_VISIBLE_DEVICES=1`,
en laissant au moins la moitié de sa VRAM libre ; arrêter les seuls processus
lancés par ce banc. Aucun calcul GPU n’a été lancé lors de cette reprise.

## Attente et reste à faire

L’accord de Lucie pour card0 est attendu, conformément au knowledge dump du banc,
section 1. La suppression des confirmations techniques ne remplace pas cet accord.
L’orchestration a demandé de préparer seulement le protocole dans cette attente.
Le ticket ne donne aucune autorisation d’activation ni de push de code.

Après accord : construire et contrôler les 880 paires, jouer les mesures,
analyser les résultats et compléter la page GGUF pour la session mémoire.

## Pour fermer

Résultats k = 5 et k = 10 réellement joués, commandes et preuves conservées,
page GGUF actualisée avec les limites du jeu. Déplacer ce ticket vers
`docs/tickets/closed/` et inscrire son état de clôture avec la preuve dans le même commit.
