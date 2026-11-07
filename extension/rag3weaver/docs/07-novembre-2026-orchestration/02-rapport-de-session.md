# Rapport de session de l'orchestration, 6 et 7 novembre 2026

Écrit à 16 h 30 le 7 novembre, avant le plein du contexte. La session est
l'orchestration rag3db (clé c7, nommée « rag3db-74 » ce jour), sur Fable 5.1 ;
toutes les autres sessions sont passées sur Opus 5.5 à 15 h 45. Lucie écrit en
français, je réponds en français ; les identifiants restent en anglais.

## D'où on part

Le 25 octobre au soir, la nuit a atterri la plus grande partie de la file ; à
0 h 45 le 26, Lucie a tout coupé pour deux semaines. Dix branches vertes sont
restées non fusionnées. Le 6 novembre, deux visions sont écrites avec Lucie :
« le dépôt qui sait pourquoi » et « un index par dépôt, des vues par
worktree ». Le 7 au matin, Lucie relance toutes les sessions avec trois
consignes : nommer les worktrees obsolètes, libérer le disque (le portable
d'abord, 1 To), pousser et fusionner tout ce qui est sans bloquant réel.

## Ce qui a été fait le 7 novembre (ordre chronologique)

1. **Inventaire et ménage** (matin). 24 worktrees ici dont 14 fusionnés, 46 à
   luciepc dont 32 fusionnés ; ~/.cache/rag3weaver-build à 129 Gio ici et 338
   là-bas (un target par worktree) ; les notes de codeparsers à 36 et 93 Gio ;
   21 Gio de copies LOCKS. Chaque session a rendu son verdict par `git cherry`
   (le patch, pas l'ancêtre), `lsof`, diffs non commités archivés, puis effacé.
   Portable : 300 → 357 Gio libres ; luciepc : 462 → ~600. Ce qui reste de gros
   sur le portable est à Lucie (Steam 183, modèles 135, baloo 25, sairen 23) ou
   vivant (targets des sessions). Refus de permission relevés : la suppression
   de branches distantes (`git push --delete`) est refusée aux sessions comme à
   moi (« Git Destructive ») ; G est refusé même sur `git worktree remove`.
   Rien n'a été contourné.
2. **Atterrissages** : anthropic-llm-2 (90df16ace, connect_llm → Arc<dyn Llm>,
   feature au jeu large), hooks lots 1, 2, 3 (2d3ecad1e, d42787227, 8f60895b8),
   poste à N détenteurs (e006f4cff), INCLUDES (9fd37ab1c), M4 pas 2
   SplitSections (6e1dc868e), whois (2f4ed7841), le creux (8f3659d8a), le bloc
   annulé du cœur (42b85dc62). En file à 16 h 30 : vue 5 (push en cours), lot 3
   bis des hooks (8ff57fd29, prêt), vues 6/7/3b, lucivy-publiee-2 (2166f20ac,
   vert), chercher-sans-dossier (hooks-essai, vert sur 3b-2), M4 pas 3, le
   correctif models.sparse du creux, puis le cœur (projection-lecture-seule-5
   08eebb0ff, v22-retrait-3 69be27856, fenêtre vers 17 h 15), puis la lib
   unique et la bascule 43 (arbre, G).
3. **Le bloquant du jour, trouvé et corrigé en une matinée** : un lecteur voyait
   les lignes d'un COPY annulé (4/60 sur master). Bissection → V2-1b pièce 2
   ouvre la fenêtre ; cause amont : `ChunkedNodeGroup::rollbackInsert(0)` fait
   `versionInfo.reset()` (= « tout validé ») sur un bloc entièrement annulé.
   Correctif : marquer les lignes annulées, jamais effacer les versions, nœuds
   et relations ; témoin déterministe rouge → vert, probabiliste 60/60. Ligne de
   nouveautés par le cœur.
4. **La base de code de Lucie est propre** : 0 écart sur 18 tables de liens après
   douze jours de montre (releve-7-nov). Le témoin (B) de hotfixs est vert dans
   quatre cas (table tardive, deux écrivains, SIGKILL ×8, drains annulés) sur
   master et 58b46a9bf ; dernier ingrédient en cours : le COPY de relations
   refusé faute de tampon. Si vert : (B) se ferme avec deux garde-fous versés au
   dépôt (le témoin à cinq cas, `tools/check_rel_directions` dans la procédure
   de bascule).
5. **Les hooks de Claude Code** : page de conception (archi), trois lots sur
   master, le quatrième prêt. Lucie a lancé la session d'essai « hooks-essai »
   dans ~/git_workspaces/rag3db-hooks-essai (script `--worktree hooks-essai
   --poser`). L'usage réel a trouvé : le genre `handle` absent de la mémoire (96
   a créé agent/alias/handle/same_as à 14 h 55) ; author/via nuls ; une source
   acceptée hors forme ; `cited_by` lu à plat (aucune session jamais retrouvée)
   ; le budget global de 5 s débordé par des appels en série ; la mémoire de
   Lucie n'a que deux fiches ; l'échelle du score à nommer, le seuil, sa propre
   fiche rendue comme résultat. Exigence de Lucie : corriger à la volée sans
   relancer Claude → settings ne porte qu'une commande stable, binaire,
   hooks.json et gabarits (templates/hooks/) relus à chaque appel,
   `hook --rejouer`. Vérifié : l'ancienne forme `hook --config` et la nouvelle
   `hook <Événement>` rendent le même bloc.
6. **Décisions de Lucie du jour** : le moteur continue en parallèle, famille
   correction d'abord, un ticket par jour, listes données au cœur et à hotfixs,
   le banc comme tableau de bord ; une session Claude d'essai seule avec les
   hooks, qui travaille un lot réel ; un worktree est jetable (effacement à la
   fusion, refus d'un worktree périmé, horizon des mini-machines à parts
   égales) ; l'équipier comme **agent de code fonctionnel d'abord** (outils
   étanches, commandes par la porte déclarée, Claude Code et sous-sessions dans
   les deux sens, `tool_issues` à chaque pas, la mémoire après) ; essai borné de
   `gemini-4-argon` (GA en région global pour son projet, countTokens 200 ; un
   run d'e2e_equipier, jetons et heure notés, Lucie lit sa facturation le 8) ;
   les données Magic de luciepc restent ; `poste lourd` à 4 places à luciepc.
7. **Procédures écrites** : docs/procedures/push-d-un-lot.md (règles de passe et
   de push, garde de rebase, paquets React, relire master juste avant le push,
   suites e2e sous le jeu large et une suite non jouée est ROUGE, le push efface
   worktree et target) ; docs/procedures/bascule-de-lib.md.
8. **Visions enrichies** : cinquième et sixième difficulté (cohérence
   temporelle entre couches, artefacts dérivés à état, génération épinglée,
   témoin cruel ; configuration de bâti), `successor_of`, résumés de scopes,
   RagForge en trois lignes, la thèse (la valeur augmente quand les agents
   deviennent interchangeables), parts égales par mini-machine.

## Ce qui attend Lucie

- Quatre mots : les branches atterries à effacer d'origin (cinq sessions
  bloquées ; critères : tête ancêtre de master, cherry sans +, ou remplacée par
  une -N) ; les worktrees de G à retirer ; rag3db-service à luciepc ; le
  `target/` de 208 Gio d'août dans son arbre principal de luciepc.
- Sa facturation Vertex le 8 au matin, pour Argon.
- Les trois questions de la page roadmaps et les trois de la page identité
  (96) ; les tickets lucivy (apostrophe typographique, clés ignorées) ;
  `~/memoire-longue/` pris par défaut le 25.
- Le lundi : le relevé de la base neuve refait après la bascule 43 ; la semaine
  « moteur ennuyeux » ; le prototype vertical des vues par scope (dimanche si la
  file est vide).

## Ce qui reste en vol au moment d'écrire

Vue 5 → 3 bis hooks → vues 6/7/3b → lucivy-publiee → chercher-sans-dossier →
M4 pas 3 → models.sparse ; cœur : projection et retrait vers 17 h 15, puis lib
unique (arbre) et bascule 43 (G, instantanés, courant, hôtes) ; hotfixs : le
cinquième cas, puis les mesures de (A), puis sa liste ; archi : l'essai Argon,
puis le pont superviseur ↔ équipiers ; everything declarative : les six lots de
l'équipier agent de code ; recherche : la vue « mes recherches » ; 96 : le
rangement de la mémoire de Lucie (procédures, page 08, fiches de session,
visions, nouveautés, les fiches de la mémoire Claude Code) sur une copie puis
sur la sienne à son mot.
