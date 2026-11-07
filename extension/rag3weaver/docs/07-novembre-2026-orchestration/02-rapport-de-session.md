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

## La soirée du 7 (écrit vers 17 h 50, après une compaction)

**Atterris sur master dans l'ordre** : lot 3 bis des hooks (archi, 120695b37) ;
vues 6 (08d10babf), 7 (7897d2c96), 3b (a4ffea748) et la ligne de nouveautés
(23d8d3c5a) ; lucivy 4.3.1 publiée sans patch (G, 37904f103) ; chercher sans
dossier (hooks-essai, e2d5c9c88, nouveautés e72044b41) ; docs : ticket du
figement (16bcd78bc), bascule-de-lib avec le contrôle des deux sens (9dc626c70),
essai Argon (6d6ceba21), rapport de recherche (9d38951fa), tickets du plein
texte long (13d0b18b9, 68e3bf4e6), filtre du signal creux (2d7f11067).

**Deux poses chez Lucie par G** : 16 h 31 (3 bis, binaire 24faff78) et 17 h 22
(37904f103, binaire 10878f01 : l'IDE React entier ; `rag3weaver-backend ide`
pour entrer). Lib épinglée 04063ee06 des deux côtés, pas de bascule 43.

**Un bloquant neuf au moteur** : VectorSearchDuringInsertTest se fige à la
cinquième passe (inversion d'ordre de verrous entre la compaction CSR de
c44af8031, index puis blocs, et takeGenerationFor de V2-2, blocs puis index).
Lot ordre-des-verrous-2 (b16377ea0) du cœur, 30/30 rejeux sans figement, push
vers 18 h ; le retrait, la projection, la lib unique et la bascule 43 attendent
derrière. Leçon : une suite « lente » se joue sous délai, un délai dépassé est
un figement.

**Décisions de Lucie** : des tickets groupés par famille pour le cœur et
hotfixs (plus un par jour) ; la session « tty » (interface terminal de l'agent
de code : ratatui + couche déclarative, crossterm, deux modes d'emplacement,
seul le design « Daemon Code Summoning » repris de ragforge a25f3ecf9) ; un pont
entre ses sessions Codex et les sessions Claude (Codex CLI 0.167 installé ;
Claude → Codex prouvé par `codex queue --thread <uuid>`, réponse écrite à
16:53:36 ; Codex → Claude par les **canaux** de Claude Code, serveur MCP
`rag3weaver-backend canal`, flag `--dangerously-load-development-channels
server:rag3weaver-canal` à sa prochaine relance, pas ce soir) ; models.sparse
autorisé dans son backend.json à condition que le creux ne s'embarque qu'au
surlignage (prouvé : 0/0/0/1, veille comprise) ; le rappel des hooks est une
pré-recherche à UserPromptSubmit sur des mots-clés techniques extraits sans
modèle, budget 1 s mesuré, repli détaché, déjà-vu par session (silence complet
si tout est connu), juge de pertinence qui ôte ; elle veut être appelée quand
il y a quelque chose à regarder.

**Le rappel lent, mesuré** : plein texte flou lucivy seul (1,9 s à 33 s selon la
taille du prompt sur deux fiches), embarquement et dense < 140 ms ; requête
extraite de dix mots → bm25 51–58 ms (lot hooks-3-ter (a), 096db952b, en
passe). Deux tickets : le nœud qui survit à son client ; la requête floue dont
le coût croît avec les termes (recherche, borne mesurée ; lucivy aussi).

**Codex à l'essai** : relecture lecture seule du diff du creux (223 622 jetons
dont 175 872 en cache, ~10 min) : un vrai défaut (garde de features du témoin
sans burn-embedder), un point jugé (dual sans signal : voulu, commenté), une
étape de témoin ajoutée (réindexation à 0) ; tout intégré par recherche.

**luciepc** : plafond 4 lourds ; la mesure exclusive de l'arbre bloquait la
file (retirée, M3–M5 à une fenêtre creuse) ; chaque session un target.

**La file au moment d'écrire** : boîte adressée (ED, dc5bb3a78 fondu, en passe)
→ tty-1-3 (43cc6a99f/3a9ed3307) → hooks-3-ter (a) puis pose par G → 96
(politique call_model accordée avec ED, puis run_e2e.sh, puis pas 3) → recherche
(creux-par-l-hote-2, filtre du creux, borne mesurée, « mes recherches ») →
archi (pont : mail, codex_threads, canal, hooks pending/seen ; puis 3 ter (b))
→ whois (73, 40da57af8) → codeparsers (garde de match, pointeur) → hooks-essai
(journal de l'hôte horodaté) → ED (étanchéité, désambiguïsation des noms).
Cœur : verrous ~18 h, puis instruction-en-echec (ed59ee988), retrait,
projection ; arbre : lib unique ; G : bascule 43 ; hotfixs : (B) puis G1…G6.

## Mise à jour de 18 h 10

- **Atterris depuis 17 h 50** : la boîte adressée (ED, b9b47ee12), tty-1 (client
  SSE du terminal + `policy` dans describe, cfc5cc8e8, nouveautés 69aa15eef),
  ordre-des-verrous (cœur, e526cc4b6 / 5f0b79466, ticket fermé 9c613605b),
  sept tickets du chat de Lucie (a4012758c, 6265b989d, 18f7cdbfb, 016878e37),
  ticket de l'index des tickets (hotfixs, 2cc565591), tickets de la requête
  longue mesurée (e7b1c8395) et de la garde de match (2721843b5), procédure
  « un contrôle qui ne juge rien est ROUGE » (b99b67de0).
- **Décisions de l'orchestration** : la lib unique se bâtit sur master après
  les verrous et instruction-en-echec-2 (87cdd7fae, en passe), sans attendre le
  retrait ni la projection, qui suivront en libs ordinaires au format 43 ;
  arbre étapes 1 à 5, G étapes 6 à 11 (contrôle des deux sens compris, une
  seule main pendant l'arrêt des hôtes) ; bascule ce soir si la lib est là
  avant 22 h, sinon demain matin. Recherche pousse ses trois lots verts en un
  seul passage (recherche-trois-lots). L'index des tickets passe à une forme
  générée (en-tête obligatoire, index non suivi, `--lot <base>` dans la passe),
  lot de hotfixs après (B). Un target cargo par lot, jamais deux passes dessus.
- **Retours de Lucie sur l'IDE posé à 17 h 22** (chat) : ordre d'arrivée des
  parties (mots diffusés, appel, mots, réponse finale à la fin), heure prise à
  l'appel, pas de flicker de scroll (flux et dépliage), le résultat d'un outil
  déplié, écran large (barres aux bords, chat fluide, aide sur une ligne),
  usage du modèle en barre latérale avec jauge sur le contexte configuré,
  tâches de l'archiviste dans la barre, chat pleine hauteur ; et la direction
  visuelle : un terminal d'avant-garde, l'animation « Daemon Code Summoning »
  de tty quand le modèle réfléchit, mêmes tokens IDE/terminal, page de
  direction soumise à Lucie avant les composants. Lot « le chat tient
  l'ordre » après l'étanchéité ; la direction visuelle ensuite.
- **Hooks** : hooks-3-ter (a) prêt (c9d5e388b → rebase sur cfc5cc8e8, passe en
  cours), pose par G pour la session d'essai dès le hash ; (b) écrit, témoins
  en cours. Le journal de l'hôte horodaté de hooks-essai (634c16b56 : tube non
  bloquant, patience 50 ms mesurée, pire 1,14 ms) attend son tour.
- **96** : le pas 1 (662dda2cc) n'a jamais servi (modèle posé sur le catalogue
  que les graphes d'outil ne lisent pas), vu par son témoin au premier rejeu
  grâce au correctif de run_e2e.sh ; lot-96-pas1-politique (call_models,
  Route/SplitSections dans la base, correctif du service), verdict ~18 h 30.
- **Codeparsers** : garde de match corrigée (672a0d5, deux endroits), sonde
  sur code-lourd en cours, lot du pointeur ensuite.
- **File** : archi 3 ter (a) → tty-2-3 → 96 → recherche (trois lots) → archi
  pont (mail, codex_threads, canal, hooks pending/seen) → whois → codeparsers
  (pointeur) → hooks-essai (journal) → ED (chat, direction visuelle) → archi
  3 ter (b) → hotfixs (index des tickets).

## Mise à jour de 18 h 40

- **Posé chez Lucie à 18 h 15** : hooks-3-ter (a) (7b41a1a04, binaire 963efa20).
  Trois tours réels de hooks-essai : rappel 93-334 ms ; mais l'extraction
  « plus longs d'abord » a pris le méta-discours, une notification de tâche a
  rendu une fiche hors sujet sans seuil, et la bonne requête n'a rendu que
  0,37. Cause lue au journal des runs par archi : le hook lisait `chunk.score`
  (le morceau dense) et ignorait `signals`, où bm25 avait touché « régime GPU »
  à 1,39. Le (b) (ae0d2d3d9, vert, en push) lit les signaux par ligne, le
  plein texte fait le candidat, le dense seul est « non jugé » jusqu'à mesure
  sur une mémoire peuplée, un rendu par tour, déjà-vu après filtre, tours de
  notification sans rappel, vocabulaire de la mémoire d'abord, mots courts du
  code gardés, ticket pour recherche (score comparable entre signaux).
- **Le lot du chat passe à hooks-essai** (formes et relecture par everything
  declarative, qui garde l'étanchéité) : lot A Rust prêt (chat-ordre-rust,
  660daaec2 : at_ms à l'appel/fin, résultat dans ToolCallFinished,
  context_len, `role` d'outil relayé dans RunStarted), lot B React ensuite
  (parties dans l'ordre d'arrivée, scroll, résultat déplié, colonne fluide,
  pleine hauteur ; sous chaque conversation : récap d'usage et fiches écrites
  pendant la conversation, route `GET /ide/chat/fiches?backend=` par les
  Sources à `session`). 96 posera `session` sur les Sources sans session
  (après M4). Direction visuelle (terminal d'avant-garde, thème « Daemon Code
  Summoning » de tty, mêmes tokens IDE/terminal) : page d'ED après le chat,
  accord de Lucie avant les composants ; tty lui demande l'accord du thème.
- **Décisions** : lib unique dès verrous + instruction-en-echec-2 (87cdd7fae,
  passe) ; arbre 1-5, G 6-11 ; bascule ce soir avant 22 h sinon demain ;
  Lucie : oui à l'effacement des instantanés bascule-04063ee06 et
  bascule-66994dd43 après réveil vert ; règle « tant que vous ne vous empêchez
  pas de progresser » transmise à toutes les sessions (pas de poste mesure
  pendant la file, un target par lot, place proportionnée).
- **Atterris** : tty-2 (1fc1e24bd, vue inline), nouveautés tty (69aa15eef,
  0a3e5752e), rapport tty (d7c1b6d5c). **Prêts** : recherche-trois-lots
  (9c0b785f3, vert), codeparsers/garde-pointeur (56293d739 : 19 arêtes de plus,
  0 de moins, ticket « marque nom » ouvert), hooks-essai journal-hote-horodate
  (634c16b56 : tube non bloquant, patience 50 ms mesurée, pire 1,14 ms),
  96 lot-96-pas1-politique-2 (le pas 1 n'avait jamais servi ; verdict ~18 h 40),
  hotfixs (B) csr-listes-temoin-3 (banc en cours, second rouge de charge
  C7_RandomMix/Thread_Crash inscrit à probabilistic.txt, ticket du cœur).
- **File** : archi (b) → 96 → recherche → hooks-essai (journal, chat A) →
  codeparsers → archi pont → whois → ED (étanchéité, reprise après la mort de
  l'hôte, direction visuelle) → hotfixs (B puis index des tickets généré).
