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
