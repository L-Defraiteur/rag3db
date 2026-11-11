# L’archiviste : des alias courts, des noms de fiches intacts

## Lot et périmètre

Branche de travail publiée : `cx-memoire-alias-candidats` ; témoins `7a6914ade`, correctif `9bf3bb33a`. Base jouée : `133b9013e`. Aucun push de code sur master n’est autorisé à ce stade.

Le gabarit `memory` présente chaque candidate au modèle avec un identifiant court `candidate_N`, son nom exact et son `about`. Une liste de correspondances accompagne la section jusqu’à `archive_decided`, qui retrouve le nom exact avant de composer l’apport, sa clé et ses liens. La fiche ne change pas d’identité. `verdict` garde le nom résolu pour une candidate, et les décisions fixes `none` et `create` gardent leur comportement.

Une fiche dont le nom est `none`, `create` ou `candidate_1` reste sélectionnable par son alias. Les noms dupliqués dans la recherche ne créent qu’un choix. Un alias inconnu ou plusieurs correspondances pour le même alias provoquent un refus explicite ; aucun choix arbitraire ni écriture sous un nom d’alias.

Les expériences précédentes modifient les valeurs enum et la présentation des choix ; elles ne prouvent ni un seuil Vertex ni un champ invalide précis.

## Rouge puis vert ciblé

À luciepc, sous `poste lourd`, huit tâches au maximum, jeu large `rag3db-native,burn-embedder,burn-ocr,code,daemon,postgres,openai-llm,anthropic-llm` et lib complète `rag3db-build-29a7bc52aaf11e6204f9beafaffb154105417e13-Release-p12-v11-g17-s18-rcOFF-stOFF-asanOFF-tsanOFF-ubsanOFF` :

1. Gabarit original et premier témoin : **0 passé, 1 échoué**, pour la bonne raison. La liste attend les alias courts mais reçoit les noms originaux, dont le bilan long et les noms réservés. Journal `~/.cache/rag3weaver-build/memoire-alias-rouge.log`.
2. Correctif et suite d’archivage : **5 passés, 0 échoué, 0 ignoré**, 34,70 s. Les trois nouveaux témoins vérifient la résolution, les refus et les décisions fixes ; les deux témoins existants vérifient la conservation de la Source, les apports, les liens et le cas sans modèle. La requête du faux serveur vérifie que seules les étiquettes courtes passent dans l’enum et que le nom reste dans le texte. Journal `~/.cache/rag3weaver-build/memoire-alias-vert.log`.

Ce vert ciblé ne constitue pas la passe complète.

## Passe historique et essai sur copie

La passe historique a terminé verte sous l’unité `cx-memoire-alias-passe` : cinq jeux de features, lib entière, étape navigateur, `e2e_archiviste`, `e2e_postgres`. Résultats : features **5/5** ; lib **2 094 passés, 0 échoué, 58 ignorés** ; navigateur **33 passés, 0 échoué, 0 ignoré** ; archivage **5 passés, 0 échoué, 0 ignoré** ; PostgreSQL **34 passés, 0 échoué, 0 ignoré**. Les deux suites e2e ont aussi joué leurs deux gardes préalables, verts. Verdict `cx-memoire-alias-verdict` : `VERT`. Aucun échec n’a été masqué. Journaux `~/.cache/rag3weaver-build/cx-memoire-alias-*` à luciepc.

Le premier document réel est aussi rejoué sur une copie neuve de l’instantané, avec les deux scripts exacts du lot et le binaire certifié d1345a115 fourni par optimiseur (SHA256 `597f6a29de486a9161996e8e4f460731802e090c71f0859a7ad80808ad7da7ee`). Résultat : **7/7 sections, zéro erreur, zéro requête refusée**, 331,5 s pour le processus entier. Ce temps n’est pas une mesure exclusive ni une durée par section ; aucune cause de sa variation n’est établie. Son dossier est `validation-correctif-alias/` sous la racine temporaire des essais de la [note précédente](03-reprise-codex-archivage.md). Les maps du processus ont confirmé la lib épinglée, dans `lib-chargee.txt`. Le résultat est consigné dans `resultats.jsonl` ; aucune durée par section n’est mesurée.

Les cinquante documents ne sont pas lancés et la mémoire réelle est inchangée. Le ticket est déplacé dans `docs/tickets/closed/` dans le lot du correctif ; sa livraison sur master reste en attente.

## Rejeu requis après rebase

Master a reçu `1904e04e42cd3666d6b3a64fee5bd51f0629c020`, un garde-fou Rust dans les tests de backend. Après la passe historique, rebaser sous un nom de branche cx- neuf, pousser la branche, relire le diff puis rejouer features, lib entière et suites du changement hors verrou push. Les journaux historiques restent intacts ; le script préparé pour ce rejeu utilise le préfixe `cx-memoire-alias-r2-`. Aucun tour de push n’est accordé à ce stade.

## Où reprendre

- Portable : `~/.cache/rag3weaver-build/cx-wt-memoire-alias`, branche `cx-memoire-alias-candidats`.
- luciepc : `~/git_workspaces/cx-rag3db-memoire-alias`, branche `cx-memoire-alias-passe`, propre sur `9bf3bb33a` au lancement ; script `~/.cache/rag3weaver-build/cx-memoire-alias-passe.sh`.
- Target unique utilisé : `~/.cache/rag3weaver-build/target-96-fiches4` (17 Go à la dernière lecture, plafond 32 Go).
- L’ancien miroir `~/git_workspaces/rag3db-memoire-alias` a seulement joué le rouge. Son unité est terminée. Git a refusé son déplacement car il contient un sous-module ; le miroir `cx-` a été créé neuf. Une remise de sécurité dans le miroir `cx-` conserve les trois fichiers joués avant passage au commit publié (`cx-memoire-alias-source-jouee`).
- Les anciennes branches distantes restent ; aucune suppression distante ni réécriture de commit poussé.
