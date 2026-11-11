# Reprise du lot require : exports par défaut

## Périmètre et preuves

Le lot `codeparsers/require-js` reste différé. La branche isolée
`codeparsers/require-temoins` part de 280b5a3f1 et porte les témoins 5767adf35,
le correctif 4d479816b et le témoin entre lots e1015a379. Son sous-module pointe
sur `e48b41499782590e0181a7a87aad19ddcd8f9093`, sauvegardé sur la branche
codeparsers `require-default-metadata`, sans avancement de master.

Le résolveur de codeparsers expose sa lecture existante de l'export par défaut :
nom ou instance. rag3weaver offre un symbole de rendez-vous qualifié par source
et fichier, relié par DEFINES au scope nommé ou à la zone de l'instance. Une
mention importée par défaut attend ce symbole, indépendamment de son alias.
Aucune sorte de scope supplémentaire, aucun changement d'oracle, de seuil
ou de politique des modules externes.

## Les quatre cas

- Alias différent de l'export : `Alias = require('./outil.js')`, export `Outil`.
  La navigation attend la classe Outil, marquée import. Rouge initial : aucune arête.
- Instance exportée : `module.exports = new Outil()`.
  La navigation attend la zone de cet export, jamais la classe. Rouge initial :
  arête vers la classe, marquée import.
- Réexportation : la mention de l'alias dans un objet renvoyé doit naviguer vers
  la définition exportée. Rouge initial : aucune arête.
- Externe ou fichier généré absent : les mentions gardent leur repli sur la
  zone de liaison, marquée liaison. Vert avant et après le correctif.

Une navigation vers la définition exportée et une définition de liaison locale
rendue par le LSP sont deux contrats. Les attentes ci-dessus sont celles de
navigation ; aucun verdict LSP nouveau n'est revendiqué pour ces fixtures.

## Ce que dit réellement l'A/B du 9 novembre

Preuves brutes relues à luciepc dans
`~/.cache/codeparsers-notes/require-mesure-2/` (`resume.txt`,
`verdicts-otees.tsv`, `verdicts-ajoutees.tsv`).

Les 415 ajouts sont des arêtes vers une zone de liaison, toutes muettes pour
l'oracle : ni 415 scopes, ni 415 gains de précision prouvés.
Les 17 arêtes justes ôtées visaient les liaisons locales : 13 dans les index.js
(4 node, 4 sync, 5 wasm), deux reader dans collect.cjs, une rag3db_wasm générée,
une CSS dans grammar.js. Le LSP désignait exactement ces lignes de liaison.
Les remplacer par une navigation plus lointaine ne rétablit pas automatiquement
ces 17 verdicts stricts. Les résultats futurs doivent rester séparés de ce constat.

## Vérifications réellement jouées

À luciepc, sous `poste lourd`, lib
`rag3db-build-29a7bc52aaf11e6204f9beafaffb154105417e13-Release-p12-v11-g17-s18-rcOFF-stOFF-asanOFF-tsanOFF-ubsanOFF` :

- `require-temoins-rouge.log` : 4 tests joués, 3 rouges ciblés, 1 vert, 0 ignoré.
- `require-temoins-correctif.log` : 4 joués, 4 verts, 0 ignoré.
- `require-temoins-incremental.log` : 5 joués, 5 verts, 0 ignoré, dont le
  rendez-vous entre fichiers ingérés séparément dans les deux ordres.

Journaux sous `~/.cache/codeparsers-notes/`. La passe complète n'est pas jouée.
Le contrôle initial de retard rendait 195, sous le seuil 200, sans le modifier.

## Mesure suivante

Le script `outils/mesure-default-exports.sh` conserve les résultats historiques,
compare A=4f54e473a/c660893 à B=e1015a379/e48b414 sous la même lib 29a7bc52a,
et garde l'oracle strict et l'échantillon de référence.
L'optimiseur a transmis explicitement le service privé 7882 après hotfixs :
granite-278m dense768, sans creux ni relecteur. L'hôte isolé est 7978 ; seules
les copies du gabarit reçoivent embeddings.address=7882. Aucun usage des services
partagés 7878–7880. La séquence complète doit rester sous un seul poste lourd.

Aucun tour de push master accordé. Aucun résultat A/B nouveau annoncé à ce stade.

## Reprise sur master et convention cx-

Après un nouveau fetch, l'ancienne branche a 216 commits de retard : elle ne
servira pas à une nouvelle passe. Les sept commits ont été repris sans conflit
sur master 2870f3e81 dans `/home/lucied/git_workspaces/cx-wt-codeparsers-require`,
branche `cx-codeparsers-require`, tous sauvegardés sur origin.
Le témoin exact `module.exports = { Alias }` est ajouté en 846fcf778 ; il n'est
pas encore joué. Le premier témoin dit « réexportation » utilisait un objet
renvoyé par une fonction : ses verts ne prouvent pas la forme exacte des index.js.

La mesure active reste figée sur e1015a379/e48b414 et ses anciens chemins :
aucun worktree ni script n'a été déplacé sous sa compilation. Ses résultats
ne seront pas présentés comme une passe du master repris. Le rejeu des témoins,
puis la validation requise, restent à faire sur la branche cx-.

Les dix tests `require_js` de codeparsers sont également passés, zéro ignoré,
sur e48b414 (journal luciepc `require-parser-metadata.log`).

## Correction de lecture : les 415 muettes n'ont pas interrogé le LSP

Vérification supplémentaire : pour les 415 arêtes liaison historiques, le champ
nom est absent de chacune des 415 lignes sources (aucune source manquante).
L'oracle cherche ce nom de cible avant toute question : aucune interrogation
n'a donc eu lieu pour ces arêtes. « Muet » ne démontre ici aucun silence du
serveur. Même risque pour Alias vers Outil et FS vers une zone exportée.
Voir le ticket `2026-11-11-l-oracle-cherche-la-cible-au-lieu-du-nom-mentionne.md`.
L'oracle et l'A/B active restent inchangés ; ces cas ne seront pas comptés comme
gains de précision. Le modèle requis sépare mention source et cible proposée.
