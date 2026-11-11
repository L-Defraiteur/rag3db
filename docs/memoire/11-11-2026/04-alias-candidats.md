# L’archiviste : des alias courts, des noms de fiches intacts

## Lot et périmètre

**Livré sur master : `41526b29a79e7137a315ac81bb9c5a5b0a6b83a9`**, au tour donné par l’orchestration. Branche distante de livraison : `cx-memoire-alias-integration-3` ; les branches de préparation restent sur origin. Témoins initiaux `7a6914ade`, correctif initial `9bf3bb33a`, première base jouée `133b9013e`.

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

Le premier document réel est aussi rejoué sur une copie neuve de l’instantané, avec les deux scripts exacts du lot et le binaire certifié d1345a115 fourni par optimiseur (SHA256 `597f6a29de486a9161996e8e4f460731802e090c71f0859a7ad80808ad7da7ee`). Résultat : **7/7 sections, zéro erreur, zéro requête refusée**, 331,5 s pour le processus entier. Ce temps n’est pas une mesure exclusive ni une durée par section ; aucune cause de sa variation n’est établie. Son dossier est `validation-correctif-alias/` sous la racine temporaire des essais de la [note précédente](03-reprise-archivage.md). Les maps du processus ont confirmé la lib épinglée, dans `lib-chargee.txt`. Le résultat est consigné dans `resultats.jsonl` ; aucune durée par section n’est mesurée.

Les cinquante documents ne sont pas lancés et la mémoire réelle est inchangée. Le ticket est déplacé dans `docs/tickets/closed/` dans le lot du correctif ; sa livraison sur master est faite sous `41526b29a`.

## Rejeu après rebase

Master a reçu `1904e04e42cd3666d6b3a64fee5bd51f0629c020`, un garde-fou Rust dans les tests de backend. Après la passe historique, le lot a été rebasé sur `0b488c0fd8deedfe78ac8aa1c0aaca0249055fa7`, sous le nom neuf `cx-memoire-alias-candidats-2`, puis poussé. Le diff du rebase a été relu : seul le témoin de `backend.rs` est du Rust nouveau, le reste est documentaire. Le correctif rebasé est `c481a66c6` ; le commit joué est `6f13217aaa775c16606e79295c5441fd51dd8e22`. Les commits ultérieurs ne changent que la documentation.

Le rejeu court est **VERT** : features **5/5** ; lib **2 095 passés, 0 échoué, 58 ignorés**, 59,31 s ; archivage **5 passés, 0 échoué, 0 ignoré**, 36,61 s ; PostgreSQL **34 passés, 0 échoué, 0 ignoré**, 9,59 s. Chaque suite e2e a aussi joué ses deux gardes préalables, verts. Le contrôle des tickets est vert et annonce la fermeture de ce ticket. La lib et le jeu de features sont ceux nommés plus haut. Les journaux historiques restent intacts ; les journaux de ce rejeu utilisent le préfixe `cx-memoire-alias-r2-`. Le tour a ensuite été donné pour ce lot seul.

## Livraison et nettoyage

Sous `poste push`, master a été relu à `2870f3e81` : seulement des documents depuis la base rejouée. Le lot a été rebasé sous le nom neuf `cx-memoire-alias-integration-3`, sauvegardé sur origin, puis poussé dans `if grep -qx VERT`. Les deux scripts, le graphe et les témoins sont vérifiés octet pour octet identiques à ceux du commit rejoué `6f13217aa`.

Après fetch, les trois contrôles ont été lus : **status propre**, **aucun rebase en cours**, **origin/master = HEAD = `41526b29a79e7137a315ac81bb9c5a5b0a6b83a9`**. Journal externe : `~/.cache/rag3weaver-build/cx-memoire-alias-integration.log` à luciepc ; hash dans `cx-memoire-alias-pousse.hash`.

Les processus du lot étaient inactifs au nettoyage. Le script a supprimé le worktree de passe `~/git_workspaces/cx-rag3db-memoire-alias`, ses branches locales et le target unique `~/.cache/rag3weaver-build/target-96-fiches4`. Le miroir du rouge `~/git_workspaces/rag3db-memoire-alias` a été supprimé après vérification que son seul fichier modifié est celui de `7a6914ade` et sauvegarde externe du diff (`cx-memoire-alias-rouge-source.diff`). Sur le portable, `~/.cache/rag3weaver-build/cx-wt-memoire-alias` et les branches locales de ce lot ont été supprimés ; aucun target n’y avait été créé pour ce lot. Toutes les branches distantes restent.

Pour reprendre, créer un worktree et une branche `cx-` neufs depuis master. Les journaux ciblés, complets et r2 restent dans `~/.cache/rag3weaver-build/` à luciepc ; les essais sur copie restent dans la racine temporaire décrite plus haut. Ces essais ne valent pas une validation du classement sémantique de cinquante documents. **Les cinquante documents ne sont pas lancés et aucune écriture n’est faite dans la mémoire réelle.**
