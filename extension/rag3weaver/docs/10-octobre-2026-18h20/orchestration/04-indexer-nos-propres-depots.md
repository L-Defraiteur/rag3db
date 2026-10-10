# Indexer nos propres dépôts pour travailler plus vite — les points bloquants

10 octobre 2026, 23 h, orchestration. **Priorité maximale, décidée par Lucie** :
« pouvoir indexer nos propres repos pour travailler plus vite d'abord ». Pour les
sessions codeparsers, mémoire, arbre principal, embarquements, et pour Lucie.

## 0. Le but, en une phrase

Que chaque session Claude Code de ce projet (le cœur C++ d'abord, puis les
autres) interroge une base rag3weaver de son dépôt par MCP — usages, impact,
voisinage, verrous sur le chemin — au lieu de grep et de lectures par tranches.
Premier cas : « qui appelle `NodeTable::update`, et qui tient le verrou de
l'index avant ? », posé par le cœur C++ avant A4′/V2.

## 1. Ce qui marche déjà

- **codeparsers** lit le C++ (tree-sitter-c, tree-sitter-cpp) : sur tout
  `rag3db/src/` (1 543 fichiers, 6,3 Mo) en 1,2 s, les scopes sortent bien
  (namespaces, classes, méthodes, fonctions, gabarits) ; une définition `A::f`
  du `.cpp` est rattachée à sa classe du `.h` (un seul scope, réuni par
  rag3weaver). Rust et TypeScript sont en usage quotidien sur rag3weaver.
- **rag3weaver** indexe, cherche, et expose `usages`, `impact`, le voisinage,
  les liens ; le gabarit `code` existe ; l'embarqueur est optionnel (plein
  texte seul sans service, vecteurs en dette).
- **Le serveur MCP** (session mémoire, `memoire-longue-6`, en fusion ce
  soir) : douze outils du gabarit `code` sur stdio, recette
  `templates/mcp/README.md` + `claude-code.exemple.mcp.json`.
- **luciepc** : services d'embarquement locaux (7878/7879/7880), lib du
  moteur à jour, worktrees par chantier.

## 2. Les points bloquants, par ordre de gravité

| # | Bloquant | Preuve | Qui | Passes |
|---|---|---|---|---|
| B1 | **Les relations C++ ne traversent pas les fichiers en mode fichier seul** (celui de rag3weaver) ; entre fichiers, 36 % des appels seulement reçoivent une relation ; `#include` ne donne aucune relation fichier → fichier | sonde codeparsers du 10 oct., 26 028 appels relevés, 11 % reliés en fichier seul | codeparsers | compté dans B2 |
| B2 | **Les receveurs C++ non typés** (`table->update(…)`, `->cast<NodeTable>()`, `auto`, champs implicites) : `NodeTable::update` a 0 appelant relié, même entre fichiers | même sonde | codeparsers | 2 à 3 |
| B3 | **Les verrous sont invisibles** : 188 gardes RAII (`unique_lock`, `lock_guard`, `shared_lock`, `scoped_lock`) et `.lock()` dans le texte, aucune relation, pas d'identité pour un mutex membre (`Classe::champ`) | même sonde | codeparsers (relation LOCKS) + rag3weaver (section « verrous sur le chemin » dans `impact`, nouvelle relation dans l'IR) | 3 |
| B4 | **Le compteur de « pas su comprendre » de codeparsers rend 0 partout** : le relevé n'existe pas ; 215 fichiers sur 1 543 ont des nœuds ERROR tree-sitter (1,37 % des octets, 72 Ko dans `c_api/rag3db.h` seul) et personne ne le voit | même sonde | codeparsers | 1 (en tête : le banc s'appuie dessus) |
| B5 | **Aucun banc de couverture par dépôt** : rien ne dit, à chaque changement, ce qui monte et ce qui descend sur rag3db, rag3weaver, codeparsers, l'IR | — | codeparsers | 1 |
| B6 | **Le premier mur MCP** : sur une base vide, `usages`/`search_code` répondent `unknown entity: File` au lieu de « base non indexée, appelle `index` » | ticket `docs/tickets/2026-10-10-unknown-entity-file-ne-dit-pas-d-indexer.md` | arbre principal (`code_tools.rs:425-430`) | 1 |
| B7 | **Deux sessions sur une même base** : le mode `--demon` du serveur MCP est « non éprouvé, ni validé ni réfuté » (son témoin n'a pas atteint le sujet) ; en attendant, une base par session, et chaque session indexe la sienne | session mémoire, 10 oct. 22 h 40 | mémoire | 1 à 2 |
| B8 | **Le tampon de 256 Mio plein à la première écriture d'un backend** (persistence et sparse rouges sur master, cause non trouvée, suspect : les défauts basculés) : un index de 28 Mo de C++ peut le rencontrer | ticket `2026-10-10-tampon-de-256-mio-plein-a-la-premiere-ecriture.md` | arbre principal | 1 à 2 |
| B9 | **Rust : couverture non mesurée** sur nos dépôts (traits, impl, closures, macros, `use` réexportés — ticket des réexportations ouvert) ; on sait qu'elle sert, pas ce qui lui manque | ticket réexportations ; aucune ligne de référence | codeparsers (B5 la mesure) | compté dans B5 |

Hors liste, parce que ce n'est pas bloquant : le temps du premier index de
28 Mo de C++ avec granite-278m local (à mesurer, l'embarqueur est optionnel :
le plein texte seul suffit pour `usages`/`impact`) ; le gabarit `code` qui
pourrait supposer Rust/TS quelque part (à vérifier par la mémoire, question
posée).

## 3. L'ordre, et qui

1. **codeparsers** (session `rag3db-ab`, chantier de couverture décidé par
   Lucie) : B4 le compteur, puis B5 le banc avec une ligne de référence
   commitée par dépôt (rag3db `src/` et `test/`, rag3weaver, codeparsers,
   l'IR), puis B2 les receveurs typés (avec B1 : les relations entre fichiers
   que rag3weaver sait réunir par le rendez-vous par nom, ou un mode entre
   fichiers borné au dépôt), puis B3 la relation LOCKS. À chaque pas la
   mesure « appelants de `NodeTable::update` » monte, et aucune ligne du banc
   ne descend.
2. **Mémoire** (`rag3db-96`) : la fusion MCP ce soir ; puis B7 — soit le
   mode `--demon` éprouvé (témoin rouge : N `initialize` concurrents sur la
   même base), soit la recette « une base par session » écrite et jouée ;
   puis la vérification du gabarit `code` sur un dépôt C++. La page « mémoire
   par MCP » vient après.
3. **Arbre principal** (`rag3db-73`) : B6 (le refus qui dit d'indexer) monte
   juste après le témoin de la fuite, avant les rouges Python ; B8 avec les
   rouges Python ; puis la section « verrous sur le chemin » dans `impact`
   quand codeparsers livre LOCKS (diff proposé par codeparsers, relation dans
   l'IR avec `rag3db-6f`).
4. **Le premier index réel** : dès B6 et B7 (ou la recette une base par
   session), indexer `rag3db/src` et `test/` sur luciepc et brancher les
   deux sessions cœur C++ par `claude mcp add` — même avant B2 et B3 : le
   plein texte, les scopes et le voisinage par nom servent déjà ; chaque lot
   de codeparsers réindexe et la réponse à « qui appelle `update` » s'améliore
   sous leurs yeux. C'est aussi le banc vivant.

Coût total : 8 à 10 passes, dont 7 à 8 chez codeparsers. L'ordre de grandeur
est une semaine de passes d'agent ; le premier index utilisable arrive avant,
dès que B6 et B7 sont réglés (2 à 3 passes).

## 4. Ce qui est décidé, ce qui attend Lucie

Décidé : la priorité ; codeparsers sur la couverture, C++ et Rust, mesurée sur
nos dépôts ; ce qu'on ne sait pas comprendre est dit, jamais jeté.

Attend Lucie : rien de bloquant. Les « nouveaux projets émergents » qu'elle a
vus pour la classification de mémoire sont pour la page de la mémoire, pas
pour celle-ci.
