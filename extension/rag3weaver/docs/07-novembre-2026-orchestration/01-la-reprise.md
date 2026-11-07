# La reprise du 7 novembre 2026 : ménage, atterrissages, un bloquant

Lucie a relancé toutes les sessions ce matin (noms du 25 octobre, plus
« rag3weaver archi », l'orchestration des 3-4 octobre devenue le chantier E).
Trois consignes avant tout nouveau lot : nommer les worktrees obsolètes,
libérer le disque (le portable d'abord, il n'a qu'un téraoctet), pousser et
fusionner tout ce qui est à moitié fini (« tout ce qui est sans bloquant réel
doit être mergé pour qu'on reparte tous à jour »).

## Le ménage (fait avant midi)

Inventaire : 24 worktrees ici dont 14 fusionnés, 46 à luciepc dont 32 fusionnés ;
~/.cache/rag3weaver-build à 129 Gio ici et 338 à luciepc (un target par
worktree au lieu d'un par session) ; les notes de codeparsers à 36 et 93 Gio
(mesures du 18 octobre) ; 21 Gio de copies reflink pour l'enquête LOCKS ;
un `target/` d'août de 208 Gio dans l'arbre principal de luciepc ;
lucivy/target 94 Gio et lucivy-review 40 Gio à luciepc.

Rendu par session (portable / luciepc, Gio) : mémoire 0,5 / 21 ; recherche
20 / 7 ; banc 9 / 2,5 ; G 13 / 41 ; everything declarative 14 / 19 ; hotfixs
1 réel sur 19 apparents (reflink) / 22 ; arbre principal 9 / 32 ; codeparsers
32 / 74 ; lucivy – / 58 ; embarquement 2 / –. Le portable passe de 300 à
357 Gio libres, luciepc de 462 à ~600. Ce qui reste de gros sur le portable
est à Lucie (Steam 183, modèles 135, baloo 25, sairen 23) ou vivant (48 Gio de
targets de sessions, effacés en fin de session).

Vérifications faites par chaque session avant d'effacer : `git cherry` contre
master (le patch, pas seulement l'ancêtre), arbre propre, `lsof`, diffs non
commités archivés (banc, recherche, arbre). Deux croisements : le banc a
effacé rag3db-mes-a/b/c de hotfixs à luciepc (diffs archivés) ; recherche a
retiré wt-l4 de l'arbre (sauvé sur arbre-principal/ancien-l4).

Refus de permission à relever pour Lucie : la suppression de branches
distantes atterries (`git push --delete`) est refusée aux sessions comme à
l'orchestration (« Git Destructive ») ; G est refusé même sur `git worktree
remove`. Rien n'a été contourné. Décisions laissées à Lucie : les branches
atterries d'origin (critères : tête ancêtre de master, ou cherry sans +, ou
remplacée par une -N), les worktrees de G, rag3db-service à luciepc, le
`target/` de 208 Gio dans son arbre de luciepc. Les données Magic de luciepc
(hors-arbre-18-oct, archive-mtga-18-oct) restent telles quelles, sur son mot.

## Les atterrissages (en cours)

Ordre de la file : cœur (projection-lecture-seule-3, puis le retrait) →
mémoire (M4 pas 2 lot-96-m4-5, pas 3) → recherche (creux surlignage-creux-4)
→ codeparsers (INCLUDES includes-3) → everything declarative (vues 5, 6, 7,
3b, fondues plus récentes que la table du 6) → G (lucivy-publiee-1) →
rag3weaver archi (anthropic-llm-2 : connect_llm rend Arc<dyn Llm>, feature au
jeu large). Chacun rebase sur le master du moment, rejoue selon
docs/procedures/push-d-un-lot.md, pousse sous if VERT. Puis la lib unique et
la bascule 43 (arbre, G).

## Le bloquant du jour

RollbackUnderReaders.AReaderSurvivesTheRollbackOfAnotherWriter : un lecteur
compte 924 ou 2058 relations au lieu de 10 pendant un COPY annulé par un autre
écrivain — des lignes non validées vues. Rouge 4/60 sur master a91f066ea,
0/40 sur 04063ee06 (avant V2) : régression des lots V2 (V2-1b, V2-1c, V2-2a,
V2-2b). Le cœur bissecte à luciepc (chaque commit moteur, 40 rejeux), lit la
cause dans le balayage CSR, ouvre le ticket. Le retrait des générations et la
lib unique attendent le correctif ; la projection peut atterrir si son seul
rouge est celui-là, nommé.

## Les hooks de Claude Code

Page de conception sur master (81b9b9176, rag3weaver archi) : PreCompact
écrit les consignes du résumé, PostCompact rend le résumé (L1 `compaction`),
SessionStart source=compact rend ce qu'il faut garder, UserPromptSubmit rend
recall + diffs des fichiers touchés + dernier verdict, Stop pousse une fin de
run (POST /journal/run) ; une commande mince `rag3weaver-backend hook`, jamais
bloquante ; le contexte réel lu dans `message.usage` de la transcription
(Claude Code compacte à ~96 % du million, mesuré sur 257 compactions), seuil
à 70 %, une fois. Code après l'atterrissage d'anthropic-llm, quatre lots.

## Graines du jour

Laya IA (classifieur ModernBERT multilingue, Apache 2.0, CPU) et SemIf/OpenJev
(la probabilité des options lue dans les logits d'un LM, MIT) : écrire le
`decider` par logits contre tout serveur local qui rend les logprobs ; mesure
commune sur les décisions de l'archiviste contre Gemini. Après le week-end.

## La semaine du moteur (décidée l'après-midi)

Lucie : le moteur continue en parallèle du chantier des vues par scope. Dette
au 7 novembre : 84 tickets ouverts pour 178 fermés, une quarantaine du moteur
en trois familles (la correction, une quinzaine ; le coût, une quinzaine ; les
lots dessinés V2-2c, V2-3, plafond L6). Règle : la famille correction d'abord,
un ticket par jour, témoin rouge d'abord, pas une ligne de perf avant zéro ; le
banc des vingt clients et ses treize rouges connus comme tableau de bord ; une
page d'état par session. Listes : au cœur les transactions (validation à moitié
appliquée, annulation sous tampon plein, COPY forcé hors du graphe, durabilité
sur faute d'E/S, takeover, échecs de rejeu non lus, puis V2-2c et V2-3) ; à
hotfixs les index et le CSR ((B), relevé des 18 tables, mesures de (A), FTS
hors du point de reprise, rétention des index, rtree, COPY de relations sans
remappage, doublon, MATCH par clé, CALL de réparation).

## L'après-midi (écrit à 16 h)

- **Atterris** : anthropic-llm-2 (90df16ace, connect_llm rend Arc<dyn Llm>), les
  trois lots des hooks de Claude Code (2d3ecad1e, d42787227, 8f60895b8), poste à
  N détenteurs (e006f4cff ; lourds-max 2 puis 4 à luciepc à 15 h 35, 1 ici),
  INCLUDES (9fd37ab1c, 299/300 au juge strict), les lignes de nouveautés de
  novembre. En tour : le pas 2 de la mémoire (deux courses perdues → le tour
  relit master juste avant le push, règle écrite), whois (arbre), le creux
  (recherche), les vues 5, 6, 7, 3b (everything declarative), lucivy-publiee
  (G), chercher-sans-dossier (hooks-essai, vert sur sa base). Fenêtre de push au
  cœur 16 h 10 – 16 h 40 pour le bloc annulé (b484a61ef), puis le retrait, puis
  la lib unique et la bascule 43.
- **La session d'essai des hooks** : lancée par Lucie à 14 h 50 dans
  ~/git_workspaces/rag3db-hooks-essai (clé `hooks-essai`) ; premier tour réel :
  fiche d'identité et handle écrits, trois défauts trouvés (genre `handle`
  absent de la mémoire → les quatre genres créés par 96 à 14 h 55 ; author/via
  nuls → la clé passée ; source acceptée hors forme → ticket) ; premier lot réel
  « chercher sans dossier » vert sur ide-react-vue3b-2 (SearchBox sur l'accueil,
  l'arbre et la recherche, le dossier en filtre scope effaçable, témoin qui
  prouve que le filtre change les résultats). Exigence de Lucie : corriger les
  hooks à la volée sans relancer Claude → settings ne porte qu'une commande
  stable, tout le reste (binaire, hooks.json, gabarits sous templates/hooks/)
  est relu à chaque appel ; `hook --rejouer` hors session ; versions au
  journal. Graines : les hooks et les prompts de conduite redéclarables par
  propositions et visibles dans l'IDE ; Laya IA et SemIf (le `decider` par
  logits sur un serveur local).
- **La base de code de Lucie est propre** : 0 écart sur les 18 tables de liens
  après douze jours de montre (releve-7-nov). Le témoin (B) de hotfixs est vert
  dans quatre cas sur master et sur 58b46a9bf ; la lecture sale du bloc annulé
  (corrigée par le cœur, 60/60) n'est pas la source des demi-arêtes ; dernier
  ingrédient essayé : le COPY de relations refusé faute de tampon ; s'il reste
  vert, (B) se ferme avec deux garde-fous versés au dépôt (le témoin à cinq
  cas, `tools/check_rel_directions` dans la procédure de bascule).
- **La vision** a gagné la cinquième et la sixième difficulté (cohérence
  temporelle entre couches, artefacts dérivés à état, génération épinglée,
  témoin cruel ; configuration de bâti), `successor_of`, et les graines des
  résumés de scopes et de RagForge.
- **Le CI de master** était rouge à 14 h 58 sur le compte de l'index des tickets
  (43 pour 44 lignes), remis par INCLUDES (46) ; les runs suivants s'annulent
  entre eux à la cadence des pushes, seul le dernier compte.
- **Toujours à Lucie** : les branches atterries d'origin, les worktrees de G,
  rag3db-service, le `target/` de 208 Gio dans son arbre de luciepc. Les
  sessions sont passées sur Opus 5.5 à 15 h 45 (l'orchestration reste sur
  Fable 5.1) : les briefs nomment fichiers, commandes et témoins.
