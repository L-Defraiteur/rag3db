# Rapport de session — l'arbre principal, au 11 novembre 2026

Écrit pour un agent qui reprend ce périmètre sans aucune mémoire des échanges entre sessions. Les
mots de la maison sont définis la première fois, et le knowledge dump d'à côté
(`02-knowledge-dump.md`) donne les règles et les procédures en entier.

**Qui est « l'arbre principal ».** Une session Claude Code (identifiant de transcription
`1d45e3cc-0d1d-4bcf-8a52-155963c3d3fc`), nommée ainsi entre sessions, qui tient : le juge des runs, le
gabarit `memory` (l'outil `whois`), les graphes cités (`ForEachNode`, `GraphNode`), le délai des
clients d'embarquement et l'arrêt borné du réacteur, le bâti des libs du moteur pour les bascules, la
procédure de bascule, et la conception des « vues par worktree ». Elle reçoit ses consignes de **c7**,
la session d'orchestration (aujourd'hui `rag3db-5a`), et travaille avec les sessions **cœur C++**,
**hotfixs** (correctifs du moteur), **G** ou « optimiseur » (les hôtes et la pose des libs),
**mémoire** ou « 96 » (la mémoire longue), **rag3weaver archi** (les hooks de Claude Code),
**recherche**, **everything declarative** ou « ED » (l'IDE et l'équipier).


## Reprise ED et dernière série avant rebase

ED a intégré **c693496a8c2b3b95dd63877d356ed80e62797bd9** avec ses trois contrôles
propres/aucun rebase/origin-master=HEAD. Le même ticket ouvert est transféré à arbre :
`docs/arbre-principal/tickets/2026-11-11-ordonner-les-entites-derivees-a-l-ouverture-du-backend.md`.
À nous : validation statique commune de search_index et vrai manifeste combiné public,
sans arête inverse de création ni activation de base utilisateur. Les preuves ED restent ED.

Dernière source propre de notre branche3 : **b6f78bf878f1ee4f610c5db70b5a6de6a98e4f2d**.
`cx-cellules-reouverture-v4.log` : **22 dérivées et 3 Scope privé verts**, rc0. Inclut titre
non projetable refusé, FTS physique, rollback/reprise bêta, réouverture durable et ScopeNONE.
`cx-contenu-pg-v4.log` : ingestion franchie, lecture publique bloquée ; aucune preuve PG
verte. Trois sessions privées dont une idle-in-transaction, aucun bloqueur SQL selon
pg_stat_activity. Attachement gdb refusé par Yama, sans contournement ; seul le PID du test
**1166384**, chemin binaire et filtre exacts relus, a reçu SIGTERM. Zéro session privée
restante ensuite. Le lanceur a terminé PG en échec puis les deux suites natives ci-dessus.
Traces : `cx-pg-v4-piles.log` (refus), journal PGv4 et journal natif hors target.
La suite v4 prévue sur ba879 n'avait jamais démarré (checkout refusé, témoin modifié) ;
la v4 réellement exécutée porte bien b6f78 dans son en-tête.

**Fournisseur terminé nettoyé au portable** : WT `cx-wt-schema-code-provider` retiré sans
force après copie fidèle `/home/lucied/.cache/rag3weaver-build/cx-provider-nettoyage-c12/administration`
et garde helper65aa SHA dca13abb…252f628 **VERT**, rapport neuf `garde.json` dans ce dossier.
Aucun sous-module initialisé, aucun target créé dans ce WT. La branche distante c12 est
conservée ; la branche locale aussi (pas de suppression forcée d'une histoire reprise par
cherry-pick). Le worktree 1a, son target et le diagnostic baseline restent en place.

Prochain pas : rebase 1a sur une branche neuve depuis master ED, relever le diff, raccord
manifeste et diagnostic PG borné. Pas de relance lourde e2e_search avant recherche ; son
correctif et le faux service léger sont désormais autorisés chez son porteur, pas ici.

## Suite 1a — gardes et cellules, après 06b3ee4cb

**Périmètre courant : toujours hors master, lot 1a ouvert.** Branche
`cx-arbre-scope-version-3`, worktree `~/.cache/rag3weaver-build/cx-wt-scope-version`
aux deux postes. Target unique luciepc `cx-target-contenu-derive`, lib29a7bc52a.
Checkpoint **06b3ee4cb** poussé : refus du changement d'association de recherche sur
entité peuplée, avancement public résolu vers l'index physique, recette nouvelle
persistée en dette globale mais racines sélectionnées par cellule avant LIMIT.

Les deux témoins association/avancement rougissent avant correction puis passent
(`cx-projection-gardes-{rouge,vert}.log`, 3 verts). Le rouge cellule de recette
repointait deux racines alpha vers une version bêta ; corrigé et vert dans
`cx-projection-cellules-vert.log` (4 verts). Tous ces journaux sont hors target sur luciepc.

Le candidat suivant ajoute les files par cellule et applique la consigne dans la
cellule explicite de l'appel. Son ticket commun, owner arbre-principal :
[la file ne suit pas la cellule](../tickets/2026-11-11-la-file-d-ecriture-ne-suit-pas-la-cellule-de-lecture.md).
La préexistence est prouvée sur master c04c62021 dans le worktree diagnostic
`cx-wt-file-cellule-baseline` **sur luciepc seulement**, runtime intact, seul un test
Product ajouté : le drain bêta consomme aussi alpha. Log `cx-file-cellule-baseline-rouge.log`.
Aucun nettoyage de ce worktree n'est fait : inventorier les administrations de sous-module
et leurs dépendances avec la garde commune avant de le retirer.

Sur le candidat : lecture stricte alpha reconstruit ses deux références, conserve
l'écriture bêta puis la draine en bêta (`cx-projection-lecture-cellule-vert-v2.log`,
5 verts). Les 18 combinaisons signal/mode/cellule et les suites entières dérivées et
cellules passent : `cx-projection-cellules-regressions.log`, **19 + 13 verts**, rc0.
Le candidat ultérieur ajoute les gardes transaction/retour d'erreur/rollback, le bilan
global des files garées et le refus du scan sans masque pour une projection.
Le témoin scan rouge réel est `cx-projection-scan-rouge-v2.log` ; le premier essai
était une erreur de fixture (`chunked=false` avec vector), pas un rouge produit.
La passe v2 donne bibliothèque 2101/0/58 et scope 13, Scope privé 3, recherche 40,
projection 2 verts ; dérivées 19/1 car le nouveau témoin laissait sa faute armée en
reprenant bêta. Désarmée, rollback/reprise passe. Une assertion dédiée révèle ensuite
le warning d'index propre Occurrence absent : rouge-v4 réel, correction par la résolution
physique existante dans l'entretien FTS. La nouvelle passe partielle est
`cx-projection-file-regressions-v3.log` ; lire son verdict avant tout bilan vert.
Verdict v3 : 2101 bibliothèque et 20+13+3+40+2 e2e sans assertion en échec, mais rc101
pour un journal du démon d'embarquement laissé dans le TMPDIR d'e2e_search :
[ticket recherche](../../recherche/tickets/2026-11-11-le-service-d-embarquement-laisse-son-journal-dans-le-tmpdir-d-e2e-search.md),
pris en suivi par recherche, sans nouveau lot activé. Aucune exemption ou journal effacé.
Le témoin SQL chaîné n'a donc pas démarré. Lancé séparément, il rougit dans
`cx-contenu-pg-premier.log` : uuid[] lié comme text[] mais requête JSON. Fix SQL candidat
unnest/ANY, non encore joué. Les ajustements ultérieurs fermeture/réouverture et ScopeNONE
sont aussi non encore joués : ce checkpoint n'est pas un verdict global.
Base privée **cx_contenu_2191_20261111**, créée par nous (preuve `cx-contenu-pg-creation.log`).
Ne pas toucher rag3weaver_test ; supprimer notre base à la fin du lot après contrôle.
Ce n'est pas encore la passe globale après ED.

**Sous-travail indépendant demandé par orchestration à la demande d'ED** :
`code::schema_configs(ChunkingConfig)` expose les vraies configurations et propriétés.
Commit **c12f840b4**, branche `cx-arbre-schema-code-provider`, base master c04c62021,
worktree local `cx-wt-schema-code-provider`, code.rs seul. ED a reçu le SHA et compose
Backend ; aucun import 1a. Le ticket ordre ED porte ce travail (trace ED 4857f58f3).
Vérification structurelle d'extraction identique et test d'équivalence ajouté. ED a confirmé
sa compilation et son vert 1/0/0 (`cibles-4.log`, source ED 11ea6ec87) ; les 16 cas d'ouverture
et 9 gabarits passent avec code, la passe finale composée bffbe5033 reste chez ED.
Aucun import 1a ni tour master demandé ici.

**Restant** : finir les filets et leur passe, récupérer la composition ED puis rebaser
sur une branche neuve, recoller la validation statique d'association commune et éprouver
l'ouverture manifeste ; ensuite seulement passe réglementaire et demande de tour.
Aucune activation du gabarit code sur base vivante, migration 1b ou mesure M3 autorisée.

## Checkpoint Scope privé — passe partielle du 11 novembre, 22:14–22:21 Paris

Porteur arbre-principal, [ticket 1a](../tickets/2026-11-11-deux-occurrences-identiques-reembarquent-le-meme-contenu.md).
Sources runtime figées à a1af9b507f90bfadd0c55d5c2058c15177a2992b, avec le seul diff de
fixture tests/e2e_scope_versions.rs (SHA256 d05eaa6443ba46b3036ccda64d8099049b61ea116e5c9ed9936b2b0ffc451d13).
8 features, lib 29a7bc52a épinglée, poste lourd, -j8, target unique cx-target-contenu-derive.
Résultats : bibliothèque 2 099 réussis/0 échec/58 ignorés ; Scope privé 3/0 ; dérivées 14/0 ;
processus terminé avec code 0 et sans résidus signalés. Pas un verdict de passe master complète.
Journal luciepc : ~/.cache/rag3weaver-build/cx-contenu-complet-prive.log ; SHA256
f570448be0bc00a6872273ae602f382cf0a05e17ab6797645f0511acd2db8263.

La fixture compose scope_config et Catalog::register_entity : ScopeContent rend les champs
name/signature/content/docstring/scope_type/language, dérive son identité du contenu et reçoit
le chunking existant ; Scope conserve ses UUIDs, positions et arêtes, pointe son search_index
sur cette représentation et ne produit plus de Scope_Chunk. Un seul embarquement pour deux
fichiers identiques ; déplacer l'une des occurrences n'en ajoute aucun. Docstrings distinctes
préservées ; recherches Scope sur les trois modes ; deux arêtes CONSUMES restent dans leurs
fichiers respectifs. Schéma code par défaut, code.rs et gabarits inchangés ; aucune base de service.
Les colonnes source des occurrences sont conservées comme entrée du rendu : ces preuves portent
sur la représentation indexée partagée et ses chunks/vecteurs, pas une mesure de déduplication
physique de toutes les colonnes source. Les chiffres M3–M5 restent absents.

Relecture ED sur 3c0363904 (runtime a166, docs seules ensuite) : validate_context reprend le corps
Catalog sans changement de règle ; config.validate précède le plan, donc identité content,
format et interdiction hashsafe sont préservés. Compatible avec root/content. search_index reste
une association de lecture, pas une dépendance de création inverse de from. Aucun import dans
la passe figée. Combinaison/rebase après atterrissage ED, puis preuve d'ouverture du vrai manifeste.

Suites encore ouvertes dans ce même ticket : garde de changement d'association sans migration,
état public de la cible indexée et comportement auto, cellules et reprise, validation de l'association
avant écritures lors de la combinaison, puis passe complète sur base finale. Les deux tickets ED
restent chez ED ; leur transmission ne vaut pas clôture. Pas de migration, M3–M5 ni activation réelle.

## Renforcement du masque et de l'ingestion — après 49caf64c6

Action portée par [le ticket du contenu partagé](../tickets/2026-11-11-deux-occurrences-identiques-reembarquent-le-meme-contenu.md).
Projection neutralisée : 1 version au lieu de 2 occurrences (cx-projection-publique-rouge.log).
Sans préfiltre, dense/creux/texte perdent tous le résultat visible derrière 12 orphelins mieux
classés (cx-projection-masque-rouge.log). Restauration : 13 e2e dérivées réussis/0 échec,
lanceur accepté sans résidu (cx-projection-masques-vert.log). Les 9 cas des trois signaux et trois
modes, l'ordre de pagination, un AUTRE_LIEN trompeur, les filtres A/B et le refus de remappage
d'octets sont couverts. Le masque calcule les versions par la relation canonique DERIVED_FROM,
puis emploie les filtres d'index existants ; la réadmission conserve le jeu des racines du même L6.
Le premier rejeu sparse passait ses assertions mais laissait un cache de fixture : code 101,
aucun vert global. Ordre de destruction des handles/TempDir corrigé avant les 13 verts ci-dessus.

Ingestion directe par lot : source déjà écrite avant le drain des dérivées, donc échec injecté
laissant A.body=après et version=avant (cx-projection-lot-reprise-rouge.log). Composition du même
L3 par ecrire_un_lot_atomique pour le drain et cette ingestion ; ciblé sur fichier : 1/0 dans
cx-projection-lot-reprise-vert.log. Après échec et réouverture sans réenregistrer le schéma,
sources/références/hashes inchangés ; nouvelle tentative réussie sur A, B intacte. La suite native
complète des dérivées reste à rejouer sur ce refactoring, puis le raccordement privé Scope.

Tentative unitaire rag3db-native seule : 1 565 réussis, 1 échoué, 15 ignorés. Le test
builtin_templates charge workspace/code sans feature code : la configuration est refusée comme
attendu. Le lanceur s'est arrêté avant le rejeu e2e : pas de passe verte revendiquée.
Suivi distinct, porteur ED :
[montage du manifeste lié sans code](../../everything-declarative/tickets/2026-11-11-le-test-du-manifeste-lie-demande-code-sans-garde-de-feature.md).
Configuration complète prévue ensuite. Accessibilité depuis un manifeste à éprouver aussi,
porteur ED : [ordre des déclarations dérivées](../../everything-declarative/tickets/2026-11-11-ordonner-les-entites-derivees-a-l-ouverture-du-backend.md).
Ce dernier point est issu de la lecture, pas d'un e2e reproduit : HashMap d'entités enregistrées
avant les relations, alors que from/gather exigent leurs dépendances. Aucune modification de
backend.rs, builtin_templates.rs, code.rs ou du gabarit code par ce lot à ce stade.

## Reprise suivante — association et projection publiques en branche

Porteur et action : [ticket des occurrences partagées](../tickets/2026-11-11-deux-occurrences-identiques-reembarquent-le-meme-contenu.md).
Association canonique search_index sur l'entité publique ; plan dérivé de from/DERIVED_FROM,
filtre relationnel commun pour le masque, projection dans les signaux avant fusion/pagination.
Aucun chemin spécial Scope. Les champs indexés doivent préserver les octets des champs publics
pour conserver leurs coordonnées ; un cas incompatible est refusé. Le stockage content direct
reste protégé. Le texte natif sans garantie de masque est refusé, sans recherche non filtrée.

Premier e2e natif : index_derive_rend_les_occurrences_publiques_avant_pagination, 1/0 dans
cx-projection-publique-premier.log (luciepc). Dense/plein texte × Aggregated/Detailed/SourceResolved,
deux occurrences et leurs positions, une seule version ; offset=1 conserve la seconde occurrence.
À éprouver maintenant : sensibilité du témoin, filtres et saturation de candidats invisibles avant
top-k pour chaque signal, sparse et cellules. Aucune passe globale sur ce diff. Scope/gabarits
inchangés, pas d'activation vivante ni de tour master. Les checkpoints précédents restent ci-dessous.

## Checkpoint d'implémentation 1a — après la projection livrée

Cette section prime sur les états de préparation ci-dessous. Branche active
`cx-arbre-scope-version-3`, worktree `~/.cache/rag3weaver-build/cx-wt-scope-version`
aux deux postes ; base du travail `c224979351e735df8766ee20fe592eba444440b8`.
Toutes les suites de cette reprise sont rattachées au même
[ticket des occurrences partagées](../tickets/2026-11-11-deux-occurrences-identiques-reembarquent-le-meme-contenu.md),
porté par arbre-principal : preuves des checkpoints 7748fa53c/d63157d8c, projection et masque,
raccordement Scope, validations, migration sur copies et coordination M3–M5. Il reste ouvert.

Runtime maintenant modifié ; lot 1a **incomplet**, aucun tour de push master.

- `DerivedIdentity` déclare `root` (défaut implicite inchangé) ou `content` avec format.
  L'adresse de contenu couvre octets des champs rendus, rôles titre/contenu, recette de découpage,
  format et cellule ; modèles/dimensions restent du ressort du magasin vectoriel existant.
- Le mode contenu réutilise `DERIVED_FROM`, porte `_render_hash` sur chaque lien, ne réinsère
  pas une version existante et repointe seulement l'origine modifiée. Les versions restent
  conservées après suppression d'une racine ; aucun GC ajouté. Les écritures directes sur une
  version sont refusées, ainsi qu'un changement d'identité sans migration déclarée.
- Premier rouge produit : en remettant temporairement l'identité par racine, le témoin
  `versions_immuables_partagees_et_references_independantes` voit 2 versions au lieu de 1.
  Restauration : 1 réussi/0 échec. Il couvre partage version/chunk, états indépendants, mutation
  d'une seule origine, retour au contenu antérieur et suppression sans perte de l'autre référence.
  Journaux luciepc hors target : `cx-contenu-reference-rouge-v3.log`,
  `cx-contenu-reference-vert.log`. Diagnostic `RAG3WEAVER_E2E_FEATURES=rag3db-native`,
  **pas une passe globale**. Les essais précédents ont échoué à la compilation du montage
  (variable rendue inutilisée par la mutation, puis `is_some` sur un Result), pas du produit.
- Composition L3 vérifiée : sans enveloppe, le rouge perd le lien A et garde B
  (`cx-contenu-atomicite-rouge-v2.log`). Avec elle, échecs d'insertion de version ET de lien :
  racines, versions et hashes intacts ; zéro écriture validée ; file indépendante préservée et
  texte annulé invisible. La dette est contrôlée sur Version : le premier essai de la fixture
  comptait aussi une dette de la dérivée historique, hors de cette assertion.
- Vérifications ciblées terminées : 3 tests de derive_nodes et 7 tests config/schéma
  (`cx-contenu-invariants.log`) ; e2e_entites_derivees **8 réussis/0 échec**
  (`cx-contenu-derivees-vert-v4.log`). Le compteur générique, rétabli par mutation en mode racine,
  reçoit 2 textes au lieu de 1 (`cx-contenu-compteur-rouge.log`) ; restauration : partage d'un
  embarquement, déplacement et retour à la version précédente sans nouvel appel. Son renforcement
  modèles 4/6 passe seul (`cx-contenu-compteur-modeles.log`) : deux espaces de vecteurs et leurs
  marqueurs, aucune nouvelle version de contenu. Ces diagnostics natifs restent distincts d'une passe
  globale ; PostgreSQL et les autres features n'ont pas été rejoués sur ce lot.
- Recette changée : nouvelle version et dette persistante par référence ; revenir à la recette
  précédente réutilise ses chunks. Les écritures directes, le reindex et les liens manuels vers les
  versions sont refusés. Les versions internes ne sont pas proposées comme cibles de recherche et
  leur recherche est refusée tant que la projection publique n'est pas déclarée.
- Bloquants avant 1a complet : association canonique explicite cible publique → index dérivé,
  redéploiement vers les occurrences avant pagination, masque avant top-k de chaque signal dans
  le même L6. Recherche confirme que from/DERIVED_FROM, filter_indirection, Forme et la réadmission
  du pré-lot se composent pour cela ; ResolveParent après pagination ne convient pas. Aucun trajet
  spécial Scope ni double déclaration indexed_by/projection. Le schéma Scope/gabarits restent
  inchangés et leur compteur partagé reste rouge. Migration 1b requise avant bases de service.
- Nouveau target unique en service sur luciepc :
  `~/.cache/rag3weaver-build/cx-target-contenu-derive`. Lib épinglée 29a7bc52a, `-j8`,
  poste lourd. Aucun accès aux bases de service, aucune mesure M3–M5 lancée.

Fichiers tenus : config.rs, derive_nodes.rs, record_nodes.rs, derived_kb.rs, catalog.rs
(hors réclamations ED), schema.rs, dialect.rs, read_view.rs, e2e_entites_derivees.rs. ED a confirmé ne pas
avoir repéré de primitive de remplacement atomique dans LinkRecordNode ; composer L3 conserve
les garanties communes. Codeparsers garde ses zones de code.rs ; aucun de ces blocs n'est édité.

## Reprise du 11 novembre — état vérifié après atterrissage

Cette section prime sur les chemins et prochains pas historiques ci-dessous.

- **Garde-fou livré** : master **1904e04e42cd3666d6b3a64fee5bd51f0629c020**.
  Témoin du scope déplacé, sans changement de comportement produit. Rouge de sensibilité :
  réutilisation neutralisée temporairement, un texte réembarqué sur un ; restauration : 1 réussi,
  0 échec. Features 5/5, lib large 2 095 réussis/0 échec/58 ignorés, navigateur 33/0/0,
  e2e_code 27/0/0 et e2e_code_sync 17/0/0 (2 suites sur 2). Le premier lancement interrompu
  pour plafonner les compilations à 8 reste distinct de cette passe terminée.
- **Intégration vérifiée** : tour orchestration reçu ; poste push ; dernier rebase documentaire
  seulement ; push dans if grep -qx VERT ; status propre, aucun rebase, origin/master égal à HEAD
  immédiatement après fetch. Branche sauvegardée : cx-arbre-deplace-sans-reembarquer-4.
- **Nettoyage terminé** : wt-deplace et sa branche locale retirés au portable ;
  wt-deplace-codex et target-A-plan retirés à luciepc après arrêt de tous nos travaux dessus.
  Aucune branche distante supprimée. Journaux conservés sur luciepc dans
  ~/.cache/rag3weaver-build/deplace-codex/ ; preuve du push au portable dans cx-deplace-push/.
- **Contrat des vues relu** : ED et recherche, corrigé et livré en docs seules sous 599cf78b2.
  Voir [03-contrats-et-lots-des-vues.md](03-contrats-et-lots-des-vues.md). Le cap ancre unique et
  pyramide est confirmé par Lucie ; pas de nouvelle attente d'accord de principe.
- **Lot 1a en cours, produit non implémenté** : branche cx-arbre-scope-version, témoin
  dd5f30465 ; worktree cx-wt-scope-version aux deux postes. e2e_scope_versions joué sur luciepc :
  1 réussi (docstrings distinctes), 1 échoué (deux occurrences correctement positionnées, mais
  leur même texte embarqué deux fois au lieu d'une). Rouge vérifié, pas de vert revendiqué.
  Journal hors target : ~/.cache/rag3weaver-build/cx-scope-version-rouge.log ; journal e2e
  détaillé dans cx-wt-scope-version/extension/rag3weaver/target/e2e-last.log.
  Rebaser ce lot sur le master comprenant le garde-fou avant validation ; nouveau target unique
  prévu pour cette session : ~/.cache/rag3weaver-build/cx-target-scope-version (luciepc).
- **Coordination code** : ED laisse code.rs et templates/backends/code à cette session.
  Codeparsers demande de conserver default_exports, pending_defaults et la résolution @default
  inter-lots ; aucune modification du sous-module, de son pointeur ou de stable_scope_keys.
- **M3–M5 différées** : copies reflink privées possibles des avant-43 format42 sur btrfs ; lib29a7
  format43, ouverture/migration à vérifier sur copie. Ni copie ni mesure lancée ; attendre file
  vide et mot orchestration, coordonné optimiseur. 7882 reste réservé au banc A/B codeparsers.

## Ce qui est sur master (le dépôt `rag3db`, branche `master`)

| lot | commits | ce qu'il fait |
|---|---|---|
| `whois` | 393d4b0c0..2f4ed7841 | outil du gabarit mémoire : par clé d'agent, handle ou alias, rend la clé, les alias et handles datés, le dernier apport `identity` et la tâche de sa ligne `doing:` |
| le juge d'un tour de Claude Code, et le graphe cité intact | 994a58e39..76f19526b | le juge (`src/daemon_host/juge.rs`) juge un run de genre `claude_code_turn` ; la substitution et la garde de bijection n'entrent plus dans un graphe cité |
| le délai partout | d63c16649..d8aeffea7 | relecteur et OCR d'un démon avec le délai de l'embarqueur ; l'arrêt borné d'un réacteur compte hors du runtime ; la lib entière ne fige plus derrière un service muet |
| procédure de bascule de lib | 9dc626c70, b99b67de0, cf14a7771 | `docs/procedures/bascule-de-lib.md` : étape 8 (contrôle des deux sens), `e2e_lectures` au verdict |
| lecture « ce que le moteur doit au modèle des vues » | 9bd377476 (+ ddb856e21) | `extension/rag3weaver/docs/07-novembre-2026-arbre/01-…` |
| conception des vues, marche 1 | 326c145b4, 199f4d75c, 10b8affce | `extension/rag3weaver/docs/08-novembre-2026-arbre/01-conception-des-vues-marche-1.md` |
| rapports et tickets | b1f4b2252, 5fd0e756a, 47684ec95, 0ada9d7f0, af035a99f, d5030c231, 5217a60b8 | rapport du 7 novembre, tickets des figements |

**Libs du moteur bâties par cette session** (le « magasin » de libs : `~/.cache/rag3db-libs/<id>`, une
lib par dossier, nommée par son dernier commit moteur) : 17a094d12 (bascule au format 43, 7 nov.),
7cd11e5fa (remplacée avant sa pose), dadbf0041 (posée par G le 8 nov. à 8 h 31), cbc06c3e1 (« fini »
donné le 8 nov. à 11 h 57). **La lib en service (`courant`) aux deux postes est aujourd'hui
29a7bc52a**, rebâtie depuis par une autre session dans le même worktree de bâti.

## Chantiers en cours et leur état exact

1. **Les vues par worktree** (la demande de Lucie : un worktree n'indexe que ses diffs au grain du
   scope, et les recherches se filtrent par worktree, branche ou pile de branches). **Page de
   conception sur master (10b8affce), en attente de la relecture d'ED et de recherche, et de la
   lecture de Lucie.** Aucun code de la marche n'est écrit. Le seul code prêt est un témoin garde-fou
   (ci-dessous).
2. **Le témoin « un scope seulement déplacé ne se réembarque pas »** :
   branche `origin/arbre-principal/deplace-sans-reembarquer` (021340283), worktree
   `~/.cache/rag3weaver-build/wt-deplace` sur le portable. Vert. Il entre dans le lot 1 des vues ;
   pas de passe complète jouée, pas sur master.
3. **Les mesures M3 à M5 du modèle des vues** (coût du filtre par appel, rappel sous filtre minuscule,
   lignes mortes) : **non jouées**. Le témoin `e2e_mesure_vues` est sur
   `origin/arbre-principal/mesure-vues` (b826a1147). Ses copies de base ont été effacées le 8 nov. ;
   il faudra de nouvelles copies reflink des instantanés `avant-43`.

## Reprendre

### Le garde-fou du scope déplacé (le seul worktree au travail non fini)

- **Où** : le portable, `~/.cache/rag3weaver-build/wt-deplace`.
- **Branche** : `arbre-principal/deplace-sans-reembarquer`, poussée ; dernier commit **021340283**
  (8 nov.), posé sur 326c145b4 ; **145 commits de retard** sur master au 11 nov. (la procédure refuse
  un worktree au-delà de 200 : on en referait un neuf).
- **Fait** : le témoin `backend::tests::a_scope_that_only_moves_is_not_embedded_again`
  (`extension/rag3weaver/src/backend.rs`), et `backend_de_code_avec`, qui prend l'embarqueur du banc.
  Joué seul, vert (8 nov.).
- **Reste** : le faire entrer dans le lot 1 des vues (ou seul, si c7 le veut), avec une passe
  complète, puis le push.
- **Le target** : `~/.cache/rag3weaver-build/target-plan-local` (1,9 Go) ; il a été bâti par
  d'autres worktrees, d'où `RAG3WEAVER_TARGET_REPRENDRE=1` au premier appel.
- **Reprendre là** (tout se tape dans un shell bash ; l'outil Bash de Claude Code est zsh, voir le
  knowledge dump) :

```bash
cd ~/.cache/rag3weaver-build/wt-deplace
git fetch -q origin && git rebase origin/master && git submodule update --init -q
git push origin HEAD:refs/heads/arbre-principal/deplace-sans-reembarquer-2   # un rebase se pousse sous un nom neuf
cd extension/rag3weaver
export CARGO_TARGET_DIR=~/.cache/rag3weaver-build/target-plan-local CARGO_INCREMENTAL=0 RAG3WEAVER_TARGET_REPRENDRE=1
export RAG3DB_LIB=rag3db-build-29a7bc52aaf11e6204f9beafaffb154105417e13-Release-p12-v11-g17-s18-rcOFF-stOFF-asanOFF-tsanOFF-ubsanOFF
# le témoin seul :
~/.cache/rag3weaver-build/poste lourd timeout 30m scripts/cargo_magasin.sh test -j 8 --lib \
  --features rag3db-native,code,daemon -- a_scope_that_only_moves_is_not_embedded_again
# la passe complète avant tout push : voir « La passe d'un lot » dans 02-knowledge-dump.md
```

  Si master a déjà un moteur plus neuf que la lib en service, ajouter `RAG3WEAVER_MOTEUR_ANCIEN=1` en
  le disant ; au 11 nov., la lib 29a7bc52a est celle du moteur de master.

### Les mesures M3 à M5 des vues (pas de worktree : il est effacé)

- **Où était le travail** : luciepc ; le témoin `extension/rag3weaver/tests/e2e_mesure_vues.rs` est
  sur la branche `arbre-principal/mesure-vues` (b826a1147), ses copies de base sont effacées.
- **Fait** : le témoin (en Cypher brut sur une copie de base), et une première passe qui a appris que
  `PROJECT_GRAPH_CYPHER` est refusé sur une base ouverte en lecture seule (le témoin ouvre sa copie en
  écriture).
- **Reste** : M3 (coût du filtre de vue par appel), M4 (rappel sous filtre minuscule), M5 (lignes
  mortes de vues fermées) — chiffres bruts à écrire dans
  `extension/rag3weaver/docs/07-novembre-2026-arbre/01-ce-que-le-moteur-doit-au-modele-des-vues.md`.
- **Reprendre** : un worktree neuf sur la branche, rebasé sur master ; copies reflink de
  `~/.local/share/rag3weaver/instantanes/avant-43/code-principal` et `code-lourd` (luciepc) dans un
  dossier à soi ; puis, **seulement quand `poste --qui` ne montre aucun lourd en attente**, et après
  le mot de c7 :

```bash
RAG3WEAVER_MESURE=1 MESURE_VUES_BASE=<copie>/code-principal.rag3db ./run_e2e.sh --test e2e_mesure_vues
MESURE_VUES_ECRIRE=1 MESURE_VUES_CYCLES=20 MESURE_VUES_BASE=<autre copie> ./run_e2e.sh --test e2e_mesure_vues   # M5
```

## Branches distantes de cette session non atterries

| branche | état |
|---|---|
| `arbre-principal/deplace-sans-reembarquer` (021340283) | le garde-fou ci-dessus, à garder |
| `arbre-principal/mesure-vues` (b826a1147) | le témoin de mesure M3-M5, à garder |
| `arbre-principal/graphe-cite-intact` (4be482a00) | contenu atterri sous un autre commit (2fe3c114f) ; inutile |
| `arbre-principal/vues-lecture-2` (0375a139b) | un ticket retiré exprès (doublon de celui de recherche) ; inutile |
| `arbre-principal/ancien-plan`, `ancien-plafond`, `ancien-l4b-essai` | essais d'octobre gardés pour mémoire |

**Aucune branche distante ne s'efface sans le mot de Lucie** (règle de Lucie).

## L'arbre de Lucie (`~/git_workspaces/rag3db`, le portable)

- Sur `master`, à jour avec `origin/master` (3c0ec28b5 au 11 nov. à 16 h 20), sans modification
  suivie **sauf le sous-module** `extension/rag3weaver/codeparsers` : le pointeur commité est
  c660893da, le sous-module extrait est à **672a0d5**. Ce n'est pas cette session qui l'a bougé ; à
  dire à Lucie, ne pas y toucher.
- Rien n'y est commité ni basculé par une session (règle de Lucie) ; les sessions y lisent
  seulement, et y font `git fetch` (qui ne touche que les références distantes) et `git worktree add`.

## Les worktrees de cette session

| poste | chemin | branche | à effacer ? |
|---|---|---|---|
| portable | `~/.cache/rag3weaver-build/wt-deplace` | `arbre-principal/deplace-sans-reembarquer` | au mot de Lucie, ou quand le lot 1 l'aura repris |
| portable | `~/.cache/rag3weaver-build/wt-docs-73` | la branche de cette passation | après le push de cette passation |
| luciepc | `~/.cache/rag3weaver-build/wt-lib-arbre` | détaché (29a7bc52a) | **non** : le worktree de bâti des libs, servi aussi par d'autres sessions |
| luciepc | `~/git_workspaces/rag3db-plan` | détaché (f7df88003) | le worktree du verdict des libs (`lib58/verdict58.sh`) : à garder tant qu'on bâtit des libs |

Les autres worktrees du portable (`wt-juge`, `wt-react5`, `wt-flux`, `rag3db-*`…) appartiennent à
d'autres sessions : ne pas y toucher.

**Disque restant à moi** : `~/.cache/rag3weaver-build/target-plan-local` (1,9 Go, portable) ;
`~/.cache/rag3weaver-build/target-A-plan` (luciepc, recréé par un verdict) ; les notes de luciepc
`~/.cache/rag3weaver-notes-A/` (scripts, journaux, copies `lib58-*`).

## Les bases et les hôtes

Cette session n'inscrit aucune base. Les quatre bases partagées (portable : `memoire-longue`,
`code-rag3db-depot` ; luciepc : `code-principal`, `code-lourd`, sous `~/.local/share/rag3weaver/`) sont
servies par les hôtes de G (le démon sur le port 7970). Les instantanés `avant-43` (copies reflink
prises avant la bascule au format 43) sont dans `~/.local/share/rag3weaver/instantanes/avant-43/` aux
deux postes ; ce sont les seuls restants.

## Tickets ouverts qui sont à cette session (dans `docs/tickets/`, non déplacés)

- `2026-11-07-le-juge-des-outils-sur-gemini-range-parfois-un-parametre-obligatoire-en-description-trompeuse.md` — la consigne du juge doit départager deux classes de défaut (c7 a tranché : c'est le témoin qui a raison).
- `2026-11-08-le-delai-du-service-d-embarquement-est-un-chiffre-rond.md` — mesurer le p99 d'un `/embed` sous charge et remplacer le défaut de 60 s.
- `2026-10-11-l-index-ne-suit-pas-l-arbre-edite.md` — à cadrer avec l'orchestration (les vues y répondent en partie).
- `2026-10-11-un-appel-d-outil-attend-derriere-un-index-sans-le-dire.md` — pour la session proto et cette session.
- `2026-10-11-e2e-impact-depend-de-la-forme-du-code-vivant.md` — codeparsers ou cette session, après une migration.
- `2026-10-11-une-lib-d-une-autre-abi-se-charge-et-tombe-sur-un-symbole.md` — ouvert par cette session au nom du cœur C++.

Ouverts par cette session mais pour d'autres : `2026-11-08-le-temoin-de-la-porte-de-l-equipier-lit-le-second-retour-avant-qu-il-arrive.md` (ED).

## Ce qui est bloqué, et sur quoi

- Le lot 1 des vues : la relecture d'ED et de recherche, et la lecture de Lucie de la page.
- Les mesures M3-M5 : une fenêtre où la file de luciepc est vide (une mesure exclusive bloque tous
  les autres lourds tant qu'elle attend).
- Le ticket du délai : la mesure sous charge, dans une fenêtre de G.

## Les trois prochains pas

1. Lire les relectures de la page des vues ; y répondre ; puis écrire le lot 1 (la version de scope
   sur l'ancre, et la migration sans index complet), témoins rouges d'abord, avec le garde-fou de
   `wt-deplace`.
2. Jouer M3 (le coût du filtre de vue par appel) sur des copies reflink des instantanés `avant-43`,
   dans une fenêtre de file vide : c'est elle qui décide la marche 2.
3. Le ticket du juge sur Gemini : départager dans la consigne du juge `parametre_devrait_etre_optionnel`
   et `description_trompeuse`, témoin sur plusieurs tirages.
