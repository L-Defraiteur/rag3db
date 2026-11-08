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

## Mise à jour de 20 h 05 — la bascule 43 est faite

- **Bascule 43** (G, hôtes coupés de 19 h 21 à 19 h 57) : lib unique bâtie par
  l'arbre sur master 17a094d12 (2 min 30), verdict format 43 vert en 4 min,
  ABI changée contre 04063ee06 (3 symboles retirés, 17 ajoutés, 3 vtables) ;
  instantanés avant-43 aux deux postes, contrôle des deux sens sans écart sur
  les 4 bases, `courant` = rag3db-build-17a094d12…, hôtes réveillés, dense
  filtré par langage et dossier vert sur la base de code de Lucie, témoin de
  l'IDE vert sur 7970. Toute passe neuve s'épingle sur cet id, MOTEUR_ANCIEN
  vide. Deux réserves : `models.sparse` posé chez Lucie mais le surlignage
  creux se replie, le binaire des hôtes étant sans burn-embedder (question à
  recherche : le service bge-m3 rend-il les positions des jetons ?) ; et
  l'effacement des anciens instantanés (bascule-04063ee06 1,9 Go,
  bascule-66994dd43 1,9 Go, locks-25-oct 4,4 Go, avant-43-controle 3,2 Go ici ;
  3,5 Go × 3 à luciepc) refusé à G par sa permission, non contourné : à la
  main de Lucie.
- **Atterris depuis 18 h 40** : hooks-3-ter (b) (20fdc9569, posé 18 h 48 :
  premier bloc juste dans la session d'essai, déjà-vu et notification justes),
  lot de 96 (ec9a333e3 : call_models, le pas 1 servait enfin), recherche
  trois-lots (77d0b0676), index des tickets généré (hotfixs, 345eb5a7c),
  (B) fermé (094aee5f8), instruction-en-echec (cœur, 17a094d12), G1 des
  décalages provisoires (hotfixs, c4f8610b9 : un COPY de relations dans la
  transaction qui a créé ses nœuds reliait les nœuds d'un autre écrivain
  validé entre-temps ; inatteignable dans l'hôte par construction, une
  connexion écrivaine par backend ; témoin du second processus chez
  recherche), l'étanchéité de l'équipier (ED, 75d8d6d1e), pages 02 et 03
  d'ED (session multi-dépôts, vocabulaire git, l'atelier ouvre les dépôts),
  tty (vocabulaire, 55402c3e5), rapports de l'arbre (b1f4b2252).
- **Décisions de Lucie** : juge Flash actif dans sa session d'essai sous
  surveillance (`recall_judge: false` pour couper) ; le produit parle en
  dépôts, worktrees, branches, PR, et « projet rag3weaver » pour le
  déclaratif pur, jamais « backend » à l'écran ; sous chaque conversation sa
  récap d'usage et ses fiches ; une session tient N worktrees, plusieurs par
  dépôt ; l'usage pro d'abord (« l'atelier ouvre les dépôts », page 03 d'ED
  à son accord) ; gabarits de la mémoire : version épinglée + `templates
  upgrade` (M8 de 96) ; règle « ne pas s'empêcher de progresser ».
- **File** : archi (c) → hooks-essai (journal, chat Rust, chat page, en un
  passage) → codeparsers (pointeur) → 96 (run_e2e.sh, pas 3) → arbre (juge
  Claude Code, whois) → archi (pont : mail, codex_threads, canal, genre
  message) → ED (reprise après la mort de l'hôte, direction visuelle) ;
  cœur : projection (rouge de charge inscrit, ticket 10-10) puis retrait →
  libs ordinaires ; hotfixs : RTree, G3 ; recherche : second-ecrivain, puis
  « mes recherches » ; G : pose suivante (étanchéité + (c) + chat) chez Lucie.

## Mise à jour de 21 h 10

- **Atterris depuis la bascule** : hooks-3-ter (c) (archi, bd1a65dec), les témoins
  V2 du banc (072c467d7), la projection en lecture seule (cœur, e86d2f688,
  moteur cee5c2d41), et les **trois lots de hooks-essai** (3d09b32c2, 27
  commits, huit tickets fermés : le journal de l'hôte horodaté et attribué, le
  chat Rust (heure à l'appel et à la fin, résultat dans ToolCallFinished,
  context_len, `role` d'outil et de réaction), la page du chat (parties dans
  l'ordre d'arrivée, scroll, résultat déplié, colonne fluide, pleine hauteur,
  barre « Dépôts / Mémoire / Projets » avec récap d'usage et fiches écrites sous
  chaque conversation, plus un mot « backend » à l'écran) ; passe entière verte
  avec les trois suites Gemini). G pose chez Lucie.
- **Prêts dans la file** : reprise après la mort de l'hôte (ED, equipier-reprise-3,
  rouge joué 109 s), pointeur codeparsers (garde-pointeur-2), run_e2e.sh et pas 3
  (96, le pas 3 en essai complet : trois défauts de forme des gabarits corrigés),
  juge Claude Code (arbre, 62e98288c), spans du creux (embarquement, 90b86d1c9 :
  le service rend les positions des jetons ; défaut de bâti du binaire du service
  trouvé et corrigé), second-ecrivain (recherche, 93c68f817 : le second processus
  écrivain est refusé par le moteur, le message nomme l'écrivain en place),
  le pont (archi, 781c3232b : mail, fils Codex, canal, règle de la boîte).
  Retrait du cœur vers 21 h 25. Lib ordinaire demain matin (c4f8610b9 +
  projection + retrait) ; d'ici là passes sur 17a094d12 + MOTEUR_ANCIEN=1 dit.
- **lucivy 4.3.2** (session lucivy, PR #21) : passage exact long retrouvé (raté dès
  44 octets), flou long 15,8 s → 96 ms (préscan O(n²) du planificateur des
  briques), apostrophe typographique, clés inconnues nommées ; format d'index
  inchangé ; un seul tag v4.3.2 sur main quand la CI est verte, au mot de Lucie
  (donné dans sa session, confirmation demandée ici). Puis rag3weaver monte à
  4.3.2 (recherche ; témoin de la ligne collée de 120 caractères, rouge aujourd'hui,
  ticket produit).
- **Leçons de la soirée** : le verrou `poste push` ne couvre que le dernier pas
  (c60cd7e9c ; archi l'a tenu 44 min avec une passe dessous) ; un saut faute
  d'identifiants n'est pas vert ; une passe figée pendant le réveil des hôtes
  venait de tests qui attendaient un service réel sans délai (ticket d'hygiène) ;
  un test du banc sorti vert par hasard reste un rouge connu.

## Mise à jour de 21 h 45

- **Posé chez Lucie à 21 h 21** : 3d09b32c2 (le chat refait, l'étanchéité, le (c)
  des hooks, le journal de l'hôte horodaté) sur la lib 17a094d12 ; page du chat
  rendue, outils dépliables avec arguments et résultat. Réserve : la barre dit
  « PROJETS » pour ses deux backends : la détection lisait le dépôt du manifeste,
  or son manifeste vit dans sa configuration et pointe le dépôt par
  `workspace.root` ; correctif dans le lot suivant de hooks-essai, et la nature
  d'un backend se déclare désormais par son gabarit (`kind`), la détection
  git n'étant qu'un repli. `bilans.memoire` absent de son manifeste de code :
  ni juge ni fiches écrites sur son chat (question posée à Lucie).
- **Atterris** : le retrait des générations (cœur, b3bc6e479 ; projection
  e86d2f688 ; lib de demain matin avec c4f8610b9, ABI changée), tty-3
  (f66e72d31 : Daemon Code Summoning en cellules, --depot/--tache), les
  nouveautés du chat (178b5dbbe), docs du cœur (dcaea137f).
- **Trouvé et corrigé par hooks-essai, en passe** : le « premier jeton perdu »
  était un défaut du flux de l'IDE (`Flux::publier` tirait l'id hors du verrou
  des abonnés, `Abonne::envoyer` sautait tout id ≤ au dernier : deux relais qui
  se croisent perdaient un événement en silence, jetons et lignes de boîte) ;
  témoin à 4 fils × 2 000 (7 577/8 000 avant), correctif 4c0237a3a accordé par
  ED et tty. Dans le même lot : la détection dépôt/branche par `workspace.root`,
  et deux tests d'agent_ports instables sur master par l'at_ms (trouvés par
  embarquement, rouges 1/3).
- **Service creux** : les positions des jetons rendues par /embed_sparse
  (`spans: true`), unité bge-m3 redéployée à 21 h 18 (15 s, parité du dense),
  témoin par le service vert ; lot embarquement cbdf40287 prêt, pousse après le
  correctif d'agent_ports ; les granite tournent sur l'ancien binaire en mémoire
  jusqu'à un redémarrage annoncé avec parité.
- **File** : ED (reprise, en passe) → hooks-essai (premier-jeton + kind + at_ms)
  → codeparsers → 96 (run_e2e.sh) → arbre (juge + graphe cité, débloque
  l'archive) → 96 (pas 3) → embarquement → recherche (second-ecrivain,
  HighlightNode par les spans, montée lucivy 4.3.2) → archi (pont). lucivy :
  PR #21, tag au mot de Lucie (confirmation demandée). Décisions en attente de
  Lucie : confirmation lucivy, thème partagé daemon.json, page 03, instantanés
  et branches distantes (effacements refusés aux sessions par permission),
  `bilans.memoire` sur son manifeste de code.

## Mise à jour de 21 h 55

- **Ordre de la file changé** : hooks-essai (premier-jeton, 7b135dc53) pousse
  devant la reprise d'ED : son lot corrige le flux de l'IDE, l'instabilité
  d'agent_ports (at_ms, rouge 1/3 sur master, trouvée par embarquement) et la
  nature des backends. Forme tranchée : `kind` (code | memory | project) déclaré
  par le gabarit et lu dans le manifeste, repli par l'espace de travail ou le
  dépôt (`kind_source: fallback`, infobulle), dépôt et branche par
  `workspace.root` puis le dossier du manifeste ; plus de reconnaissance par
  `bilans.memoire`, qui n'allume que le juge et les fiches (mot de Lucie
  demandé). Chez Lucie après la pose : code sous « Dépôts » (rag3db · master),
  mémoire sous « Projets » jusqu'à M8.
- **Demain matin** : lib ordinaire bâtie ce soir par l'arbre (c4f8610b9,
  e86d2f688, b3bc6e479 ; check_rel_directions sous la lib neuve sur des copies
  des instantanés avant-43 ; rebâtie à 8 h si un commit moteur arrive la
  nuit) ; fenêtre G à 8 h 30 : hôtes rebâtis, arrêt, granite redémarrés par
  embarquement avec parité, services, courant, réveil ; pas d'instantané.
- **Cœur** : ticket de la validation à moitié appliquée fermé comme non
  reproduit (5b35a5a70), témoin HalfAppliedCommit en garde ; passe à
  l'annulation sous tampon plein. **Recherche** : creux-par-les-spans écrit
  (un appel pour la requête et les morceaux, repli dit), push après
  embarquement. **tty** : tty-3 sur master (f66e72d31).

## Mise à jour de 22 h 30

- **Atterri** : premier-jeton (hooks-essai, 23757c7d6, 11 commits) : le flux de
  l'IDE ne perd plus de trame quand deux relais se croisent, la nature d'un
  backend vient du `kind` de son gabarit (repli dit par `kind_source`, dépôt et
  branche par l'espace de travail), agent_ports compare sans horloge. G le pose
  chez Lucie (dernière pose du soir). Groupe 1 du cœur clos : deux tickets fermés
  comme non reproduits avec témoins en garde (5b35a5a70, 15f16bcdc). Banc :
  build-asan effacé, page du jour (3532676f5). G : 18 Gio de targets effacés,
  liste des targets de luciepc par session dans sa page (l'arbre : 32 Gio
  d'octobre à effacer).
- **Mots de Lucie** : « rien ne me gêne, on focus boulot ce week-end, tout pour
  avancer vite, en surveillant les disques » ; « on limite luciepc seulement
  quand quelqu'un veut le silence pour un bench » → lourds-max de luciepc 4 → 6,
  la mesure exclusive au mot de c7 ; ligne disque à chaque pose (portable 352
  Gio libres, luciepc 580) ; géo : « ça viendra, pour le moment osef », graine
  notée (le temps comme géométrie 1D, les canevas, la géographie vraie, la ville
  du code en marketing) ; lucivy : une seule publication quand tout est fini,
  tag sur main.
- **Verts, en attente de leur tour** : reprise d'ED (à toi sur 23757c7d6),
  codeparsers, 96 (run_e2e.sh), arbre (juge + graphe cité ; lib de demain bâtie
  ce soir), embarquement (spans, à rebaser), recherche (second-ecrivain,
  creux-par-les-spans : 139 ms contre 429 avec le tokenizer local), archi (pont).

## Mise à jour de 22 h 35

- **Lib de demain : « fini » à 22 h 08** (arbre) : rag3db-build-7cd11e5fa… (c4f8610b9,
  e86d2f688, b3bc6e479), verdict format 43 vert, ABI : vtables Table/RelTable/
  NodeTable, contrôle des deux sens sous la lib neuve sur copies d'avant-43 :
  aucun écart sur les quatre bases ; rebâtie à 8 h seulement si un commit moteur
  arrive la nuit ; fenêtre G 8 h 30 sans instantané, granite redémarrés avec parité.
- **Reprise d'ED sur master (4ae8c93a8)**, mais poussée en raccourci : les suites
  chat, lectures, tty, juge, équipier n'avaient pas de verdict sur la tête
  poussée ; elles tournent maintenant sur master, correctif en avant si rouge,
  et son script refuse désormais une suite sans verdict. Le juge des bilans n'a
  aucun plafond : ticket 89c6b9adc, politique `bilans.max_per_hour` (20 par
  défaut) à coder dès les suites rendues.
- **Retours de Lucie sur la pose de 21 h 51** (tickets 14f4e7274) : largeur fixe du
  texte des bulles, code entre accents graves et blocs à colorer et lier, mise
  en page humaine des retours d'outils (texte intégral gardé, bouton « brut »,
  rien retiré ni résumé). Coloration : B (spans typés par les grammaires
  tree-sitter de codeparsers, route POST /highlight, octets + UTF-16) avec A
  (highlight.js) en repli ; page de codeparsers acceptée ; relevé LR_CodeRag par
  tty (ca1368779). **La barre tranchée** : session › {Dépôts › worktrees (branche,
  chargé, rag), Sous-sessions (récursif)} ; badge des deux côtés ; fil
  d'Ariane ; filtre ; menu « par session / par dépôt ».
- **Essai à deux** : Lucie peut créer une tâche sur son dépôt dans l'IDE ; c7 écrit
  dans sa boîte (POST /ide/teammate/mailbox, from c7) ; `bilans.memoire` prêt chez
  G (ligne absolue vers sa mémoire, sauvegarde .avant-bilans-7-nov, relecture du
  manifeste par POST /backend/open sans couper la session), à poser à son oui ;
  sans plafond du juge des bilans tant que 89c6b9adc n'est pas codé.
- **hooks-essai** : appel-a-delai prêt (350eaabe7 : `deadline_ms` appliqué par
  stop_run, `cancelled_by: "deadline"` ; ticket « une connexion par run » à part),
  dossier de session sur master (825ba6bf1). **Cœur** : groupe 1 clos (deux tickets
  non reproduits avec témoins). **Hotfixs** : RTree reclassé chantier géo, passe à
  G2. **luciepc** : 6 places lourdes ; targets 201 → 150 Gio ; disques sains.

## Mise à jour de 22 h 50

- **lucivy 4.3.2 publiée** (PR #21, tag au mot direct de Lucie, crates.io/npm/PyPI) :
  passage exact long, flou long, apostrophes, clés inconnues nommées ; montée de
  rag3weaver en lot chez recherche (montee-lucivy-4-3-2 : trois crates, doublement
  de l'apostrophe retiré, L et K par la mesure, témoin de la ligne collée).
- **Essais de l'équipier par c7** (Lucie : « fais des essais seule, demain je
  regarderai les sessions d'exemple ») : `bilans.memoire` posé dans son manifeste
  de code (G, 22 h 10, relecture sans coupure, juge des bilans armé, surveillé) ;
  la première tâche refusée par l'hôte : « le backend n'est pas dans un dépôt git »
  (la création d'une tâche cherchait le dépôt dans le dossier du manifeste, pas
  dans workspace.root) → ticket et correctif tache-hors-depot (hooks-essai, en
  passe ; la relecture d'ED y a trouvé un second défaut, les déclarations hors
  dossier d'un manifeste code-tree) ; pose par G avec `task_writes: true` et
  l'outil `edit_file` dans son manifeste (sans eux l'équipier ne peut pas écrire).
  Trois consignes prêtes (journaux d'annulation dans /tmp, racine disparue, bases
  laissées dans /tmp par les tests). Méthode de Lucie : « regarde les retours de
  ses outils et si les params sont cohérents avec ce qu'il voulait, s'il est
  bloqué, et itère ».
- **Décisions de Lucie** : des outils intégrés de l'hôte pour attacher dépôts et
  worktrees (jamais la ligne de commande : pas de hook sinon), repli déclaratif ;
  une tâche doit s'ouvrir même sans git (espace = worktree si dépôt, dossier
  sinon, `git_init` sur demande) ; le modèle : une tâche = l'intention, 1..n
  espaces (un worktree par dépôt touché), plan unique dont chaque pas dit son
  espace, une proposition par espace, sous-tâches déléguées ; écran : session ›
  tâches › espaces ; « une session tient potentiellement plusieurs worktrees ».
- **ED** : suites manquantes de la reprise jouées, toutes vertes ; garde du script
  (pas de push sans verdict complet) ; ticket du plafond du juge des bilans
  (89c6b9adc) en cours de code. **Arbre** : lib de demain finie (7cd11e5fa…),
  passe du juge + graphe cité en troisième tour. **Codeparsers** : pointeur en
  passe avec un drapeau qui attend tache-hors-depot.

## Mise à jour de 23 h 10 — fin de soirée

- **Essais de l'équipier** : non lancés ce soir. La création de tâche est corrigée
  et posée (76ecc990e, 23 h 01), mais l'écriture de `task_writes` et `edit_file`
  dans le manifeste de Lucie a été refusée à G par sa permission (« Security
  Weaken »), non contournée : geste d'une ligne à sa main (fichier prêt dans le
  scratchpad de G). La voie de secours (`code-tree add --from code-rag3db` sur un
  worktree frère) échoue : code_tree.rs cherche `backend.json` et son manifeste
  s'appelle `rag3db.json` (ticket à ED). Worktree d'essai prêt :
  ~/git_workspaces/rag3db-essais-c7. Trois consignes prêtes. Reprise demain matin.
- **Figements de la lib entière** : trois cas ce soir (archi sous l'ancienne lib au
  portable à charge 1,2 ; l'arbre, binaire mort à 7 min ; codeparsers, 62 min, 8
  tests arrêtés), mêmes familles (taches, daemon_host, reactor, runtime, relais) :
  ni la lib 43 ni la charge ; master nu passe en 28 s ; codeparsers répète la passe
  avec un guetteur qui prend les piles (eu-stack sous ptracer) au premier figement ;
  ticket unique ; piste : un test qui attend un service réel sans délai.
- **Atterris** : tache-hors-depot (76ecc990e), reprise (4ae8c93a8, suites jouées
  après coup, vertes), G2a de hotfixs (9ec767dd4 : COPY refusé non reproduit) ;
  G2b (blobs) bloqué par un SIGSEGV du COPY de blobs annulé, enquête. Lib de
  demain finie (7cd11e5fa…), fenêtre G 8 h 30 avec les granite ; `bilans.memoire`
  posé chez Lucie (juge armé, surveillé, plafond en file) ; lucivy 4.3.2 publiée,
  montée verte chez recherche (L = 900, K = 64 remesurés) ; fausse alerte du
  plein texte de la mémoire (copie prise sous un écrivain : règle de copie).
- **Décisions de Lucie** : une tâche = une intention à 1..n espaces ; barre
  session › tâches › espaces + vue par dépôt ; outils intégrés pour les dépôts ;
  tâche sans git ; géo plus tard ; « ne pas s'empêcher de progresser » ; luciepc
  à 6 places ; une seule publication lucivy.
- **File restante** : codeparsers (pointeur) → ED (plafond du juge) → 96
  (run_e2e.sh) → arbre (juge + graphe cité, vert par tickets) → 96 (pas 3) →
  embarquement (spans) → recherche (second-ecrivain, spans, montée 4.3.2) → archi
  (pont) → hooks-essai (appel-a-delai) ; hooks-essai : la barre, le chat (largeur,
  retours d'outils, code A), la tâche sans git ; codeparsers : coloration B.

## Mise à jour de 23 h 45

- **Atterris** : pointeur codeparsers 672a0d5 (f3a115887 : la garde de match compte
  comme usage, ticket « marque nom » ouvert), plafond du juge des bilans
  (ff2725f90, `bilans.max_per_hour` 20), `code-tree add --from` par nom réel
  ou chemin (0683cc474), chat-et-delai de hooks-essai (682655352 : largeur des
  bulles, code en ligne et coloration A, retours d'outils mis en page avec
  « brut », barre session › tâches › espaces avec la vue par dépôt, règle d'état
  unique de l'équipier, `/call` à `deadline_ms` par stop_run), plafond de target
  32 Go mesuré (G, d845744a2), ticket unique des figements de la lib
  (a6beebc60 + af035a99f), page de la coloration (codeparsers, 73d4bfdf6 :
  tree-sitter-highlight officiel, ancien module remplacé).
- **Pose du soir** : G pose 682655352 chez Lucie (coupure d'une minute), puis c7
  lance les essais de l'équipier sur ~/git_workspaces/rag3db-essais-c7 inscrit
  par `code-tree add --from code-rag3db` (edit_file ajouté au manifeste neuf) ;
  Lucie garde la main sur task_writes/edit_file de son propre manifeste.
- **Reste de la nuit** : 96 (run_e2e.sh, puis le pas 3 après l'arbre) → arbre
  (juge + graphe cité, vert par tickets) → embarquement (spans) → recherche
  (second-ecrivain, spans, montée lucivy 4.3.2 : L = 900, K = 64) → archi (pont,
  canal). Demain : banc 7 h 30 (mesure), G 8 h 30 (lib 7cd11e5fa… + granite),
  le réacteur sous charge à ED, la coloration B (codeparsers + hooks-essai +
  tty), la tâche sans git, les liens de symbole, M8 des gabarits de mémoire.

## 00 h 45, le 8 — la file de la nuit avance, les essais de l'équipier butent sur Vertex

- **Atterris depuis 23 h 45** : arbre, le juge d'un tour de Claude Code et le graphe cité
  (994a58e39..76f19526b, cinq commits ; la substitution n'entre plus dans le graphe cité,
  2fe3c114f ferme le ticket de 96 ; rejeu court : lib entière sans EMBED_SERVICE 1998 passés,
  e2e_juge 5/5 avec les deux juges Gemini) ; hooks-essai, barre-noms (830e82256, cfbe928a4 :
  une session se nomme par son dépôt, « la mémoire » pour une mémoire ; première passe VERT puis
  arrêt du push devant du code arrivé, fusion et rejeu complet, worktree et target effacés) ;
  embarquement, les spans du creux (59f02b636, 9231f07dc, 1b629d749 ; lib entière 1895/0,
  e2e_service_spans vert avec les services ; les branches spans-creux à -5 restent, listées).
- **En cours** : ED pousse equipier-jamais-muet-2 (9914ab946) sur 1b629d749, rejeu complet ;
  puis recherche enchaîne ses trois lots (second-ecrivain → creux-par-les-spans-2 a0f31f8e1 →
  montee-lucivy-4-3-2) sans me redemander entre deux ; puis archi (hooks-pont-7, le canal).
- **ED prend le lot 2 de la page 01 avant le réacteur** (les commandes par la porte déclarée,
  branche commandes-declarees 97e5a68cf) : sans `run_command`, l'équipier ne joue aucun test,
  donc aucune vraie tâche de code pour les essais de Lucie demain. Le réacteur sous charge
  attend ; **arbre tient la bissection du figement** à luciepc depuis 0 h 29 (cadre : lib
  17a094d12 + MOTEUR_ANCIEN=1, jeu large AVEC les services 7878-7880, un figement = 10 min sans
  ligne, pile par eu-stack sur son pid ; phases : les trois tests de la mémoire seuls ×10, la lib
  entière ×4, sans les trois ×4, puis chacun seul ×3 ; journal à
  `~/.cache/rag3weaver-notes-A/bissection/journal` ; arrêt net à 7 h 15 ; ticket à jour à la fin,
  pas de correctif sans mon mot).
- **Essais de l'équipier** : page `docs/08-novembre-2026-orchestration/01-essais-de-l-equipier.md`.
  L'essai 1 muet (pas d'outil d'écriture, pas de `step_done`) a donné le lot d'ED ; l'essai 2 a
  fait son plan puis est mort du fournisseur : 503 attendu 600 s sans reprise, puis deux 429
  RESOURCE_EXHAUSTED, trois vies de `max_failed_steps` brûlées sans un pas. Les 429 ne viennent
  pas de la passe d'ED (pas encore à ses suites Gemini) ni des crochets de Lucie (aucun 429 dans
  hooks.log) ; Vertex global, gemini-3.8-flash. Je relance après le verdict d'ED, pour ne pas
  tirer sur le quota pendant e2e_juge et e2e_equipier. Pour ED, après le lot 2 : un 429/503 ne
  consomme pas une vie, la reprise tient la fenêtre d'un quota par minute.
- **L'hôte de Lucie ne prend un lot qu'à une pose de G** : jamais-muet et le lot 2 n'y seront
  qu'à 8 h 30, sauf pose intermédiaire que je demanderai à G si la nuit le permet.

## 02 h 05 — recherche atterrie, le premier lot écrit par l'équipier en passe, deux décisions de conception

- **Atterris** : ED, lot express du compte des suites de `run_e2e.sh` (91543d7f0 : `--test e2e <suite>`
  rougissait toute passe depuis 5eb6fb277, « Suites jouées : 0 sur 1 ») puis equipier-jamais-muet
  (aff3611fb, passe complète : lib 2003, navigateur 19, chat 15, e2e_lectures 22, e2e_tty 2,
  e2e_juge 5/5, e2e_equipier 1/1) ; hotfixs, **G2b** (dadbf0041, C++ du moteur : COPY de nœuds
  borné en octets, journal par tranches, BlobCopyCheckpointTest, 13 suites, RollbackUnderReaders
  ×20 ; G2 est fini) et check_tickets --lot qui lit un ticket fermé réécrit comme « fermé »
  (306907718) ; recherche, ses trois lots (second-ecrivain 80ca3d561, creux-par-les-spans-2
  a4a8535e1, montée lucivy 4.3.2 ccec05572 ; le dernier poussé sans rejeu après l'arrivée de
  306907718 et du C++ seuls, dit tel quel). **La lib de demain se rebâtit à 8 h sur dadbf0041**
  (G) ; hotfixs ne pousse plus de C++ avant le « basculé ».
- **En cours** : archi, hooks-pont-9 sur ccec05572 (deux conflits de teammate.rs fusionnés,
  passe complète hors verrou) ; puis **hotfixs pousse le lot de l'équipier**
  (agent/journaux-annulation-tmp-3-3 = bddbdc0f6, relu sans faute : seule une fin réussie efface,
  seul le retour arrière d'une migration relit une annulation, `lire_texte(...).ok()` lit un
  `@file:` effacé sans planter ; passe à luciepc ; un push parti dans la commande d'un rebase en
  conflit a posé la tête d'origine sur -2, d'où -3, sans force) ; ED, lot 2 des commandes par la
  porte (9ceec57f1 : première passe rouge d'un **SIGSEGV à la sortie du binaire e2e après
  e2e_equipier vert**, rejoué seul vert, nommé, ticket s'il revient ; passe relancée).
- **Le figement des passes est tenu** (arbre) : au portable, master c1bdbb183 vert 3/3, d845744a2
  figé 3/3 ; la sonde des tunnels n'a jamais été muette ; la pile désigne l'hôte de test (un ureq
  sans délai dans une_tache_ouverte_survit_a_l_arret_de_l_hote_et_se_fusionne), pas le service.
  Bissection des 23 commits entre les deux pour nommer le correctif ; ticket à la fin.
- **Décision G3 (hotfixs, avis du cœur)** : la base rouverte juste après un point de reprise
  (journal vide) ne chargeait pas l'extension vector notée à côté (1/6 au banc sous charge). Je
  tranche la voie du moteur : **toute ouverture relit les extensions notées** (PostgreSQL charge
  au démarrage, pas au rejeu) ; un échec **ne refuse pas l'ouverture** (la garde 1 et ses témoins
  tiennent : index « en retard », rebâti), il s'écrit au journal du moteur et se lit par une
  colonne `extension_error` de SHOW_INDEXES. Témoin déterministe écrit (4ec960e99). C++ après le
  basculé ; **mot de Lucie avant le push**, puisque le comportement visible change ; le refus
  avec option reste une proposition de ticket pour elle.
- **Essais de l'équipier** : l'essai 3 a abouti (page 08/01) : témoin rouge puis vert joué par
  moi à luciepc, correctif au plus étroit après mon recadrage, commit b46bddcac. Constats pour
  le ticket des essais d'ED : pas de `run_command` (lot 2) ; les 429 Vertex ne frappent que
  l'équipier (même sur un pas au fil court : l'hypothèse du fil entier faiblit, ED mesure) ; deux
  crochets d'après-outil d'`edit_file` en erreur (gabarit `motif-ailleurs` relatif au répertoire
  courant, appel sans `old`) ; le **juge note chaque pas comme la tâche entière** (2/10 au pas
  « lire ») et dit faux sur les outils déclarés ; l'équipier a dit « constate l'échec » sans
  jouer, et a clos le plan malgré « attends ma confirmation ». Essai 4 (racine-disparue) en
  cours : il a posé ses trois questions d'exécution avant d'avoir écrit une ligne, recadré.

## 03 h 00 — archi atterri, le figement expliqué, l'essai 4 au vert

- **Atterri** : archi, hooks-pont-9 (46633adcc : `mail send|reply|pending|seen`, fils Codex et
  livraison par `codex queue`, boîte dans le hook, canal, règle de la boîte, témoin par le chemin
  de l'hôte ; passe VERT hors verrou, 40 s sous poste push). **Pour Lucie** : rien ne change tant
  qu'elle ne fait rien ; le canal demande son `.mcp.json` et la commande de la page 07 § 4 bis, les
  hooks Codex sa confiance par `/hooks` dans Codex ; G le souligne : un message adressé à `codex:`
  met un autre agent en marche.
- **La file** : hotfixs pousse le lot de l'équipier (agent/journaux-annulation-tmp-3-3, passe à
  luciepc sous les services 7879 pour le témoin du creux, lib sans service) ; puis ED, lot 2 des
  commandes par la porte (commandes-declarees-5, 45654ed69 : une course corrigée en route — un oui
  posté pendant le dernier pas d'un lancement se perdait ; un rouge d'e2e_juge départagé comme
  variance de classement de Gemini, vert 2/2 sur master et 3/3 sur la tête) ; puis archi, 3 ter (d)
  (le client du hook passe `deadline_ms` à 95 % de son délai, hooks-3-ter-d) ; puis arbre, le lot
  du délai.
- **Le figement est expliqué** (arbre) : aucun commit correcteur, le figement suivait l'heure
  (0 h 45 → 1 h 46, la charge de luciepc sur les services bge-m3 derrière les tunnels) ; reproduit
  à volonté avec un faux service muet (127.0.0.1:7999) : même figure, 1 226 fils en futex_wait, les
  tests bloqués dans `Backend::stop_reactions` → `stop_within` → `rt::bloquer`. L'embarqueur a un
  délai de 60 s (`RAG3WEAVER_EMBED_TIMEOUT_SECS`), le relecteur (/rerank) et l'OCR (/ocr) n'en ont
  aucun. Lot d'arbre : un délai commun aux trois clients (défaut 60 s gardé, mesure du p99 sous
  charge demain dans la fenêtre de G), erreur nommée « service <adresse> muet après N s », témoin
  par le faux service muet, et un témoin que `stop_within` revient dans sa borne ; ticket réécrit
  d'abord ; passe et push à mon « à toi ». La règle « lib entière sans EMBED_SERVICE » tient
  jusqu'à son atterrissage.
- **La lib de demain** (G) : rag3db-build-dadbf0041 (tête 7bf887a01, md5 253c898f…) publiée aux
  deux postes, verdict vert (e2e_code 27/27, e2e_lectures 23/23, format 43), ABI changée (vtables
  Table, RelTable, NodeTable), contrôle des deux sens sans écart ; fenêtre 8 h 30 tenue ; courant
  reste 17a094d12. G pose le binaire seul au portable au hash du lot 2 d'ED.
- **Essai 4 de l'équipier (racine-disparue) au vert** : témoin rouge sur la bonne assertion après
  trois sorties rendues, correctif (`racine_absente` dans `state()`, erreur nommée dans
  `call_tool_on`), 1 passed, et les modules daemon_host + backend 66 passés, 0 rouge. Recadré sur
  une liste de noms d'outils en dur (la politique déclarée suffit). Constats produit dans la page
  08/01 : l'attente du superviseur compte comme pas raté et tue la tâche, les `answer` ne sont pas
  lues, la répétition en attente, le plafond de 40 pas atteint par les lignes d'attente.

## 03 h 50 — le lot 2 et le premier lot de l'équipier sur master, l'hôte de Lucie reposé

- **Atterris** : ED, les commandes par la porte déclarée (ad496c462 : dans une tâche dont le workspace
  a une porte, `run_command` passe par elle ; deux courses de la boîte corrigées en route — un oui
  posté pendant le dernier pas se perdait, puis la fin d'un lancement marquait toute la boîte lue ;
  passe complète VERT : lib 2026, e2e_juge 5/5, e2e_equipier 1/1) et le ticket des essais réels
  (eb154bf5f, `docs/tickets/2026-11-08-l-equipier-sur-gemini-ce-que-les-essais-reels-ont-montre.md`,
  l'état d'attente en premier lot) ; **hotfixs, le premier lot écrit par l'équipier** (b860ea2dd,
  `nettoyer_apres_fin` ; passe : check_features 5/5, lib 2027, e2e_lectures 24/24 sous 7879,
  e2e_tx 3+1+18+3). Son script résout seul un conflit limité à nouveautes.md, s'arrête sur tout
  autre ; une heure perdue sur un guet qui attendait G3 (dit tel quel).
- **G a posé le binaire seul au portable à 3 h 43** (tête eb154bf5f contre 17a094d12, binaire
  200773e7, essai vert sur 7991) : l'hôte de Lucie porte jamais-muet et les commandes par la
  porte ; luciepc garde celui de 0 h 04 ; les lots d'après vont à 8 h 30.
- **La file** : archi (3 ter d) à toi à 3 h 44 → arbre (délai partout, 67fadca5a) → hotfixs
  (racine-disparue + b51a10994 : `read_files` sur `index` et `estimate` dans les deux gabarits,
  3e assertion, le 3e symptôme renvoyé aux tickets existants, un ticket neuf pour le 4e) → ED
  (l'état d'attente, puis 429/503 hors de max_failed_steps). G3 : prémisse d'un test de la garde 1
  ajustée (la liste retirée avant la réouverture), passe complète après le basculé, mot de Lucie.
- **Essai 5 de l'équipier lancé à 3 h 44** sur l'hôte reposé (`tests-bases-dans-tmp`, avec
  `run_command` par la porte `approval` : les commandes hors liste sûre me sont soumises) ; la
  réponse du POST porte désormais `task: {branche, depart}`.

## 04 h 15 — le délai partout sur master, la règle « sans service » levée

- **Atterris** : archi, 3 ter (d) (624aaadc9, `deadline_ms` à 95 % du délai du client sur chaque
  /call du hook) ; hooks-essai, deux tickets du rappel (4181b96b4 : les bilans du juge remontent sur
  des mots courants ; une fiche déjà vue redevient neuve après la relance de l'hôte — `DefaultHasher`
  ou résumé réécrit, blake3 comme forme) ; **arbre, un délai partout** (d63c16649..d8aeffea7 :
  relecteur et OCR au délai de l'embarqueur, `stop_within` compté hors du runtime tokio à deux fils,
  e2e_arret_borne seule dans son processus ; la preuve : la lib entière sous un faux service muet
  finit en 28 s au lieu de figer ; la procédure dit maintenant « la lib entière se joue avec ou sans
  RAG3WEAVER_EMBED_SERVICE », le ticket du figement est fermé, deux tickets ouverts : la valeur
  mesurée du délai, le client ureq sans délai du test de l'hôte ; un ticket pour une course du
  témoin des commandes d'ED, que son lot de l'attente corrige déjà). Arbre efface ses worktrees et
  s'arrête ; wt-lib-arbre reste pour le rebâti de 8 h.
- **La file** : hotfixs pousse racine-disparue (22342c504, forme (b) : le message commun dit quoi
  faire, `read_files` sur `index` et `estimate`, le ticket fermé corrigé — IndexNode avait déjà sa
  garde) ; puis ED, l'état d'attente (equipier-attente-2, ecb5945ba : une question ou une approval
  sans réponse suspend l'équipier, aucun appel au modèle, ni réussi ni raté ni compté ; **la mesure
  du quota** : 5 275 jetons au pas 1, 122 242 au pas 10, 950 000 en trois minutes au pas 5 — le
  fil entier renvoyé à chaque tour) ; puis le bac à sable (equipier-bac-a-sable 49a5ca610 :
  `sandbox.extra_read` de la chaîne Rust, `sandbox.env` avec {task_dir} et {repo_target}, target
  partagé par dépôt sous verrou, 32 Go, `taches purge`, TMPDIR hors de /tmp, /dev/null
  inscriptible) ; puis le fil par pas, puis la reprise sur 429.
- **Essai 5** (`tests-bases-dans-tmp`, hôte reposé) : `run_command` par la porte marche (`id`,
  `cargo --version` accordés par `answer {approved:true}`), mais **cargo ne lit pas ~/.rustup dans
  le bac à sable** (Landlock : /usr, /lib, /etc et le workspace seuls) — d'où le lot du bac à sable.
  L'équipier a d'abord écrit son témoin dans `src/` (code produit, non monté), recadré vers
  `tests/common` ; il a ensuite posé un vrai témoin sur `deux_processus_partagent_la_base_par_le_demon`,
  **ROUGE sur le défaut réel** joué par moi : « la suite a laissé 3 fichier(s) dans /tmp » (la base,
  .wal, .ecrivain). Vingt lignes « en attente » en dix minutes entre-temps : l'attente d'ED n'est
  pas encore dans l'hôte.

## 05 h 00 — la file de la nuit est vide, trois lots d'ED sur master, l'essai 5 au vert

- **Atterris** : hotfixs, racine-disparue (b0a9ab6e5, 1b9983b59, 1de9c908c ; passe sur d8aeffea7 :
  check_features 5/5, lib 2030, e2e_lectures 24/24, e2e_tx ; sa garde du dernier pas prenait
  `extension/rag3weaver/docs` pour du code, corrigée ; docs 4f813c3ee) ; ED, **l'état d'attente**
  (b0301a0e3 : une question ou une approval sans réponse suspend l'équipier, aucun appel au modèle,
  ni réussi ni raté ni compté, survit à un redémarrage ; la course de son témoin des commandes
  corrigée avec) et **le bac à sable** (927ae9680 : `sandbox.extra_read` de la chaîne Rust,
  `sandbox.env` avec {task_dir} et {repo_target}, target partagé par dépôt sous verrou qui dit son
  attente à 60 s, 32 Go, `taches purge`, TMPDIR={task_dir}/tmp, CARGO_NET_OFFLINE, /dev/null
  inscriptible ; témoins : cargo compile, lie et teste sous Landlock, l'écriture dehors refusée,
  et une tâche joue `cargo test --lib -- un_test` par la porte et lit « test result: ok. 1 passed »).
  Les arbres déjà inscrits gardent leur manifeste : rejouer `code-tree add` ou copier le bloc
  `sandbox`.
- **Rien d'autre ne pousse avant la fenêtre** : ED tient prêts le fil par pas (9fef96de9 : un pas
  repart de la consigne, du plan, de la boîte depuis le dernier pas et du résumé des step_done) et
  le correctif de **l'arrêt d'inactivité qui ne comptait pas une tâche d'équipier** (767dcb036 :
  à 4 h 46 l'hôte de Lucie s'est arrêté « inactif depuis 1804 s sans travail en cours » pendant
  que l'équipier de l'essai 5 éditait ; systemd l'a relancé par la socket, la tâche a repris à sa
  frontière, le pas en cours perdu) ; « à toi » après le « basculé » de G.
- **hotfixs, G5** : HyperLogLog 64 → 1024 registres attend la bascule 44 (témoin en known_red) ;
  analyze au seuil : témoin, forme du ticket, mesure du coût sur une copie de code-rag3db dans ma
  fenêtre de 9 h sous `poste mesure` ; copy-de-relations-annulé d'abord. G3 attend le basculé et le
  mot de Lucie. Arbre a effacé ses worktrees (10 Go rendus), garde wt-lib-arbre pour le rebâti de
  8 h, et dort.
- **Essai 5** : témoin ROUGE sur le défaut réel, garde `BaseTemporaire` (Drop, `RAG3WEAVER_GARDER`
  pour garder) dans tests/common, cinq témoins corrigés dans e2e_rag3daemon, e2e_code et
  e2e_prise_atomique, **tous verts** joués par moi à luciepc ; deux journaux à nom fixe
  (`/tmp/rag3weaver-e2e-acces`, `-e2e-rag3daemon`) dits hors du lot. Il écrit le ticket.

## 05 h 20 — fin des essais de la nuit, ce qui attend Lucie

- **Essai 5 clos par `stop`** à 5 h 14 : le code vérifié (cinq verts) commité sur
  `agent/tests-bases-dans-tmp` (1b4c73228 + 674b41f8d), le ticket fermé à hotfixs avec la passe,
  après la fenêtre. Trois lots réels sont sortis des essais ; la page 08/01 dit tout, et « ce qu'il
  faut en retenir » en tête de la section finale.
- **Le matin** : 7 h 15 arbre arrêté (déjà) ; **7 h 30 banc** sous « fenêtre » (`poste mesure`, la
  place rendue avant 8 h 15) ; **8 h** arbre rebâtit la lib si un commit moteur est arrivé après
  dadbf0041 (aucun à 5 h 20) ; **8 h 30 G** : lib rag3db-build-dadbf0041, hôtes rebâtis sur le
  master du moment, granite, courant, réveil ; puis « à toi » à ED (fil par pas + arrêt
  d'inactivité, deux lots prêts), puis hotfixs (troisième lot de l'équipier), puis ma fenêtre de
  9 h pour hotfixs (analyze au seuil, les mesures de A) sous `poste mesure`.
- **Décisions qui attendent Lucie** (rien n'est poussé sans son mot) :
  1. **G3** : l'ouverture d'une base relit toujours ses extensions notées (un échec se dit par
     `extension_error` de SHOW_INDEXES, l'ouverture n'est pas refusée) — comportement visible qui
     change, témoin et code prêts chez hotfixs (g3-extensions-a-l-ouverture), passe après le basculé.
  2. **Le pont Codex** (46633adcc) : son `.mcp.json` et la commande de la page 07 § 4 bis pour le
     canal, `/hooks` dans Codex pour la confiance ; un message adressé à `codex:` met un agent en marche.
  3. **Son manifeste de code** : `task_writes` + `edit_file` restent à poser de sa main
     (`cp -p $M $M.avant-taches-7-nov && cp <fichier> $M`, fichier prêt dans mon dossier de travail
     de la session d'hier) ; et depuis 927ae9680 le bloc `sandbox` du gabarit (`code-tree add` à
     rejouer, ou copie à la main) pour que cargo tourne dans ses tâches.
  4. **Thème daemon.json** (tty), **page 03 de l'atelier** (ED), les **633 branches distantes**
     restantes (listes par session), la **facture Vertex** (31 jugements réels sur la nuit, 429 à
     répétition sur l'équipier : le fil entier, corrigé par le fil par pas).
  5. Les essais eux-mêmes : lire la page 08/01, puis « les mettre sur un vrai taff » — un ticket
     qu'elle choisit, lancé par `POST /ide/teammate/mailbox` sur son propre arbre une fois le
     manifeste complété, ou sur `code-essais-c7` tel quel.
- **Ménage à faire à son mot** : `~/git_workspaces/rag3db-essais-c7`, l'arbre
  `~/.config/rag3weaver/code-essais-c7`, les cinq tâches sous `~/.local/share/rag3weaver/taches/e4d8635ecfe7/`
  et la base `code-essais-c7.rag3db` ; à luciepc `wt-c7-essais` et `target-c7-essais` (je les efface
  moi-même ce matin) ; les branches `agent/*` sur origin restent.

## 08 h 40 — la fenêtre du banc, la bascule de G, la file du matin

- **Banc, 7 h 26 → 7 h 32** sous `poste mesure` à mon mot « fenêtre » : les lignes hors du graphe
  d'un index vectoriel coûtent 0,75 à 1 µs par ligne et par recherche (120 000 vecteurs de
  dimension 768, k = 10 : 13,4 ms à 0 ligne hors graphe, 23 ms à 10 000, 88,5 ms à 100 000 ; binaire
  statique de master, aucune lib du magasin à épingler). Ticket du 18 oct. et page du banc
  (28b479b7e, 1e7084927). **Recherche en a tiré le seuil** (365ec8f02) : « la recherche ne doit
  pas doubler », reconstruire au repos de la montre quand N × 1 µs ≥ t₀ (N* ≈ 13 400 lignes à 768
  pour t₀ = 13,4 ms ; la reconstruction de 143 s ne se rembourse qu'en 10 700 recherches, c'est donc
  la latence qui déclenche). **hotfixs a ouvert le ticket moteur** (5d04e9794) : `rows_outside_graph`
  dans SHOW_INDEXES ou un CALL en lecture, sous l'instantané de l'appelant, après G3 et G5 ; le lot
  produit (le déclencheur et son témoin à N* ± 1) sera à recherche. Le banc a effacé rag3db-vingt et
  build-vingt à luciepc (contrôles faits) et se repose.
- **G a basculé à 8 h 31** : lib rag3db-build-dadbf0041 (md5 253c898f) et binaire b64748ef
  (1e7084927) aux deux postes, coupure de 1 min 16 s, granite-278m et -107m redéployés par
  embarquement avec un écart nul, courant = dadbf0041, les quatre bases rouvertes, l'IDE vert ; la
  recherche dense bornée surligne `source: "sparse"` chez Lucie. Disques : portable 357 Gio libres
  (build 38 Gio), luciepc 608 Gio (build 115 Gio). Les instantanés bascule-*, locks-25-oct,
  avant-43-controle et migration-fiche-25-oct ont été effacés **par moi hier soir au mot de Lucie**
  (G l'a vu ce matin) ; avant-43 reste le seul retour. Depuis le basculé : MOTEUR_ANCIEN vide, la
  lib neuve épinglée par son identifiant complet dans toute passe.
- **La file du matin** : ED pousse le fil par pas + l'arrêt retenu par un équipier (7d38dc243, à
  toi à 8 h 35) → G pose le binaire seul aux deux hôtes dix minutes après son hash → **9 h, ma
  fenêtre** pour la mesure G5 de hotfixs (CHECKPOINT après un DELETE de 11 % sur une copie reflink
  de code-lourd à luciepc, deux libs prouvées par md5, trois prises chacune) → hotfixs, le
  troisième lot de l'équipier (dix témoins sous la garde, ticket « en cours ») → recherche, le
  retrait du contournement du dense filtré sous l'instantané → archi, le (e) des hooks (le
  vocabulaire n'apprend plus les mots d'arrêt ni les noms de fiches de bilan, résumé rendu borné
  à 200 caractères ; vu au premier tour de hooks-essai sur le binaire posé, 180 ms, un bloc rendu)
  → hotfixs, G3 (au mot de Lucie) puis G5 sur le moteur du worktree prouvé par md5.
- Un écart vu par G : la barre de hooks-essai dit « memory » et non « la mémoire » (cfbe928a4) ;
  G le lui dit.

## 09 h 05 — la pose d'ED, un SIGSEGV de l'hôte du portable, la fenêtre de hotfixs qui attend

- **G a posé à 8 h 45** le binaire d'ED aux deux hôtes (1b9796d9f : fil par pas, arrêt
  d'inactivité retenu par un équipier, la demande en cours ; binaire fe994691, lib dadbf0041), IDE
  vert, code-lourd fermé puis rouvert pour les six copies reflink de hotfixs (« copié » 8 h 47).
  ED a poussé son rapport de la nuit (424b9ebb4).
- **L'hôte du portable a fait un SIGSEGV à 8 h 50 m 11 s** (SEGV_MAPERR, fil demon-5) sous ce
  binaire et la lib dadbf0041 : rip = librag3db.so + 0xd12c73 =
  `rag3db::storage::BufferManager::optimisticRead`. Au même instant : la montre réindexait
  code-rag3db-depot et mon arbre d'essais remis sur master (backend.rs, 4,7 s de verrou), deux fils
  lucivy fusionnaient des segments, l'équipier de l'essai 6 lisait. systemd l'a relancé par la
  socket à 8 h 50 m 57 s ; la tâche a repris à sa frontière. Le core (32 Gio une fois extrait,
  tronqué, seule l'adresse du fil fautif lisible) est gardé à
  `~/.cache/rag3weaver-build/core-c7/core.39731`. L'hôte de luciepc n'a pas planté (G) ; aucun
  autre plantage de la nuit sur aucun binaire. **Le cœur a le ticket** (moteur : un lecteur
  optimiste et une page évincée ou réécrite pendant une indexation) ; rien ne départage encore la
  lib dadbf0041 (G2b, projection, retrait) d'une course ancienne qu'une charge révèle ; G garde les
  binaires d'avant pour rejouer la charge sur un hôte d'essai si le cœur le veut. Pas de retour de
  courant, pas de pose sans mon mot.
- **Le cœur** : le groupe durabilité mord — avec une faute d'E/S injectée (disque plein pendant la
  synchronisation du journal), une validation échouée **laissait sa clé prise** dans l'index de clé
  primaire (publication avant l'écriture du journal) ; correctif sur la branche coupure : journal
  écrit et synchronisé avant la publication, taille rendue sur faute, plus rien ne se valide après
  un journal irréparable ; six témoins de coupure verts, transaction 350/350. Push en lot groupé à
  une heure qu'il me dira ; il remettra MOTEUR_ANCIEN=1 aux passes jusqu'au rebâti.
- **La fenêtre de hotfixs attend** depuis 8 h 47 : un `transaction_test` de rag3db-colonnes (le
  cœur) tient la place de lourd à luciepc depuis 8 h 46 ; personne ne force. Derrière : hotfixs
  (troisième lot de l'équipier), hooks-essai (memoire-nommee : la nature avant le dépôt),
  recherche (dense filtré sous l'instantané, témoin rouge écrit), archi (3 ter (e), VERT au
  portable ; trois rejeux d'un test seul hors `poste lourd`, dits et promis non reproduits), 96 (M4
  pas 3, rejeu à refaire sous dadbf0041).
- **Essai 6** lancé à 8 h 48 sur l'hôte posé (ticket de hotfixs : l'undo_json qui pointe un fichier
  effacé ; cargo par la porte, `sandbox` posé dans mon arbre avec `CARGO_BUILD_JOBS=8` — à ajouter
  au gabarit, dit à ED) : la mesure « après » du fil par pas. Première ligne de l'équipier : une
  question de commit à vide, puis le plan en six items, puis la relance de l'hôte ; il lit.

## 09 h 15 — second plantage, repli de l'hôte, la base de code de Lucie à rendre

- **Second SIGSEGV de l'hôte du portable à 9 h 05 m 49 s** (pid 102525, core de 65 Mo tronqué),
  toujours sous dadbf0041 : deux plantages en quinze minutes, aucun en dix heures sous 17a094d12.
  **G a appliqué la règle convenue et reposé l'hôte d'avant à 9 h 06 m 25 s** (binaire eb154bf5f,
  lib 17a094d12 ; `courant` reste dadbf0041 pour les passes). Le cœur a lu les registres du premier
  core : la boucle de `ConcurrentVector::operator[]` dans `optimisticRead` suit un numéro de page
  de 330 Gio — un lecteur qui porte un numéro faux, lu dans un segment libéré ; ticket 60e78e6e5.
- **La base de code de Lucie (code-rag3db-depot) refuse de se rouvrir sous 17a094d12** : son .wal
  de 5,8 Mo, écrit à 9 h 03 par l'hôte dadbf0041, porte l'update d'une relation CONSUMED_BY
  (187668, nœud 10769) « not found in the forward direction… it would have reached one direction
  only ». Famille G2b / deux sens. Décision prise à sa place (elle dort ; sa règle du 7 : « tant
  qu'on n'est pas stable, mes données de test je m'en fiche ») : (a) une copie reflink ouverte
  sous dadbf0041 sur un hôte d'essai pour voir si la lib neuve rejoue ; (b) **l'original — base,
  .wal, .ecrivain — mis de côté tel quel** dans
  `~/.local/share/rag3weaver/preuves/2026-11-08-code-rag3db-depot-wal-dadbf0041/`, la pièce du
  défaut pour le cœur et hotfixs ; (c) **retour par avant-43** (l'état du 7 nov. 19 h 21) et la
  montre qui rattrape les fichiers changés depuis — search_code revient. memoire-longue se rouvre,
  elle ne bouge pas. G exécute et donne l'heure.
- **hotfixs** : mesure G5 rendue à 9 h 08 (analyze au seuil ≈ 390 ms une fois, au point de reprise
  qui passe le seuil, sur 116 000 lignes à FLOAT[768] : 838 → 1 232 ms, md5 des deux libs au
  ticket) ; **le témoin du mélange G2b** est écrit (g2b-melange-hnsw f37d4e9ef : COPY de 30 000
  vecteurs FLOAT[768] sous 256 Mio et 4 fils, plusieurs vidages ONTO_HEAD sous un lecteur), en
  route à luciepc ; sa lecture : la copie-échange ONTO_HEAD tourne désormais sous des lecteurs, ce
  qui n'arrivait pas avant G2b. Priorité absolue devant son troisième lot de l'équipier. Le cœur
  pousse coupure vers 9 h 55 et passe ensuite à l'ASAN sur V2 et la compaction CSR ; les deux
  correctifs iront dans un même rebâti de la lib, dans l'heure qui suivra, avec G.
- **La file reprend à luciepc** (six places) : hooks-essai pousse memoire-nommee ; recherche,
  archi et 96 jouent leurs passes sous dadbf0041 et poussent à mon mot dans cet ordre.
- **Essai 6 arrêté net** par les deux plantages : la tâche a repris deux fois à sa frontière, puis
  le backend de mon arbre n'a plus pu s'ouvrir sous l'ancien binaire (« unknown field `env` » : le
  bloc `sandbox.env` du bac à sable n'existe que dans le binaire reposé par-dessus) ; retiré de
  mon arbre, à remettre à la prochaine pose. La mesure « après » du fil par pas attend une pose
  stable.

## 09 h 20 — la base de code de Lucie est rendue, la pièce du défaut mise de côté

- **G a fait les trois gestes** : (a) une copie reflink ouverte sous dadbf0041 sur un hôte d'essai
  (7991) **refuse le même .wal** : « Relationship 187668 of table CONSUMED_BY was not found in the
  forward direction… it would have reached one direction only » — les deux libs refusent, donc
  **c'est l'écriture sous dadbf0041 qui a posé une relation dans un seul sens**, pas le rejeu qui la
  casse ; (b) l'original (base de 1,84 Go du 8 h 45, .wal de 5,8 Mo du 9 h 03, .ecrivain du pid
  planté, .fts, .extensions, .readers, .checkpoints) est mis de côté tel quel à 9 h 09 m 56 s dans
  `~/.local/share/rag3weaver/preuves/2026-11-08-code-rag3db-depot-wal-dadbf0041/` ; hotfixs a sa
  copie dans `~/.cache/rag3db-locks/depot-9h05/`, G la sienne dans
  `~/.cache/rag3weaver-build/essai-wal/` pour l'ASAN du cœur ; (c) **code-rag3db-depot est revenue à
  avant-43** (l'état du 7 nov. 19 h 21, 1,48 Go, sans .wal), réveil à 9 h 10 m 09 s sur l'hôte
  17a094d12, la montre rattrape 328 fichiers (171 à 9 h 13), l'IDE est vert (18,5 s), search_code
  répond par balayage le temps du rattrapage puis par l'index. memoire-longue n'a pas bougé.
- **Pour Lucie** : sa base de code est vivante ; ce qui a été indexé entre le 7 au soir et ce matin
  se refait par la montre ; la pièce du défaut est au chemin ci-dessus. Le cœur départage au commit
  (63a2d07f8 « COPY journalisé par tranches » et c4f8610b9 « nœuds locaux d'abord » en tête, puis
  G2b et le retrait pour les SIGSEGV) en rejouant ce .wal réel ; hotfixs a le témoin du mélange.
  Aucune pose d'hôte avant le correctif du moteur et son rebâti.

## 09 h 30 — deux défauts du moteur nommés par le témoin de hotfixs

- **A, le refus du .wal est antérieur à G1 et à G2b** : le témoin `RelCopyThenUpdateReplay` rougit
  dans ses deux cas, même sans nœud local. Un COPY de relations journalisé puis une mise à jour de
  ses relations dans la même transaction ne se rejoue pas : au rejeu, la TABLE_INSERTION insère dans
  le stockage **local** de la transaction de reprise, puis REL_UPDATE cherche le décalage définitif
  dans les listes et la garde du sens unique refuse. dadbf0041 n'a fait que rendre la suite
  fréquente dans une réindexation. La base de 8 h 45 était saine ; c'est le rejeu d'un journal
  correct qui échouait. **Correctif** (hotfixs, tout de suite) : au rejeu, une correspondance du
  décalage journalisé vers le décalage local, par table, le temps de la transaction rejouée ;
  REL_UPDATE et REL_DELETION traduisent et prennent le chemin local, comme en service. Témoin réel :
  le .wal de la preuve rejoué sur une copie sous la lib corrigée.
- **B, plus grave : des réponses fausses sous un second écrivain avec G1** :
  `TwoFlushesInOneTransaction` rejoue sans refus mais deux relations désignent les nœuds de l'autre
  écrivain (3000→3001 pour 1000→1001). G1 verse les nœuds locaux en milieu de transaction, ils
  prennent leurs décalages définitifs là, un second écrivain valide derrière ; au rejeu, les
  transactions repassent dans l'ordre des commits et les relations de A, journalisées avec les
  décalages de service, désignent les nœuds de B. Un COPY de nœuds en est protégé parce qu'il tient
  l'index de sa table jusqu'à la validation ; le versement de G1 ne le tenait pas. **Décision** : le
  verrou de versement (flushNodeTable tient sur la table de nœuds le verrou d'un COPY de nœuds
  jusqu'à la fin de la transaction, le second écrivain attend ou est refusé nommément par le délai),
  pas le retrait de G1 (il rouvrirait le ticket du 5 octobre). Reste à mesurer si la mémoire en
  service était juste et seul le rejeu faux. En service l'hôte n'a qu'un écrivain par base : B y est
  peu probable, mais les bases ouvertes de 8 h 31 à 9 h 05 (code-principal, code-lourd,
  memoire-longue) se relèvent par ancre sur des copies au lieu de le croire.
- **Les SIGSEGV restent à part** (ASAN du cœur sur V2 et la compaction CSR ; témoin HNSW de hotfixs
  à refaire sous 512 Mio). G5 a ses chiffres au ticket (13b8a6d57). Aucune pose avant le correctif
  A (et B s'il tient dans l'heure) et le rebâti de la lib.
- **Correction (9 h 40, G et le cœur)** : ma phrase de 9 h 20 « c'est l'écriture sous dadbf0041 qui
  a posé une relation dans un seul sens » est **fausse**. Le cœur a lu le .wal : la relation 187668
  est insérée dans les deux tables jumelles (25 et 27) puis mise à jour ; le journal est cohérent.
  C'est le rejeu qui ne sait pas mettre à jour une relation insérée par un COPY plus tôt dans la
  même transaction — le défaut A, antérieur à dadbf0041. Que les deux libs refusent le même .wal le
  montrait déjà ; nous en avions tiré la mauvaise conclusion. Le correctif du rejeu est sur
  g2b-melange-hnsw (426153052), prouvé sur une copie de la preuve à luciepc. Deux correctifs avant
  tout retour sur dadbf0041 ou une lib suivante : le rejeu (A) et le SIGSEGV (cœur, ASAN).

## 09 h 50 — pour Lucie au réveil : l'essentiel en six lignes

1. **Ta base de code est vivante** (search_code par l'index depuis 9 h 26), revenue à l'état du
   7 au soir et rattrapée par la montre ; ton hôte tourne sur l'ancien binaire et la lib 17a094d12
   après deux SIGSEGV sous la lib neuve (8 h 50, 9 h 05). La pièce du défaut est dans
   `~/.local/share/rag3weaver/preuves/2026-11-08-code-rag3db-depot-wal-dadbf0041/`.
2. **Deux défauts du moteur trouvés et un corrigé** : le rejeu d'un COPY de relations suivi d'une
   mise à jour (antérieur à tout ce week-end ; correctif hotfixs, le .wal réel rejoue sans écart,
   passe en cours) ; le SIGSEGV dans `optimisticRead` (cœur + hotfixs, témoin du mélange G2b en
   cours) ; plus G1 sous un second écrivain au rejeu (verrou de versement, hotfixs) et un débordement
   dans le CREATE_VECTOR_INDEX concurrent (ASAN du cœur). Aucune pose d'hôte avant le rejeu + le
   SIGSEGV corrigés et la lib rebâtie.
3. **La nuit a atterri** : ~25 lots (section par section plus haut) dont trois écrits par l'équipier
   Gemini sous ma supervision — lis `docs/08-novembre-2026-orchestration/01-essais-de-l-equipier.md`
   et sa section « Ce qu'il faut en retenir ».
4. **Le cœur a trouvé un vrai défaut de durabilité** (clé prise par une validation au journal non
   synchronisé) : lot coupure poussé vers 9 h 55.
5. **Décisions qui t'attendent** : § 05 h 20 (G3, le pont Codex, ton manifeste, thème, page 03,
   branches, Vertex) — rien n'est poussé sans ton mot pour G3.
6. **La file en cours** : archi (hooks 3 ter e), 96 (M4 pas 3), ED (429/503), hotfixs (A, puis B,
   puis G3/G5, puis le 3e lot de l'équipier), la coloration B (hooks-essai, codeparsers, tty).

## 09 h 52 — la file du matin a tourné, le lot coupure du cœur est sur master

- **Atterris** : hooks-essai, memoire-nommee (7f1a40e2f : la nature avant le dépôt) ; recherche,
  dense-filtre-sous-l6 (0e528854a : la recherche filtrée revient sous l'instantané de son appel ;
  **exige une lib ≥ dadbf0041 à la pose**, G le sait, aucun binaire qui le contient ne sera posé sur
  17a094d12) ; archi, hooks 3 ter (e) (0768770d7 : vocabulaire sans mots d'arrêt ni noms du juge,
  empreinte blake3, nom rendu borné à 200, les bilans du juge écartés du rappel ; poussé avec un
  rouge nommé hors du lot : `la_sonde_mesure_l_embarqueur_attache`, un témoin de durée qui dérive
  dans la lib entière, vert seul 3/3 deux fois, ticket 279ef8345 — archi le corrige tout de suite
  par une horloge injectée) ; le cœur, **coupure** (d7636375a, commit moteur a312f823e : le journal
  écrit et synchronisé avant la publication, taille rendue sur faute, JOURNAL_BROKEN ; quatre courses
  perdues contre du Rust avant une fenêtre de dix minutes où tout le Rust a été retenu). Depuis
  a312f823e, **MOTEUR_ANCIEN=1** revient à toute passe rag3weaver épinglée sur dadbf0041, jusqu'au
  rebâti.
- **Le cœur, ASAN** : V2, la compaction CSR, VectorSearchDuringInsert, ReadersDuringDelete,
  HalfAppliedCommit, le banc : propres. Deux défauts réels trouvés : un heap-buffer-overflow dans le
  CREATE_VECTOR_INDEX concurrent (ticket 9bf152795, chez hotfixs) et un **heap-use-after-free dans
  l'index HNSW à la fin d'un COPY** (`shrinkForNode` lit le vecteur du nœud dans un tampon que la
  lecture des voisins réalloue — le chemin de la montre ; correctif étroit f97293306, branche
  vecteur-du-noeud-copie). Ni l'un ni l'autre n'explique seul le numéro de page de 330 Gio ; le
  mélange sous ASAN continue avec le correctif.
- **hotfixs** : A (le rejeu d'un COPY de relations suivi d'une mise à jour) en passe complète depuis
  9 h 30, verte vers 10 h 20 ; B mesuré (la mémoire en service était juste, seul le rejeu sous un
  second écrivain fautait), verrou de versement écrit, témoin à jouer.
- **La file** : 96 (à toi à 10 h 01) → ED (equipier-quota : reprise doublée sur 429/503 hors de
  max_failed_steps, le champ inconnu nomme le binaire, `check-manifests` pour le repli de G,
  CARGO_BUILD_JOBS=8 au gabarit) → hotfixs A dès « vert » → la sonde d'archi → recherche (le score
  comparable d'un signal à l'autre : `chunk.signal`, choix du morceau par la contribution RRF,
  `best_signal`, pas de score absolu inventé) → hooks-essai (coloration B). **Le rebâti de la lib**
  portera a312f823e, le rejeu de hotfixs et vecteur-du-noeud-copie ; la pose attend aussi la cause
  du SIGSEGV.

## 10 h 03 — Lucie est réveillée : ses premières décisions

- **G3 : oui**, « rouvre toujours les extensions notées, absolument » → hotfixs mène G3 (1)+(2) au
  push après A, dans le même rebâti si possible ; le refus avec option reste une suite non tranchée.
- **Le pont Codex : plus tard.** Rien à activer pour l'instant.
- **Le thème (tty) : « je vois pas pourquoi attendre mon accord avant de brancher »** → tty branche
  daemon.json tout de suite ; Lucie réagit au rendu. Règle à garder : un rendu ou un design se
  branche et se montre, on ne demande pas d'accord avant.

## 10 h 15 — la cause du SIGSEGV, l'atelier lancé, un chantier de Lucie pour le banc

- **Le SIGSEGV de l'hôte a sa cause, cœur et hotfixs d'accord** : le heap-use-after-free de
  `OnDiskHNSWIndex::shrinkForNode` à la finalisation de l'index par un COPY (le vecteur du nœud pris
  par pointeur dans le tampon que la lecture des voisins réalloue). Le témoin du mélange de hotfixs
  rougit sous ASAN sans le correctif du cœur, vert avec (1/1, 0 rapport, 23 min). **Ce n'est pas
  G2b**, qui reste en service tel quel ; la réindexation y passait parce qu'elle fait des COPY sur une
  table indexée HNSW. Réserve écrite au ticket : la lecture ne produit pas à coup sûr le numéro de
  page de 330 Gio — la preuve sera l'hôte sous la même charge. **Le rebâti** portera a312f823e
  (coupure), A (le rejeu), G3 et vecteur-du-noeud-copie ; G posera **d'abord luciepc** avec une
  réindexation forcée de code-lourd, une heure sans plantage, puis le portable.
- **A** : passe complète rouge d'un seul témoin de temps du banc (`AutoCheckpointIsNotPostponed…`,
  charge 15,8 à luciepc, vert 5/5 seul) → ligne probabilistic avec ses comptes + ticket « un témoin
  de temps du banc rougit sous charge », pas de relâchement ; rejeu court sur l'arbre rebasé, push à
  mon mot. **G3** : Lucie a dit oui (10 h 25, transmis), branche fe8a9dd02 prête pour sa passe.
- **Lucie, au réveil** : G3 oui ; Codex plus tard ; le thème se branche sans accord (tty) ;
  **l'atelier : « oui absolument, je pensais que vous auriez déjà commencé »** → ED en fait son
  chantier principal après son lot des crochets (vert, en attente de push) ; premier pas : un
  gabarit livré `atelier` avec `declare_repo` et `list_repos`, un nœud intégré qui refait
  `code-tree add` et publie `RepoAttached`, témoin en quatre exigences par le chemin de l'hôte,
  vocabulaire de Lucie (dépôt, worktree, branche, index), aucun git lancé à ce pas ; démonstration
  dans l'IDE : « ouvre le worktree … » et le dépôt paraît dans la barre.
- **Un chantier de Lucie** (10 h 50) : « un bon petit modèle de décision, ça coûterait beaucoup
  moins cher — doit-on charger ce contexte au préchargement de mémoire ? » → au banc, en mesure :
  bge-m3 (/rerank, déjà en service), ModernBERT « Laya », Qwen2.5-0.5B et SmolLM2-360M en oui/non
  par les logits, contre Gemini Flash et une relecture à la main de 60 paires ; jeu : les 125
  rappels de hooks.log (le juge différé existe mais est éteint chez Lucie, `recall_judge: false`)
  plus 100 prompts d'autres sessions ; mesures : accord, courbe du seuil (« un filtre ôte »),
  latence CPU/GPU, mémoire ; page `docs/08-novembre-2026-banc/01-modele-de-decision.md`, aucun code
  produit avant lecture ; gardes : Vertex cadencé, CPU d'abord, GPU seulement sur une carte sans
  service (le 25 oct., sept resets).
- **Son manifeste** : fichier neuf prêt dans mon dossier de travail (`rag3db-taches-8-nov.json` :
  task_writes, edit_file, run_command, porte approval, chaîne Rust en lecture ; `sandbox.env` à la
  prochaine pose), geste donné.
- **La file** : recherche (best-signal, à toi) → ED (crochets) → hotfixs A → hooks-essai
  (coloration B, verte) → 96 → tty (thème) → G3 → cœur (vecteur-du-noeud-copie) → rebâti.

## 10 h 33 — six lots de plus, A sur master, le banc bloqué par une permission

- **Atterris** : recherche, best-signal (d540813d5 : `chunk.signal`, le morceau par la contribution
  RRF, `bestSignal` par ligne ; le ticket reste ouvert pour le seuil, à mesurer) ; ED, les crochets
  (8249512e6 : un gabarit nommé se lit dans le gabarit livré, un crochet sans argument requis se
  tait) ; hooks-essai, **la coloration B** (667d5e35a : `POST /highlight` et `/ide/highlight` par les
  grammaires tree-sitter de codeparsers, 13 langues, le chat en `hl-*` avec highlight.js en repli,
  le pointeur à 2bfa28b et son Cargo.lock — leçon : un pointeur qui apporte des dépendances commite
  son Cargo.lock avant la passe) ; hotfixs, **A** (6a71f641d le rejeu d'un COPY de relations suivi
  d'une mise à jour ; c4e213ca5 la ligne probabiliste du témoin de temps du banc et son ticket) ;
  tty, **le thème** (f2e6e2521 : daemon.json embarqué dans templates/themes, les couleurs sur les
  cellules quand le modèle réfléchit ; branché sans attendre, comme Lucie l'a dit).
- **Une faute à dire** : hotfixs a poussé A avec sa passe rag3weaver sur le moteur du worktree
  **rouge**, jugée d'environnement (l'extension vector cherchée à côté de la lib sous
  RAG3DB_LIB=bati, 68 échecs). Un rouge d'environnement reste un rouge tant qu'il n'est pas rejoué
  vert ; il rejoue à l'instant avec l'extension à côté de la lib ; si ça rougit pour une autre raison,
  A revient en arrière dans la minute. Le cœur attend ce verdict pour pousser vecteur-du-noeud-copie ;
  G3 suit avec un verdict complet avant push.
- **Le banc, modèle de décision** : jeu de 166 cas et 1 328 paires, étiquettes Gemini en cours
  (cadencées), bge-m3 scoré (p50 183 ms pour 8 paires) ; deux trouvailles : le juge du rappel des
  hooks **coupe sa réponse sous 400 jetons** (Gemini compte sa réflexion ; ticket d596c829c, à
  archi), et le relecteur refuse une requête de plus de 512 jetons par choix écrit (pas un défaut ;
  le 500 au lieu d'un refus typé, noté). **Bloqué** : sa permission refuse `pip install` et le
  téléchargement des poids depuis huggingface.co à luciepc ; personne ne contourne ; posé à Lucie
  (une règle dans sa session, ou sa main en ssh). En attendant : une page à deux candidats.
- **En cours** : archi, le (f) (`recall_min_score` retiré, la clé acceptée sans effet) puis le
  plafond du juge ; 96, M4 pas 3 (encore rouge : un port de résultat d'outil doit être terminal,
  corrigé, rejeu) ; recherche, « mes recherches » avec les clics (route paginée sur les runs, clic
  fondu dans le run, vue React, règle générique « un outil dont le graphe porte un nœud de
  recherche ») ; ED, l'atelier (atelier-attach df3be297b : `attach_worktree`, `list_worktrees`,
  WorktreeNode, RepoAttached ; passe complète ensuite).

## 10 h 58 — Lucie au clavier : le modèle de décision prend forme, le banc débloqué

- **A confirmé** par une mesure honnête : la passe rag3weaver entière sur la tête de master (A +
  coupure) liée à un moteur bâti dans rag3db-dbg (lib et extension à côté, md5 relevés) :
  check_features 5/5, lib 2045, e2e_lectures 24/24, e2e_tx 3+1+18+3. Le montage mixte (lib de A,
  en-têtes de dadbf0041) avait rendu 20 refus de build id et a été jeté. Le cœur rejoue
  proportionnellement sur sa tête rebasée (A a touché wal_replayer en C++) : transaction entière,
  e2e vector, produit court ; push vers la demi-heure. **G3** en passe complète sur
  g3-extensions-a-l-ouverture-3 (ddd35d857, A dessous ; deux conflits résolus : nouveautes.md et
  probabilistic.txt), verdict complet avant push.
- **Atterris** : tty, la coloration du code dans le terminal (3696a71cb, par le vrai port de
  POST /highlight) ; archi, le (f) (d4c1d1b12 : `recall_min_score` retiré, la clé acceptée sans
  effet). En push : recherche, mes-recherches (route paginée sur les runs, le clic fondu dans le run,
  la vue, et une course du journal des runs corrigée au passage). Prêts : archi (g) (le juge du
  rappel : plafond 2 048 et coupe dite ; la session qu'il sert dans la consigne, la fiche propre
  ôtée — c7bd53250) ; 96 (M4 pas 3, un port terminal pour le rapport, rejeu).
- **Le modèle de décision (banc)** : première mesure sur master (1ab3398b3, a63182975) — « ce jeu ne
  classe aucun modèle » : 1 328 paires réelles, 19 fiches, aucune utile à la main ; Gemini garde 66
  fois sur 67 la fiche d'identité de la session parce que sa consigne ne lui dit pas quelle session
  il sert ; bge-m3 au hasard (AUC 0,53) ; 202 appels, 466 000 jetons. **Lucie tranche** : fabriquer
  ou construire de vraies fiches, la permission d'installer « sans souci », et **l'entrée du modèle
  est le contexte de l'agent** (« on lui donne tout le chat et les outils, et on lui dit : ce bout
  de mémoire est-il pertinent avec le contexte de l'agent »). Le banc construit ~80 à 100 positifs
  depuis les transcriptions (la fiche qui aurait aidé, provenance vérifiée, sans recopier les mots
  du prompt), 2 négatifs durs + 2 au hasard par cas, contexte borné à 2 k / 6 k / 16 k jetons,
  Gemini par cas comme référence (≈ 520 appels, 4,8 M de jetons, accordé) ; mesures : accord,
  AUC, seuil qui garde 95 puis 100 % des utiles, latence CPU puis card0, mémoire. **Installations
  faites de ma main sur l'autorisation explicite de Lucie** (12 h 25) : venv transformers 5.22 +
  laya 0.4.29 à luciepc, poids laya 647 Mo, Qwen 0,5B 954 Mo, SmolLM2 4,7 Go (formats en trop à
  élaguer).
- **Lucie** : « après les quotas, une minute puis un retour exponentiel » → la règle du client
  `models.llm` lui-même (chat, juge, archiviste, hooks), lot court chez ED derrière l'atelier.
- **L'atelier** (ED) : premier pas écrit (atelier-attach df3be297b : gabarit livré `atelier`,
  `attach_worktree {path}` et `list_worktrees`, WorktreeNode, RepoAttached), modules puis passe.

## 11 h 18 — les README remis à jour, la file de la mi-journée

- **Atterris** : recherche, mes-recherches (932197e79 avec la relecture d'ED, et la course du
  journal des runs corrigée) et sa ligne de nouveautés (ad07c8b16) ; ED, le ticket du témoin du
  juge des outils (9d748de62 : il exige un genre que Gemini ne tient pas — la correction admet les
  deux genres pourvu que la preuve cite `required`, lot court après l'atelier) ; archi, le (g)
  (9daa2a450 : le juge du rappel sait quelle session il sert, plafond 2 048 et coupe dite ; trois
  passes pour deux courses) ; **lucivy, les deux README** (58b762a35 racine, 7f9a03515 rag3weaver,
  +167 −48, depuis les nouveautés, chaque hash dans le commit ; l'archiviste « en cours », le pont
  Codex « testé contre un substitut », rien sur G3 ni le HNSW) et deux corrections de nouveautés
  (72f17733a). **Lucie, 12 h 50** : « on met à jour le readme de temps en temps ? » → règle : un lot
  dont la ligne de nouveautés change ce que l'utilisateur voit met le README à jour ; rattrapage
  hebdomadaire sinon.
- **La CI de master est rouge depuis le 18 octobre** (ticket du 25 oct., `<cassert>` dans geo et
  le format clang) et son correctif baf19b9e5 n'a jamais atterri : à hotfixs avec G3.
- **En cours** : recherche, mes-recherches-suite (en push) ; cœur, vecteur-du-noeud-copie (rejeu
  sur la tête rebasée, A touchait wal_replayer) ; hotfixs, G3 (passe complète avec A dessous) ;
  96, M4 pas 3 ; ED, l'atelier (passe complète) puis les quotas du client LLM et le témoin du juge ;
  archi, la barre latérale du chat (usage en jetons par tour et cumulé, tâches de l'archiviste,
  récapitulatif et fiches sous chaque conversation, « Dépôts » sur RepoAttached) ; banc, la
  construction des positifs (8 sur 130 à 12 h 40) puis la référence Gemini et bge-m3 — les petits
  modèles attendent que sa propre permission d'exécuter soit levée par Lucie dans sa fenêtre (il a
  dit, à raison, qu'il ne pouvait pas vérifier d'ici l'autorisation donnée à moi) ; lucivy, la
  liste des 633 branches distantes pour Lucie.
- **Le rebâti** suit le hash du cœur : l'arbre bâtit la lib (a312f823e + A + HNSW, + G3 s'il est
  là), G pose luciepc d'abord sous une réindexation forcée de code-lourd (une heure, deux sens
  relevés), puis le portable avec check-manifests et un instantané reflink avant.

## 11 h 42 — une correction d'heures

Les titres des six sections précédentes (de « 09 h 52 » à « 11 h 18 ») portaient des heures en avance
d'une heure à une heure et demie (« 10 h 05 » à « 13 h 00 ») : j'estimais l'heure au lieu de la lire.
Ils sont remis sur l'heure réelle du commit de chaque section. Les heures citées **dans le texte** de ces
sections pour les mots de Lucie et les événements du matin sont approximatives et peuvent être en
avance d'autant ; les hashs, eux, sont exacts.

## 11 h 58 — l'atelier et la barre du chat sur master, la lib se bâtit

- **Atterris** : recherche, mes-recherches-suite (022ade8e8 : pages de 2 000 runs, `relancable`
  par l'hôte, outils lus sans ouvrir la base) ; lucivy, les deux README (58b762a35, 7f9a03515) et
  les corrections de nouveautés (72f17733a), puis **la page des 732 branches distantes**
  (bce8a72ed, `docs/08-novembre-2026-orchestration/02-branches-distantes.md` : 540 à effacer,
  21 à garder, 171 à demander dont 94 de l'amont Kuzu — rien d'effacé, Lucie décide) ; le cœur,
  **vecteur-du-noeud-copie** (f7df88003 : le témoin de hotfixs et le correctif cbc06c3e1 ;
  transaction 351, e2e vector 74, produit vert) ; archi, **la barre du chat** (2daf2e332 +
  c76339105 : l'usage en jetons du tour et des tours récents lu au journal des runs, le
  récapitulatif et les fiches sous chaque conversation, les tâches de l'archiviste qui survivent au
  rechargement) ; ED, **le premier pas de l'atelier** (8258fe56b : `rag3weaver-backend atelier`,
  `attach_worktree {path}`, `list_worktrees`, WorktreeNode, RepoAttached) et le témoin du juge à
  deux genres (abf7020e0, ticket fermé).
- **Le rebâti** : l'arbre bâtit la lib à luciepc sur f7df88003 (nommée cbc06c3e1 par le magasin),
  « fini » attendu vers 12 h 15 ; G pose ensuite luciepc une heure sous charge, puis le portable
  avec un binaire ≥ abf7020e0. **La démonstration de l'atelier** est donnée à Lucie (cinq pas).
- **G3** : la passe rougissait sur les attentes des anciens .test (la colonne `extension_error`,
  `-RELOADDB` qui recharge l'extension, les écritures qui passent après RELOAD DB) — attentes
  mises au comportement tranché, le refus nommé gardé dans le témoin C++ avec MERGE et COPY ;
  passe rejouée. **La CI** : le `<cassert>` était déjà sur master (25 oct.) ; ses runs sont
  annulés avant leur premier job par la concurrence des pushes, ni rouge ni verte ; le commit de
  formatage (clang-format 18 en conteneur, ≈ 300 fichiers) attend la fenêtre de cette nuit ;
  garde-fou posé : chaque lot reformate ses seules lignes (A 4, G5 3, B 15 corrigés).
- **Lucie, midi** : « après les quotas, une minute puis exponentiel » → la règle passe dans le
  client `models.llm` (ED, prochain lot, avec le 429 du flux Vertex classé refus) ; « le README
  de temps en temps ? » → règle dans la procédure (lucivy) ; le modèle de décision : fabriquer de
  vrais positifs, l'entrée est le contexte de l'agent, l'installation autorisée (faite de ma main ;
  le banc attend qu'elle lève le refus d'exécuter dans sa fenêtre).
- **Une faute à moi** : six titres du rapport en avance d'une heure et demie, remis sur l'heure des
  commits ; règle en mémoire (lire l'heure avant de l'écrire).

## 12 h 15 — la lib corrigée est posée à luciepc, l'heure de preuve tourne

- **La lib** : rag3db-build-cbc06c3e1… (tête f7df88003 : coupure, le rejeu des COPY, le HNSW), bâtie
  par l'arbre, verdict vert sans MOTEUR_ANCIEN, deux sens sans écart sur quatre copies d'avant-43,
  ABI compatible contre dadbf0041 (0 retiré, 2 ajoutés ; librag3db.so change aussi, par ajouts
  seuls), copiée aux deux postes ; **les passes l'épinglent sans MOTEUR_ANCIEN** dès maintenant,
  `courant` y passera avec la pose du portable. Le binaire : 9208a2b6 sur abf7020e0 (atelier,
  barre du chat, hooks e/f/g, borne de la recherche).
- **Pose à luciepc à 12 h 06**, mais le réveil a refusé : **granite-278m (7878) en
  `failed (start-limit-hit)` depuis 10 h 47 m 54 s** (code 1 après 7 s, huit départs) ; 7879 et
  7880 répondaient. Embarquement l'a relancé en un départ contrôlé à 12 h 07 m 30 s (768 dimensions,
  écart nul sur le vecteur témoin). La cause de 10 h 47 n'est plus lisible (le journal --user avait
  tourné, pas de dmesg sans sudo) : ticket f66ad4449 sans cause inventée, et des **journaux
  persistants par unité** (StandardOutput en append dans ~/.cache/rag3weaver-unites-embarquement/)
  pour la prochaine fois. Hôte de luciepc réveillé à 12 h 07 m 56 s ; hotfixs a ses copies de
  code-lourd et code-principal (coupure-12h06) ; l'`index confirm` de code-lourd a pris sa place
  de lourd à 12 h 09 — **l'heure de preuve court jusque vers 13 h 10** (guet : SIGSEGV, NRestarts).
  Puis les deux sens sur une copie, puis le portable avec check-manifests et un instantané.
- **Sur master depuis midi** : recherche, mes-recherches-suite ; lucivy, la règle du README dans la
  procédure (25effedad) ; embarquement, le ticket de 7878 ; ED, l'atelier (8258fe56b) et le témoin
  du juge (abf7020e0) ; archi, la barre du chat (2daf2e332, c76339105).
- **En cours** : hotfixs, G3 rejoué (attentes des .test mises au comportement tranché) ; 96, le
  rejeu du pas 3 sous cbc06c3e1 ; hooks-essai, **la tâche sans git** (verte sur branche : copie,
  relecture par diff de dossier, fusion par recopie, `git_init` sur déclaration ; ED relit, puis
  la passe avec les e2e_tx) ; recherche, le cache de Catalog hors de /tmp et le témoin de suite ;
  archi, « Dépôts » sur RepoAttached ; ED, la reprise exponentielle dans le client models.llm ;
  banc, la construction des positifs et la référence Gemini (les petits modèles attendent le geste
  de Lucie dans sa fenêtre) ; lucivy, libre.
- **Pour Lucie** : la démonstration de l'atelier (cinq pas) et les deux lignes de tty pour le
  terminal sont prêtes pour la pose du portable ; la page des 732 branches attend ses deux mots.

## 13 h 11 — G3 sur master, le verdict de luciepc, la pose du portable à 13 h 40

- **Atterris** : hotfixs, **G3** (a9067fa2b : toute ouverture relit ses extensions notées, l'échec se
  dit par `extension_error` sans refuser ; verdict complet : liste C++ avec le banc, transaction
  353/353, e2e vector 74 + 63, rag3weaver sur son moteur 2051 ; l'accord de Lucie dans le commit)
  → **MOTEUR_ANCIEN=1 pour tous** jusqu'au rebâti de la nuit ; le commit de format de A (c6c84af6a) ;
  ED, **la reprise du client models.llm** (aed8184b4 : une minute puis le double, dix minutes au plus,
  cinq reprises, 503 et 5xx RESOURCE_EXHAUSTED traités comme un quota, le 429 glissé dans le flux
  rejoué, l'erreur qui dit sa durée ; chat, juge, archiviste, hooks ; le client Anthropic hors du lot) ;
  archi, **« Dépôts » sur RepoAttached** (1005c5544) et **le thème daemon dans l'IDE** (44442ea75 : le
  fichier commun servi par l'hôte, un troisième choix, l'invocation qui tourne pendant que le modèle
  réfléchit) ; archi, la page « l'IDE, ce qui reste » (b181dbadf : tout fait sauf le lien vers un
  symbole, qu'il prend) ; codeparsers, les rejugements (904c34fff : l'indexation ne changeait rien,
  le juge strict oui ; TypeScript 243/39 dont 17 liaisons d'import et 21 locales homonymes →
  correctif `visible_until_line` vert dans codeparsers, mesure du gain en cours) ; embarquement,
  le ticket de 7878 (f66ad4449).
- **Le verdict de luciepc sur cbc06c3e1** (G, 13 h 03) : aucun plantage, mais la charge a été
  légère (deux COPY indexés de 4 min 30 et 3 min 30, vecteurs réutilisés ; un arrêt d'inactivité
  normal entre les deux). Le cœur l'a dit : ce n'est pas une preuve ; **le banc « rouge d'abord »**
  (dadbf0041 sur une base neuve réembarquée avec search_code et read_file en boucle, puis cbc06c3e1
  deux fois le délai) tourne en parallèle sur un hôte d'essai à luciepc — j'ai choisi la pose
  maintenant plutôt que trois heures d'attente, le mécanisme étant prouvé par l'ASAN et le repli
  automatique ; les recherches denses de Lucie ralentiront pendant le réembarquement. Un écart du
  plein texte (103 documents de plus que de lignes dans Scope, 102 sur code-principal) en ticket.
- **La pose du portable à 13 h 40** (G) : binaire d982b513 sur ff6e88ddf contre cbc06c3e1,
  check-manifests ok sur les deux arbres, le contrôle des deux sens sur les copies de luciepc
  d'abord, puis « arrêté » → instantané avant-cbc06c3e1 de code-rag3db-depot et memoire-longue →
  copie de memoire-longue par hotfixs pour le relevé → `courant` = cbc06c3e1 aux deux postes →
  réveil → sandbox.env dans mon arbre → « posé ». Le relevé des deux sens de hotfixs sur
  code-lourd et code-principal : 0 écart sur 36 tables (la forme A visible, la forme B ne naît qu'au
  rejeu à deux écrivains).
- **En cours** : ED, open_branch (à toi) puis le juge par pas, le réacteur ; hotfixs, **B** (le
  verrou de versement + un oubli trouvé : acquireLocksBeforeExecution ne visitait que le puits de
  chaque pipeline, IndexLookup n'était jamais appelé ; corrigé, le double versement est vert) puis
  le troisième lot de l'équipier, G5 ; hooks-essai, la tâche sans git (un intermittent de modèle
  nommé en ticket, le script d'affichage réparé, rejeu à 13 h 10) ; 96, M4 pas 3 ; archi, le lien
  vers un symbole ; recherche, le cache hors de /tmp ; codeparsers, les locales JavaScript ; banc,
  les positifs et la référence Gemini.

## 13 h 54 — ÉCART : la pose du portable est annulée

- **Le contrôle des deux sens** (outil 339cbb5dc, sous cbc06c3e1) sur les copies de code-lourd et
  code-principal prises à 13 h 03, hôte arrêté, **après les deux `index confirm` complets de G sous
  cbc06c3e1**, rend « BASE ATTEINTE » : code-lourd INHERITS_FROM 650 avant / 658 arrière (1 d'un
  seul côté, 11 vues deux fois, 13 aux bouts différents), QUERIES (5 / 6 / 9), LOCKS 544 / 546 ;
  code-principal QUERIES (7 / 8), IMPLEMENTS 331 / 334 avec `line` et `resolution` à deux valeurs,
  LOCKS 425 / 436. Les grosses tables (MENTIONS, CONSUMES/CONSUMED_BY, DEFINES, CHUNKED_FROM) sont
  égales. Les copies d'avant les index (hotfixs, 12 h 06 m 30 s) faisaient **0 écart sur 36 tables**,
  comme les copies d'avant-43 sous cbc06c3e1 (arbre). **Un index complet, qui réécrit toutes les
  lignes (DELETE puis COPY de relations), laisse des relations dans un seul sens ou aux bouts
  différents dans les tables jeunes** : la famille LOCKS/QUERIES du 25 octobre. Pas de .wal sur ces
  copies : ce n'est pas le rejeu, c'est l'écriture en service posée par un point de reprise.
- **G n'a rien avancé** : pas de pose au portable, `courant` = dadbf0041 aux deux postes, le portable
  sur eb154bf5f + 17a094d12. Les bases en service de luciepc portent ces écarts (bases d'essai) et
  reviendront sur les copies de 12 h 06 après le départage. Logs à luciepc :
  ~/.local/share/rag3weaver/instantanes/preuve-cbc06c3e1-controle/.
- **Le départage** (G, hôte d'essai 7991, sous poste lourd) : le même `index confirm` sur une copie
  saine de code-lourd sous 17a094d12, dadbf0041, puis cbc06c3e1, chacun suivi du contrôle. Rouge
  sous cbc06c3e1 seul → un de A, a312f823e, b142b6a71, cbc06c3e1, et hotfixs bissecte ; rouge sous
  les deux ou les trois → plus ancien, la famille LOCKS que B ne couvre pas. **Règle jusqu'au
  verdict : aucune indexation complète sous cbc06c3e1.** La base de Lucie (avant-43, 17a094d12)
  n'est pas touchée ; ses deux sens se contrôlent sur une copie, une minute d'arrêt annoncée.
- Le banc « rouge d'abord » du SIGSEGV passe après ; B continue sa passe.

## 14 h 39 — la base de Lucie saine, le départage relancé, un rouge de master sur l'index d'une tâche

- **La base de code de Lucie est saine** (G, 14 h 15) : contrôle des deux sens sur l'instantané pris à
  13 h 59 sous 17a094d12 — LOCKS 543/543, QUERIES 22 977/22 977, INHERITS_FROM 639/639, IMPLEMENTS
  373/373 ; elle n'a pas eu d'index complet, seulement la montre.
- **Le départage des libs est relancé** (G, 14 h 26) : les deux premiers runs n'avaient rien indexé (le
  manifeste copié hors de son arbre perdait ses chemins relatifs ; le « 0 écart » du run 1 ne portait
  que sur la copie saine) — vu et dit par G ; le script copie maintenant tout l'arbre des gabarits,
  rougit si l'ouverture ou l'index échoue, et n'accepte le contrôle qu'après « indexation terminée ».
  Runs 1b (17a094d12), 2b (dadbf0041), 3b (cbc06c3e1), même binaire Rust pour les deux derniers,
  copie saine de 12 h 06 à chaque fois, lecture de charge et de GPU à chaque fin ; verdicts vers
  15 h 40. Les témoins à un seul écrivain de hotfixs (réindex complet en trois tours ; deux versements
  d'une table) font 0 écart : il manque le volume, les groupes CSR persistés, un lecteur pendant
  l'index. Le cœur lit la forme comme des colonnes de nœuds remappées deux fois (G1 à un écrivain) ;
  si 17a094d12 est sain et dadbf0041 malade, G1 (c4f8610b9) se retire avant tout rebâti.
- **Un second signal** (ED) : `taches::tests::la_base_principale_n_est_jamais_ecrite_par_la_tache`
  rougit **seul sur master e8a91105f, 2 fois sur 3**, sous cbc06c3e1 (« la tâche n'a pas indexé en
  60 s », writing: true), à trois lourds sur six ; hooks-essai a le même rouge dans sa passe. ED le
  départage par la lib (17a094d12, dadbf0041, cinq fois chacun, le journal du run à 60 s) ; un index
  de tâche qui ne finit plus et un index complet qui abîme les deux sens, le même jour sous la même
  lib, se lisent ensemble. Personne ne pousse de Rust qui touche l'index avant le verdict ; les
  autres lots passent avec ce rouge nommé et chiffré.
- **Lucie** : « attention je joue un peu aux jeux vidéo depuis luciepc (seat1) » → seat1 rend sur
  card0 (la carte dite libre), pas sur celle des services ; toute mesure à luciepc porte sa lecture
  de charge et de GPU à côté du chiffre (lecture-poste.sh de G). « Il ne faut pas qu'on s'imagine
  des barrières à essayer des modèles » → la règle du 25 octobre redevient ce qu'elle était : lire
  la VRAM, charger si ça tient, un modèle à la fois, arrêt au premier reset ; jamais « CPU seulement »
  par prudence. **Elle a levé la permission du banc dans sa fenêtre** (14 h 28) : les petits modèles
  tournent (Qwen 0,5B 3,9 Gio, SmolLM2 3,1 Gio, Laya 3,0 Gio en CPU ; passe CPU à la borne 2 000,
  puis GPU pour 6 000 et 16 000) ; le jeu à vrais positifs : 88 positifs à provenance vérifiée,
  440 paires, 1,7 M de jetons de construction ; bge-m3 AUC 0,61 (ne sert pas) ; la référence Gemini
  en cours.
- **Atterris** : ED, open_branch (0619c22b6) ; embarquement, le ticket de 7878 sans l'hypothèse du
  jeu (e8a91105f). **Prêts** : 96 (en push), archi (le symbole cité en lien : `POST /ide/symboles`
  par `usages`, l'appel discret qui ne journalise pas un rendu), hooks-essai (la tâche sans git,
  avec le rouge de master nommé), ED (le juge par pas, idem).

## 14 h 48 — 7878 fait la queue pour tout luciepc pendant le départage

- **Le rouge de master sur l'index d'une tâche s'explique par la file de 7878** (hooks-essai, ED) :
  l'index d'une tâche écrit « indexation lancée : 1 fichiers, granite-278m » puis attend ; /sante
  répond en 0,3 ms mais les runs de départage de G réembarquent tout code-lourd (137 000 morceaux)
  sur 7878 depuis 14 h 26, et le banc chargeait ses modèles : /embed fait la queue, le délai de 60 s
  de l'arbre le dit (« pas indexé en 60 s ») au lieu de figer comme la nuit. e2e_lectures seule chez
  hooks-essai : 19 rouges sur 24 avec les mêmes signes, verte 24/24 à 13 h 10 avant les runs. ED
  départage quand même par la lib avec une sonde /embed toutes les 5 s pendant chaque essai (un
  rouge avec /embed lent = la file ; avec /embed rapide = l'index). Règle du moment : G dit la fin
  de la phase d'index de chaque run et « 7878 libre » à la fin ; les e2e des autres se glissent entre.
- **Le jeu de Lucie** : vérifié sur luciepc, Bloodborne (bb-probe, gamescope, Steam, seat1) ne rend
  que sur card0 (renderD130, 85 %, 5,6 Go) ; les trois services d'embarquement sur card2
  (renderD129, 21,5 Go : granite-278m 7,7 Go, bge-m3 8,6 Go, granite-107m 5,1 Go — trois modèles,
  voulu). Son jeu a été lent à démarrer : la charge CPU du poste (21 à 28), pas la carte. Le script
  du banc tenait des descripteurs sur les deux cartes (torch ROCm énumère tout à l'init) : passes
  CPU relancées avec les cartes masquées (44 Kio de contexte, aucun calcul), passes GPU bornées à
  card2 par HIP_VISIBLE_DEVICES, VRAM de card0 relue avant et après.
- **Codeparsers** : le correctif des locales JavaScript mesuré sur les mêmes 298 mentions — faux
  30 → 10, aucun juste perdu, 78 arêtes justes de plus hors échantillon ; lot en passe (pointeur
  c660893, le verdict « liaison d'import » dans l'oracle, ticket fermé) ; le suivant : le `require`
  relatif. **Archi** : le symbole cité en lien est prêt (`POST /ide/symboles` par `usages`, l'appel
  discret qui ne journalise pas un rendu). **96** en push. Tous les pushes attendent le dernier pas
  seul sous poste push ; les passes rendent leur place lourde au vert.

## 15 h 14 — le rouge de master innocente la lib : c'était la file de 7878

- **Le départage d'ED** sur `la_base_principale_n_est_jamais_ecrite_par_la_tache`, test seul sur
  master e8a91105f : cbc06c3e1 pendant l'index de G sur 7878 → 2 rouges sur 3 ; cbc06c3e1 dans la
  fenêtre libre (15 h 08 – 15 h 12) → 0 sur 5 ; dadbf0041 → 0 sur 5 ; 17a094d12 → 8 sur 8 (lib trop
  ancienne pour le Rust de master, ne dit rien). **La lib n'y est pour rien : l'index de la tâche
  attend l'embarquement de son fichier derrière les gros lots du départage**, et le délai de 60 s
  tombe. Une sonde /embed courte restait à 0,2 s : un texte court ne voit pas la file des gros lots ;
  la mesure juste est l'âge du lot en attente. L'interdit sur le Rust de l'index est levé ; ED
  fera un lot court pour que l'index et le témoin nomment leur attente (« attend le service 7878
  depuis N s, N morceaux en file »). hooks-essai a confirmé dans la fenêtre : e2e_lectures 24/24 en
  18,5 s, le test des tâches en 10 s.
- **luciepc saturé** : charge 33 à 36 sur 24 cœurs à 6 lourds (le réembarquement du départage, la
  mesure des résidus, le banc à 8 fils, une lib entière) ; les témoins de temps rougissaient partout,
  une passe de 20 min en prenait 45. **lourds-max ramené de 6 à 4** à 15 h 20 ; on remonte quand la
  file est courte. Le run 1b du départage (17a094d12) : index fini à 15 h 07 après 41 min (137 943
  morceaux réembarqués), contrôle en cours ; les runs 2b et 3b prendront ~40 min d'index chacun —
  verdicts vers 17 h ; G annonce la fin de chaque phase d'index pour que les e2e des autres se
  glissent entre.
- **Atterri** : hooks-essai, **la tâche sans git** (d987d2143, par fusion en avance rapide de
  l'arbre exactement joué : une racine hors dépôt se travaille dans une copie, relecture par diff de
  dossier, fusion par recopie, `git_init` sur déclaration ; l'intermittent de l'équipier nommé en
  ticket, le script d'affichage réparé). **En push** : archi (le symbole cité en lien) ; **en file** :
  96 (M4 pas 3, rejeu seul dans la fenêtre), codeparsers (les locales JavaScript : deux témoins de
  temps verts seuls, suites en cours), ED (le juge par pas), recherche (le cache hors de /tmp).

## 15 h 39 — le run 1b tranche : l'écart des deux sens existe déjà sous 17a094d12

- **Run 1b de G** (17a094d12, binaire 41411ff0, copie saine de 12 h 06, `index confirm` complet de
  14 h 26 à 15 h 07, arrêt sans .wal) : **ÉCART**, les mêmes chiffres qu'à midi sous cbc06c3e1 —
  INHERITS_FROM 650 / 658, LOCKS 544 / 546, QUERIES 5 d'un seul côté, 6 vues deux fois, 9 aux bouts
  différents, IMPLEMENTS égales. Le défaut est **antérieur aux lots du jour** (la famille LOCKS du
  25 octobre), déterministe ; les runs 2b (dadbf0041, en cours, 7878 occupé jusqu'à ~16 h 18) et 3b
  (cbc06c3e1) confirmeront sans changer la décision. Instantanés intacts à luciepc pour le moteur :
  departage-1b-17a et preuve-cbc06c3e1.
- **Décisions** : **la pose du portable reprend** (cbc06c3e1 n'ajoute pas ce défaut et retire le
  SIGSEGV HNSW et le rejeu des COPY ; même déroulé qu'à 13 h 40, `courant` → cbc06c3e1 aux deux
  postes) ; **aucun `index confirm` complet sur les bases de Lucie jusqu'au correctif** — la montre
  seule ne fait pas naître l'écart (contrôle de 14 h 15 vert sur sa base) ; **le chantier moteur
  passe devant B chez hotfixs**, ticket gravité perte, la recette reproductible (copie saine, index
  complet, contrôle 339cbb5dc), la bissection par le chemin (DELETE puis COPY de relations dans une
  même transaction sur une table déjà au disque, groupes CSR persistés : ce que les témoins à un
  écrivain n'avaient pas) puis par les commits d'avant 17a094d12 ; le cœur lit avec lui.
- **Atterris** : archi, le symbole cité en lien (36aced578) et la page 03 sans « pas fait »
  (d59812592) — la liste de Lucie d'hier soir sur l'IDE est faite en une journée. Codeparsers : un
  troisième rouge sous charge au navigateur (un module de l'aperçu à chaud servi en 422 juste après
  le vidage du magasin, vert seul 2/2) : une course possible, à archi avec la relecture d'ED.

## 16 h 40 — le portable est posé deux fois, Lucie essaie l'atelier, ce que son essai révèle

- **Le défaut des deux sens est plus ancien que les V2** : hotfixs retrouve la demi-arête LOCKS sous
  58b46a9bf (25 octobre) ; le chemin désigné est le point de reprise CSR qui réécrit une base
  pleine (`populateRegionCSROffsets` écrase le bord droit d'une région sous un `KU_ASSERT`, la garde
  de densité ne lit pas les longueurs qu'il range — hypothèse, dans le ticket avant la preuve). Le
  run 2b (dadbf0041) rend les mêmes chiffres que 1b. Le run sous 04063ee06 est désarmé (base de
  départ différente, 40 min de 7878) ; hotfixs mène, le cœur l'aide ; sa trace d'`index confirm`
  tourne depuis 16 h 21 sur un hôte d'essai 7992 avec **un service d'embarquement à lui (7882)** —
  j'avais cru qu'elle saturait 7878 et je me suis trompé, dit à hotfixs. memoire-longue est saine
  (0 écart sur 18 tables, copie prise à l'arrêt). Trois témoins de forme verts chez hotfixs : la
  forme ne suffit pas, la trace du vrai régime est nécessaire ; accord d'avance pour un binaire à
  trace JSON si la forme ne suffit pas.
- **Deux poses du portable** : 15 h 41 (ff6e88ddf, G coupe quatre minutes avant l'heure annoncée
  parce qu'il s'était fié à une heure non lue : dit tel quel) puis **16 h 08** (e59e59834, parce que
  `open_branch` 9fd49b38d n'était pas dans le premier binaire ; coupure de 7 s). `courant` →
  cbc06c3e1 aux deux postes, tunnels vérifiés, 0 core. Le 7878 de luciepc est rendu aux pushes à
  16 h 08.
- **L'essai de l'atelier par Lucie** (16 h 00 → 16 h 35), et ce qu'il a révélé, chaque point avec
  son lot et son témoin rouge d'abord :
  1. le modèle dit « je ne peux pas exécuter de commande shell, faites-le dans votre terminal » :
     `open_branch` manquait au binaire (repose) **et** au manifeste de l'atelier, copié du gabarit
     livré de l'ancien binaire et jamais remis à jour (« atelier déjà posé »). Lucie : « il faut
     1 - qu'il ait commande shell, 2 - des outils spécialisés tenus par notre framework » et « que
     ce soit dynamique, qu'on n'ait pas besoin de tout recharger manuellement à chacune de nos
     avancées ». → ED, pile `gabarits-vivants` : commandes de l'atelier hors tâche par sa porte
     (approbation), `chat_instructions`, **gabarits vivants** (`<manifeste>.livre`, remplacé à
     l'ouverture par l'hôte s'il est resté tel qu'écrit, `TemplateRefreshed` sur le flux) ; geste
     manuel fait une fois pour Lucie (templates déplacé, `atelier` rejoué, `open_branch` déclaré) ;
  2. `list_worktrees` dit « plein texte 4/7, vecteurs 3/7 » pour la base principale de Lucie, qui
     croit qu'un index tourne dessus : `fts_counts`, `reads`, `watches` comptés comme des entités,
     `not_declared` compté comme un manque ; et « 0/4 » pendant un index au lieu de « en cours » →
     ED a538a1575 et 878c874da ;
  3. le `workspace` vide du gabarit est listé comme un worktree attaché → ED ;
  4. **l'index d'un worktree attaché ne reprend pas après un redémarrage de l'hôte** : wt-react5
     attaché à 16 h 04, hôte reposé à 16 h 08, « index pas ouvert » et rien ne le rouvre ; rouvert
     à la main (POST /backend/open) l'index HNSW est reconstruit mais **le rattrapage des vecteurs
     ne repart pas** (15 % depuis 16 h 12 : ED lit que rien ne solde la dette à l'ouverture, seul
     l'outil `index` le fait). Tranché **(b)** : `embeddings.catch_up_on_open` déclaré dans le
     gabarit livré du code, hérité par les worktrees ; la base de Lucie, posée sans lui, ne bouge
     pas ; un rattrapage de fond s'arrête au paquet suivant sur /backend/close au lieu de refuser
     (ED 744d568eb, 67247bfbb, c7d0a7b35). wt-react5 relancé à la main par `index confirm` à
     16 h 34 (base jetable) ;
  5. l'accueil de l'IDE dit encore « Backends » et des chemins de manifeste → archi
     `ide-accueil-depots` (f5c8a6868, passe verte, en file) ;
  6. Lucie : « il devrait avoir notif quand indexation finie », « une barre de progression qui
     glisse dans le chat quand je charge un worktree, ou un handle pour l'agent », « par SSE, pas
     par polling, voire rendre générique quand on a besoin d'un SSE » → archi + ED : `RepoIndexed`
     à la dernière ligne du journal d'index, `IndexProgress` sur le flux (1/s au plus par backend),
     un champ `ui` sur les résultats d'outils (`{kind, repo, sujets}`) rendu par la page sous le
     tour et branché sur le flux déjà ouvert, `show_progress` comme handle ; zéro polling, règle
     pour toute progression à venir.
- **Mesures** : un worktree de rag3db s'indexe en **13 min** par le tunnel vers 7878 (essai-atelier :
  111 024 Scope_Chunk en 10 min 20 s, ~160 morceaux/s) ; pendant ce temps la carte des services
  de luciepc est à 26–51 % et le service à 12 % d'un cœur : **sous-utilisée, pas bloquée**, les
  clients envoient un lot et attendent (Lucie : « j'entends pas fort travailler les GPU de
  luciepc »). Pas de service sur le portable (Lucie : « le portable devient lourd dingue »). La
  vraie réponse est la marche 1 des vues par worktree (6 novembre) : un worktree de rag3db
  réembarque ce que la base principale a déjà.
- **La file** : 96 (M4 pas 3) et codeparsers (locales JS) ont chacun un seul rouge, le témoin
  `un_symbole_cite_se_resout_par_l_index_sans_ouvrir_de_depot` de 36aced578 sous charge (« catalogue
  tenu par une écriture ») ; ED porte le correctif (relire tant que `why` le dit, 10 s) dans
  juge-par-pas-3 ; 96 attend que les index du portable finissent pour rejouer seul ; codeparsers
  rejoue court et pousse ; archi ensuite avec deux lots ; puis ED, recherche (un `.tmp` de 4 Ko
  intermittent, cherche la source). Quota Vertex épuisé 24 min vers 15 h 40 : pas le banc (fini
  à 14 h 51, en série), nos passes à Gemini jouées en même temps. Le banc passe ses reprises au
  doublement.
- **Règle apprise** : un index de mesure ou de trace tourne sur **son propre** service
  d'embarquement (même modèle, port neuf, VRAM lue), jamais sur le 7878 partagé — il n'y a pas
  de fenêtre dans une file de pushes.
