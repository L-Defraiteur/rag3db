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
- **Lucivy** : un tag `v*` publie partout ; il ne se pose qu'au mot de Lucie. 4.3.9 est posée (0825bcd).
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
| banc | en attente | | |
| codeparsers | en attente | | |
| coeur-cpp | `docs/coeur-cpp/16-11-2026/01-passation-pour-codex.md` | 13 entrées par gravité : P1 COMMIT sous index vectoriel ; C1 dictionnaire, C2 lecteur de l'index de clé, C3 optimisticRead ; F1 pk-lookup ; B1 Schema changed, B2 verrou à la liaison, B3 point de reprise ; L1–L3 ; S1–S2 | 96daa8f12 |
| coeur-cpp-hotfixs-tickets | en attente | | |
| embarquement | en attente | | |
| everything-declarative | en attente | | |
| lucivy | en attente | | |
| memoire | en attente | | |
| optimiseur | en attente | | |
| rag3weaver-archi | en attente | | |
| recherche | `docs/recherche/16-11-2026/03-passation-pour-codex.md` | 10 entrées : état d'index et orphelins, search_code avant l'index, CSV d'un COPY tué, filtre e2e vide, témoins du chat sous charge, arrêt par une connexion par run, cinq suites à résidus, banc de charge, juge Gemini, deux tickets neufs après 2a′-3 | 04102a27f6 |
| tty | en attente | | |

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
