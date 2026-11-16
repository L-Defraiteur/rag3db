# Passation pour Codex, lundi 16 novembre 2026

Lucie, 9 h 3x : « on est lundi matin ; dis à toutes les sessions de faire un rapport de passation avec
demandes claires de tickets à traiter ensuite, chacune pour Codex ; comme ça si pendant la semaine je
bosse avec Codex il saura quoi faire ; on peut lui prévoir beaucoup de boulot, juste faut bien lui
expliciter ce qu'il doit traiter ; faisons comme la dernière fois. »

Cette page est le point d'entrée d'une session Codex neuve, sans aucune mémoire de nos échanges. Elle
dit ce qu'elle lit, ce qu'elle fait, et où est le travail de chaque périmètre. Ouverte à 09:28, remplie
à mesure que les treize pages de session arrivent (hash et chemin dans le tableau).

## 1. Ce que Codex lit d'abord, dans cet ordre

1. `docs/README.md` : l'organisation des docs et les règles qui ne se discutent pas.
2. `docs/orchestration/11-11-2026/03-reprise-par-codex.md`, §§ 2, 2 bis et 6 : le rôle de l'orchestration, les
   trois niveaux de décision (la session, l'orchestration, Lucie), les amorces par session. Rien n'y a
   changé depuis, sauf ce que la section 3 ci-dessous ajoute.
3. `docs/orchestration/14-11-2026/01-rapport-de-session-c7.md`, bloc « 9 h 1x (16 novembre) : l'état complet
   pour reprendre » : les hôtes, la lib, la garde, l'état de chaque session, les mots attendus de Lucie.
4. `docs/procedures/push-d-un-lot.md` : la passe et le push, mot pour mot (dont « un lot du moteur seul »).
5. La page de passation de la session dont il reprend le périmètre (tableau de la section 4), puis les
   tickets qu'elle désigne, dans son ordre.

## 2. Les règles apprises depuis le 11 novembre (à appliquer, pas à rediscuter)

- **Un seul « à toi » à la fois**, de l'orchestration seule, et c'est le seul mot qui autorise un push de
  code. Une page de docs seules se pousse sans tour, mais jamais entre un « à toi » et son « poussé ».
- **Du code apporté se rejoue**, fichier commun ou pas : la garde (`tools/poste/push_garde.sh`) refuse un
  push dont la passe n'a pas vu le master courant ; en mode pile, on rejoue sur la branche du lot d'avant.
- **Les suites exigées par chemins** (`tools/poste/required_suites.txt`) : un lot qui touche code.rs,
  code_sync.rs, le catalogue, generations.rs ou marque_d_ecrivain.rs joue e2e_code_sync, sinon la garde refuse.
- **Un lot du moteur seul** : la lib du journal est le BUILD_ID écrit par le binaire joué (`CALL BUILD_ID()`),
  jamais écrit à la main.
- **Vert est aucun rouge ; une suite non jouée est rouge ; rouge d'abord** (le témoin vu rouge avant le correctif).
- **Un refus de permission ne se contourne pas** par une autre session ni en reformulant : on le remonte à Lucie.
- **Que de l'anglais** pour ce qu'une machine lit ou qu'un utilisateur tape dans tout code neuf : identifiants,
  /commandes, options, clés de manifeste et de JSON, noms d'outils et de nœuds, tests, unités, scripts, et
  les descriptions d'outils, invites et gabarits du produit. Le français reste pour les docs, les commits, les
  commentaires, les tickets, l'affichage du terminal. Pas de lot de renommage de l'existant ; un alias quand
  on y touche.
- **Les hôtes se relancent sans attendre Lucie** (seul le sudo est à elle) ; **jamais un essai sur le 7878 en
  place** ; card0 interdite ; `-j8` au plus ; `/tmp` est de la mémoire vive ; jamais de pkill par nom.
- **Lucivy** : un tag `v*` publie partout ; il ne se pose qu'au mot de Lucie. 4.3.9 est posée (0825bcd) et en ligne sur crates.io, npm et PyPI ; le pointeur rag3weaver « 4.3.9 » est en file.
- **Identité** : gmail seule, GitHub L-Defraiteur, npm luciformresearch ; zéro attribution d'assistant dans
  les commits ; `gh auth switch --user L-Defraiteur` avant tout appel gh.

## 3. Ce qui attend Lucie (Codex n'y touche pas, il le lui rappelle)

- **L'heure de la fenêtre de la bascule de lib** vers 59ac9c86bc (plan : `docs/optimiseur/16-11-2026/01-plan-de-la-bascule.md`,
  ≈ 20 min sur la tablette, ≈ 40 à luciepc) ; ensuite le lot qui épingle la lib neuve, puis la bascule de ses
  bases (banc : `docs/banc/16-11-2026/01-la-parite-de-la-migration.md`).
- La mise à jour de ses manifestes de tâches après la pose des hôtes ; le rejeu des essais D5/D6 d'Algareth.
- L'import ES externe (codeparsers), toute suppression, toute publication, son sudo.

## 4. Les pages de passation des sessions

| session | page | tickets pour Codex | commit |
|---|---|---|---|
| orchestration | cette page | — | — |
| arbre-principal | en attente | | |
| banc | `docs/banc/16-11-2026/02-passation-pour-codex.md` | 8 entrées : F300 et N rejouée (45 décisions vides), ranger le juge de parité sur master, tolérance de score du comparateur, pile de codeparsers sous le manifeste de Lucie, parité au pointeur lucivy 4.3.9, banc d'étalonnage du débit (page d'abord), mémoire d'un index avec ses vecteurs, témoin de temps sous charge (six tickets neufs) | 5f376f1521 |
| codeparsers | `docs/codeparsers/16-11-2026/01-passation-pour-codex.md` | 12 entrées : finir les mots vides de Rust (lot (5) en branche, 044c7d4250 / b48da05 ; un chemin de std relié à un homonyme, ticket neuf), mots vides Python et C, réexportations, receveurs sans type, définition liée à l'en-tête, oracle, manifestes, crate au nom distinct, vcpkg et find_package, fichier Rust lent, classe derrière une macro, nettoyage du parent | 87c584898d |
| coeur-cpp | `docs/coeur-cpp/16-11-2026/01-passation-pour-codex.md` | 13 entrées par gravité : P1 COMMIT sous index vectoriel ; C1 dictionnaire, C2 lecteur de l'index de clé, C3 optimisticRead ; F1 pk-lookup ; B1 Schema changed, B2 verrou à la liaison, B3 point de reprise ; L1–L3 ; S1–S2 | 96daa8f12 |
| coeur-cpp-hotfixs-tickets | `docs/coeur-cpp-hotfixs-tickets/16-11-2026/02-passation-pour-codex.md` | 14 entrées (70–90 h) : ADD COLUMN sous verrou de schéma, lecture au-delà de la colonne en cache, rang déplacé pendant un CREATE, rows_outside_graph, quarantaine d'une annulation partielle, journal sans borne hors COPY, nœuds liés par leur clé, DROP TABLE et ses pages, RTree, doublon du COPY, cible clean, traces de run_e2e, optimisticRead, tools/shell ; les entrées 2, 3, 5, 7 passent par l'orchestration avant d'écrire (fichiers du cœur) | d61841b448 |
| embarquement | `docs/embarquement/16-11-2026/01-passation-pour-codex.md` | 13 entrées : tolérance de parité, bit selon le lot, cubecl déterministe, refus qui nomme le programme, creux d'un autre modèle, plafond en jetons, cause d'arrêt d'un service, débit par service (les 6 s), anciens morceaux géants, repli CPU des tests, mesure de la carte pleine, granite-278m, résidence (cinq tickets neufs) | 80ba4f661c |
| everything-declarative | `docs/everything-declarative/16-11-2026/01-passation-pour-codex.md` | 13 entrées : llama-server, access/provider (alias au 29 nov.), erreurs de libération masquées, contrat de reçu d'un accord, estimation, genre d'une méthode d'un autre fichier, ALTER (lib 101ff50a6), sept témoins instables, morceaux refusés jamais repris, redécoupage après la bascule, postgres à générations, namespaces, cinq tickets à fermer sur mesure (deux tickets neufs) | d663340ef |
| lucivy | `docs/lucivy/16-11-2026/02-passation-pour-codex.md` | 6 entrées : le bm25 seul de la migration sous 4.3.9 (avec la reprise du pointeur s'il n'est pas sur master), la compaction qui grossit un dictionnaire sans texte mort, un ulp d'écart en parse, indexed:false ignoré, préparer la 4.3.10 sans la taguer, le chemin sans positions (lot long) ; trois tickets neufs ; 4.3.9 en ligne sur crates.io, npm, PyPI | 9c1d71fb11 |
| memoire | `docs/memoire/16-11-2026/01-passation-pour-codex.md` | 14 entrées de la moins à la plus bloquée (35–40 h sans attente) : l'usage du découpage au total, compose_restart au-delà de 2 000 apports, la borne query_chars côté embarqueur, les 15 tickets relus contre master (trois tickets neufs, 2f8270612) | db6533bf8d |
| optimiseur | `docs/optimiseur/16-11-2026/02-passation-pour-codex.md` | 7 lots : témoin de poste, témoin de run_e2e qui compte l'attente, tools/shell en voie C++, juge known_red intermittent, base un-seul-sens au témoin, épingler la lib après la bascule, défaut de 40G (quatre tickets neufs, six fermés) | 499cb10de6 |
| rag3weaver-archi | en attente | | |
| recherche | `docs/recherche/16-11-2026/03-passation-pour-codex.md` | 10 entrées : état d'index et orphelins, search_code avant l'index, CSV d'un COPY tué, filtre e2e vide, témoins du chat sous charge, arrêt par une connexion par run, cinq suites à résidus, banc de charge, juge Gemini, deux tickets neufs après 2a′-3 | 04102a27f6 |
| tty | `docs/tty/16-11-2026/02-passation-pour-codex.md` (+ `03-knowledge-dump.md`) | 7 lots (25–35 h) : messages entre sessions dans le terminal, commandes et options en anglais, mode plain et fin d'un pas relancé, copie de l'hôte par un champ, terminal d'un agent pair, style du thème, frappe avant l'invite (cinq tickets neufs) | d1ea5a947a |

Chaque page a quatre parties : l'état du périmètre ; le travail pour Codex dans l'ordre, par tickets réels
(quoi faire, comment prouver, taille, blocage) ; ce qui attend Lucie ; les règles du périmètre.

## 5. Comment Codex travaille un ticket

1. Un worktree à lui (`git worktree add ~/git_workspaces/rag3db-cx-<lot> -b cx-<lot> origin/master`), un
   target cargo à lui sous `~/.cache/rag3weaver-build/target-cx-<lot>`, `git submodule update --init`.
2. Le témoin rouge d'abord, commité tel quel ; puis le correctif ; puis la passe par
   `passe_epinglee.sh <worktree> <NOM de journaux> <filtre>` sous `POSTE_MEM_MAX=32G` (lib courante des
   hôtes : lire `RAG3DB_LIB` du dernier lot atterri ; `RAG3WEAVER_MOTEUR_ANCIEN=1` tant que les hôtes sont
   sur 373b5477b).
3. « prêt » à l'orchestration avec la tête, le résumé (première ligne = la tête) et le verdict ; push au
   seul « à toi », par la garde ; « poussé » avec le hash et les trois contrôles.
4. Le ticket fermé passe dans `tickets/closed/` avec la preuve ; une ligne dans le rapport du jour de la
   session ; `docs/nouveautes.md` si c'est une étape atteinte.
