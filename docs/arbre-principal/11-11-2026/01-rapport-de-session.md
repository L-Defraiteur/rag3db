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
  le worktree de validation distant et target-A-plan retirés à luciepc après arrêt de tous nos travaux dessus.
  Aucune branche distante supprimée. Journaux conservés sur luciepc dans
  ~/.cache/rag3weaver-build/cx-deplace-preuves/ ; preuve du push au portable dans cx-deplace-push/.
- **Contrat des vues relu** : ED et recherche, corrigé et livré en docs seules sous 599cf78b2.
  Voir [03-contrats-et-lots-des-vues.md](03-contrats-et-lots-des-vues.md). Le cap ancre unique et
  pyramide est confirmé par Lucie ; pas de nouvelle attente d'accord de principe.
- **Projection dense/creuse livrée** : master **6c4b5ad0c0f757ee17905c1643dd8b39174ccaa9**.
  Un morceau peut rendre tous ses parents admis ; chaque parent conserve champs, positions et
  attribution. Déduplication de la paire morceau/parent, meilleur score, égalités par identité,
  réadmission sous filtre effectif et cellule de la même lecture. Aucun format ni stockage partagé
  activé. Le [README](../../../extension/rag3weaver/README.md#search) et les nouveautés le décrivent.
- **Preuves de projection** : rouge perte d'un parent ; rouge de fuite provoquée sans réadmission ;
  rouge sur le choix ex aequo ; verts après restauration/correction. Passe complète 88136cc0e :
  features5/5, lib2096/0/58 ignorés, navigateur33/0/0, quatre suites e2e sans saut. Rejeu après
  ED91e5f760b sur 51a2faa1f : unités projection/L4 1/0 chacune, features5/5, lib2097/0/58 ignorés ;
  e2e_projection_occurrences1/0, e2e_search40/0, e2e_scope12/0, e2e_postgres34/0 (87 tests, 4/4 suites).
  Navigation non rejouée dans ce rejeu court. Journaux luciepc : cx-projection-passe-v3/ et
  cx-projection-rejeu-ed/ sous ~/.cache/rag3weaver-build/ ; anciens lancements conservés séparément.
- **Push et nettoyage de projection** : tour reçu, poste push, if grep -qx VERT ; dernier rebase
  documentaire et sauvegarde cx-arbre-projection-occurrences-5. Trois contrôles lus : status propre,
  aucun rebase, origin/master=HEAD6c4b5ad0c. Worktrees cx-wt-projection-occurrences et
  cx-wt-projection-cellules retirés aux deux postes, branches locales retirées, target
  cx-target-scope-version et base PG privée cx_projection_2191_20261111 supprimés. Références
  distantes, preuves hors target et worktrees 1a conservés.
- **Cellule explicite contrôlée séparément** : témoin diagnostique fdd710eb2, 12 cas verts
  (dense/creux × 3 modes × alpha demandée/beta courante), deux parents alpha conservés via le
  même morceau, parent beta exclu, catalogue toujours en beta. API Catalog::rechercher normalise
  options.scope puis fournit le service scope de la cellule demandée. Aucun correctif runtime
  supplémentaire, aucun changement de précédence. Ce témoin n'est pas dans le candidat livré ;
  il est conservé dans la préparation 1a. Log cx-projection-cellules-v2.log. Le premier diagnostic
  sparse était mal préparé ; le comparateur sans pré-lot s'arrête sur l'ancienne perte de parent
  avant son contrôle no-handle prévu. Ne pas compter ces diagnostics comme rouge du lot livré.
- **Lot 1a en cours, stockage non implémenté** : branche **cx-arbre-scope-version-3**, tête
  **c22497935**, worktree cx-wt-scope-version aux deux postes. Rebasé après la livraison ; les
  fichiers de runtime sont identiques au master6c4b5ad0c lors de ce rebase. Reliquat : témoins, registre e2e et préparation
  déclarative [04-identite-des-derivees-et-projection.md](04-identite-des-derivees-et-projection.md).
  Le témoin d'embarquement partagé reste un rouge connu (preuve antérieure : deux textes identiques
  embarqués deux fois), ticket ouvert dans ce dossier. Aucun nouveau cargo après le nettoyage.
  Prochain target unique prévu au besoin : ~/.cache/rag3weaver-build/cx-target-contenu-derive.
- **Coordination code** : ED laisse code.rs et templates/backends/code à cette session.
  Codeparsers tient analyze_in_project, default_export_symbol et resolve_across_batches ; conserver
  default_exports/pending_defaults tuple3 et @default avec fichier exact, extension comprise.
  Repère communiqué 3f8f1e319 / sous-module 72810b4, à réactualiser avant édition. Aucun changement
  par cette session de code.rs, du gabarit, du sous-module, de son pointeur ou de stable_scope_keys.
- **M3–M5 différées** : copies reflink privées possibles des avant-43 format42 sur btrfs ; lib29a7
  format43, ouverture/migration à vérifier sur copie. Ni copie ni mesure lancée ; attendre file
  vide et mot orchestration, coordonné optimiseur. 7882 a été annoncé libéré après les index A/B ;
  les sondes/oracle continuent hors GPU. Aucune fenêtre exclusive n’a été attribuée.

## Lots antérieurs sur master (le dépôt `rag3db`)

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

## Chantiers en cours et reprise

### Le contenu partagé déclaré (lot 1a)

- **Où** : ~/.cache/rag3weaver-build/cx-wt-scope-version aux deux postes.
- **Branche sauvegardée** : cx-arbre-scope-version-3 (c22497935). Aucun runtime propre au lot
  ne reste à rejouer par-dessus la projection : conserver les corrections déjà sur master.
- **Lire** : le contrat 03, puis la préparation 04. DeriveNode utilise une identité par racine et
  un _render_hash par ligne ; une version partagée exige une identité de contenu et un suivi par
  racine/lien, en composant la relation dérivée déjà existante. La déclaration exacte est à éprouver.
- **Témoin rouge** : e2e_scope_versions::deux_occurrences_partagent_un_embarquement_et_gardent_leurs_positions.
  Le compteur voit deux embarquements d'un même texte ; le partage du stockage n'est pas livré.
- **Reste** : identité/liaison de contenu déclarées, projection sur occurrences conservée, preuves
  de titre/docstring/recette/modèle et gestion explicite d'ancien format. Migration 1b sur copies
  privées avant bases vivantes. Coordonnées et arêtes restent propres aux occurrences.
- **Target** : cx-target-contenu-derive à créer seulement au prochain travail de compilation.
  Ne pas recréer les targets nettoyés du garde-fou et de la projection.

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

## Branches distantes historiques de cette session

| branche | état |
|---|---|
| `arbre-principal/deplace-sans-reembarquer` (021340283) | témoin livré sous 1904e04e4 ; référence distante conservée |
| `arbre-principal/mesure-vues` (b826a1147) | le témoin de mesure M3-M5, à garder |
| `arbre-principal/graphe-cite-intact` (4be482a00) | contenu atterri sous un autre commit (2fe3c114f) ; inutile |
| `arbre-principal/vues-lecture-2` (0375a139b) | un ticket retiré exprès (doublon de celui de recherche) ; inutile |
| `arbre-principal/ancien-plan`, `ancien-plafond`, `ancien-l4b-essai` | essais d'octobre gardés pour mémoire |

**Aucune branche distante ne s'efface sans le mot de Lucie** (règle de Lucie).

## L'arbre de Lucie — état historique de la passation

- Sur `master`, à jour avec `origin/master` (3c0ec28b5 au 11 nov. à 16 h 20), sans modification
  suivie **sauf le sous-module** `extension/rag3weaver/codeparsers` : le pointeur commité est
  c660893da, le sous-module extrait est à **672a0d5**. Ce n'est pas cette session qui l'a bougé ; à
  dire à Lucie, ne pas y toucher.
- Rien n'y est commité ni basculé par une session (règle de Lucie) ; les sessions y lisent
  seulement, et y font `git fetch` (qui ne touche que les références distantes) et `git worktree add`.

## Les worktrees de cette session

Le worktree actif de reprise est cx-wt-scope-version. Les chemins hérités des autres lignes sont
ceux de la passation ; leur état n'a pas été revérifié pour ce lot.

| poste | chemin | branche | à effacer ? |
|---|---|---|---|
| portable et luciepc | `~/.cache/rag3weaver-build/cx-wt-scope-version` | `cx-arbre-scope-version-3` (portable), détaché (luciepc) | non : lot 1a en cours |
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

1. Reprendre la préparation 04 et les témoins 1a sur cx-arbre-scope-version-3 ; composer l'identité
   de contenu et la projection déclarées, après coordination des blocs code.rs avec codeparsers.
   Le garde-fou et la primitive de projection sont déjà sur master ; la migration reste le lot 1b.
2. Jouer M3 (le coût du filtre de vue par appel) sur des copies reflink des instantanés `avant-43`,
   dans une fenêtre de file vide : c'est elle qui décide la marche 2.
3. Le ticket du juge sur Gemini : départager dans la consigne du juge `parametre_devrait_etre_optionnel`
   et `description_trompeuse`, témoin sur plusieurs tirages.
