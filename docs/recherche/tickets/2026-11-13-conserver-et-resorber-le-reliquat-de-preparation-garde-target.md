# Conserver et résorber le reliquat de préparation garde target

- **État** : ouvert (13 novembre 2026), prototype archivé et W retirée ; trois références refusées-d conservées avec leur motif, sans suppression forcée.
- **Famille** : produit
- **Gravité** : conservation de travail local et de références après retrait des ressources qualifiées ; aucun défaut runtime nouveau établi.
- **Atteignable en service** : non, préparation documentaire hors source de service.
- **Touche rag3weaver** : oui, worktrees et historique du lot garde.
- **Pour** : recherche
- **Signalé par** : Recherche/C7 après intégration garde9feb et nettoyage autorisé.

## Inventaire RO avant toute action de résorption

W local `~/.cache/rag3weaver-build/cx-wt-recherche-garde-target-union-pilote`,
branche `cx-recherche-garde-target-union-pilote-r1`, HEAD3b259fbce8f61b2f1cc1bad27dedf43560298386.
Seuls cinq fichiers non suivis sous docs/recherche/13-11-2026/garde-target-union :
attester.py, driver.py, finaliser.py, lancer.py et sdk-exec.py. Aucun changement
suivi ni fichier ignoré ; fichiers réguliers UID1000, préparation créée par
Recherche avant pilote438 puis abandonnée au profit du pilote-r2. Aucune
ressource Lucie/autre lot attribuée à ce prototype. Aucun .env/.vault ne sera
lu/copier ; présence éventuelle d'une telle zone ferait conserver la W.

Branches locales conservées après seul `git branch -d` :
cx-recherche-garde-target (698), cx-recherche-garde-target-pilote-r1 (bdff),
cx-recherche-garde-target-reprise-r1 (a5e). Git les dit non complètement
fusionnées (histoires sources d'avant rebase) ; leurs branches distantes et
archives administratives demeurent. Aucun `-D` autorisé ni nécessaire.

## Résorption proposée dans le périmètre C7

1. Sauvegarde fidèle des cinq fichiers non suivis et de toute donnée ignorée
   constatée, en dehors de W/T, avec modes/UID/GID/mtime/SHA comparés deuxpostes.
2. Publier le prototype sur une branche cx d'archive isolée NON FUSIONNÉE,
   sans modifier ces fichiers ni prétendre recette exécutable qualifiée.
   Ce commit d'archive permet la propreté sans reset/clean/effacement caché.
3. Nouvelle archive administrative fidèle deuxpostes après ce commit, scan
   de quiescence puis garde COURANTE6f3 et rapport fraisVERTLU avant retrait.
   Tout refus conserve la W. Branche distante d'archive reste disponible.

Les trois refus-d n'autorisent aucune suppression forcée de branche ; conserver
leurs références avec motif tant qu'un arbitrage distinct ne les retire pas.
Le ticket garde déjà corrigé reste clos. Aucun Cargo/rustc/service/runtime,
aucune autre ressource Recherche ou autre lot à rejouer/nettoyer par ce suivi.

## Résultat de l'archivage et du retrait du prototype

Sauvegarde5fichiers identiques modes/UID/GID/mtime/size/SHA sur deuxpostes,
`cx-archive-prototype-garde-13nov`. Commit d'archive :
SHA complet ccb3d90c416bbde37ee8f65176c017207de5f265, branche
cx-recherche-garde-target-prototype-archive-r1 publiée NONMERGÉE, aucun changement
des cinq fichiers et aucun runtime. Nouveau commit rend la W propre sans
reset/clean. Nouvelle archive administrative fidèle deuxpostes après cecommit,
quiescence[] et rapport6f3fraîcheVERTLU : Wprototype retirée normalement.
L'ancienne Wbilan97 retirée de même. Traces/archives complémentaires
`cx-archives-garde-target-reliquat-13nov` sur deuxpostes.

Les trois références historiques restent : refus-d « non complètement
fusionnée » (698/bdff/a5e non ancêtres du master rebasé), sans W ni target.
Archivesfidèles et branchesdistantes les rendent retrouvables. Aucun usage
mémoire/disque de build associé, aucun-D/contournement proposé. Ticket garde
reste clos ; le présent suivi reste ouvert tant que ces références sont
conservées ou qu'un arbitrage les classe sans suppression.
