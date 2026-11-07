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
