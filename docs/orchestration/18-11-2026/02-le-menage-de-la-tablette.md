# Le ménage de la tablette (18 novembre 2026)

Lucie, 12:25 : « mon disque sur la tablette est presque plein, est-ce qu'ici aussi on a des worktrees ou targets
morts à supprimer ? » Relevé : 838 Go sur 950 (104 libres) ; 303 worktrees de rag3db (140 Go apparents, 61 dont la
branche est déjà dans master), ~137 Go de `target-*`, 105 Go sous `~/.cache/rag3db-locks`, 48 Go `1b-copie`, et
des copies sous `~/.cache/rag3weaver-build`. Les tailles sont celles de `du` sur btrfs : des reflinks partagent
leurs blocs, la place rendue se mesure après coup (arbre a rendu 0,1 Go en effaçant 18 Go apparents de copies).

Chaque session a listé, sans rien effacer, ce qui est mort dans ce qui est à elle (worktree propre dont la branche
est dans master ou poussée à l'identique sur origin ; target d'un lot atterri ; copie de mesure finie dont la
preuve est posée ; journaux), après avoir vérifié qu'aucun processus, hôte ni passe ne le tient. Les branches
distantes restent toutes : seuls les dossiers partent. **Rien ne s'efface avant le mot de Lucie** ; puis chaque
session efface le sien par chemins littéraux (`git worktree remove` pour les worktrees) et mesure la place rendue.

## Ce qui est mort, par session

| session | quoi | chemin | taille apparente |
|---|---|---|---|
| c7 | essais du 8 nov. (base sparse, morts) | `~/.cache/rag3db-moteur-notes/essais-c7` | 11 Go |
| arbre | 4 worktrees atterris (vues-2b-prime-7, migration-generations-5, vues-en-anglais-3, stack-error-2) | `~/.cache/rag3weaver-build/wt-{vues,migr-gen,anglais,stack}` | 1,9 Go |
| arbre | 15 worktrees de Codex, inutiles, branches sur origin | `~/git_workspaces/rag3db-cx-arbre-*` | 6,9 Go |
| arbre | targets | `~/.cache/rag3weaver-build/target-vues`, `target-vues-garde` | 12 + 5,7 Go |
| arbre | copies des vérifications de 1b (preuve dupliquée à luciepc aux mêmes md5) | `~/.cache/rag3weaver-build/1b-copie/{requetes,verif-20261114,avant-20261114,2026-11-14-depuis-avant-835d5298f,migree-20261114-1641}` | 48 Go |
| embarquement | dossiers de journaux morts (7–15 nov.), lsof vide | `~/.cache/rag3weaver-build/{embarquements,demo-embarqueur,groupe-pile,groupe,docs-groupe,repli-pile,repli,conception-repli,rapport-emb,dense-tenue,lot-embarqueur*.log}` | 25 Mo |
| banc | 11 worktrees (branches sur origin comme archives ; rag3db-begin atterrie) | `~/.cache/rag3weaver-build/wt-banc-parite`, `~/git_workspaces/{cx-banc-embedding-rate,cx-banc-oracle-migration,cx-banc-corpus-public-20261116,cx-banc-lucivy-indexed-ro-20261116,cx-banc-placement-explicite,cx-banc-residence-appels,cx-banc-residence-nocturne,cx-banc-suivi-k10,rag3db-begin}` | 4,5 Go |
| banc | gel de source de Codex et miroir des dépôts publics (se refait par les pins) ; mesures d'octobre et des 12–13 nov. posées dans leurs pages | `~/.cache/rag3db-banc-notes/{source-freeze-16nov,public-corpus-20261116,mesure-regle-*,regle-*,liste-*,compose,p1,joignabilite-export,k10-*,publication-*,contexte-rocm,residence-nuit-12nov}` | 5,7 + 3,2 + 1 Go |
| codeparsers | 4 worktrees (3 atterris, 1 remplacé par la repose 4) | `~/.cache/rag3weaver-build/wt-{homonyms,retimport,new,stopwords}` | 1,9 Go |
| codeparsers | 21 worktrees de Codex (branches sur origin) | `~/git_workspaces/cx-codeparsers-*` | 9,7 Go |
| codeparsers | sonde de la première pose | `~/.cache/rag3weaver-build/stopwords` | 28 Mo |
| mémoire | targets (sans provenance ; bâti sur 373b5477b) | `~/.cache/rag3weaver-build/target-96`, `target-96-compaction` | 19 + 7,8 Go |
| mémoire | 7 worktrees (4 atterris vérifiés par git cherry ; 3 de Codex en docs seules, branches sur origin) | `~/.cache/rag3weaver-build/cx-wt-{96-query,memoire-compaction,96-push-compose,memoire-script-fixtures-r4,memoire-passation-weekend,memoire-passation-claude-17nov-r1,memoire-script-fixtures-cleanup-r1}` | 3,2 Go |
| mémoire | instantanés de la mémoire de Lucie du 14 nov., dépassés par M8 | `~/.local/share/rag3weaver/instantanes/{avant-archivage-14-nov,avant-chantiers-14-nov,avant-chantiers-14-nov-2,ecrit-pendant-la-fenetre-14-nov}` (+ .sha256) | 0,6 Go |
| archi | 9 worktrees (8 dans master ; wt-nav poussé à l'identique) | `~/.cache/rag3weaver-build/wt-{branchement,equipier-fin,livrable,nom,pas,pont,relai,socle,nav}` | 4,2 Go |
| archi | 19 worktrees de Codex (poussés à l'identique) ; preuves de Codex sauf cx-archi-pas-va-au-bout-r2-59ac-pass1-local (le rouge admis, à garder jusqu'au push) ; journaux de passes closes | `~/.cache/rag3weaver-build/cx-wt-archi-*`, `cx-archi-*`, `cx-revue-archi-*`, journaux bout-*, pont-*, nom-*… | 8,8 + 0,1 Go |
| cœur | 28 entrées d'octobre et deux du 13 nov. (lots atterris) | `~/.cache/rag3db-moteur-notes/{etape-4,bases-fautives,deux-sens,borne-voisins,a3,a5,a5bis,copie,garde2,copy-refuse,livraison-3-octobre,plantage-reouverture,…,insertion-defaite,etendue}` | 2,4 Go |
| cœur (Codex) | dossiers du 16 nov. de la session Codex du cœur, preuves archivées dans les branches cx-coeur-* ; **core-ro-01/02 = la matière du refus de la plateforme : Lucie tranche** | `~/.cache/rag3db-moteur-notes/{rows-outside-graph-red-01,-02,dictionary-global-01,dictionary-append-fix-01,two-reds-01,reference-strings-01,c1-offsets-02,core-ro-01,core-ro-02}` | 2 Go |
| ED | 14 worktrees posés sur master (git cherry 0) + 7 dépassés ou sondes (poussés) + la mesure navigateur du 13 nov. (4 lignes d'instrumentation non commitées, jamais faites pour master) | `~/.cache/rag3weaver-build/wt-{agent-nom,agent-renommer,batch-grain,chunker-ties,ecrivain-essai,gabarits,llm-absent,models-llm,react5,short-lots,socle,sync-guarantees,tie-identity,chunker,ties,ecrivain-18,ecrivain-pile,mesure-3,sonde-aretes,sonde-offsets}`, `~/git_workspaces/rag3db-{equipier,ed-navigateur}`, `~/.cache/rag3weaver-build/ed-navigateur` | 10,5 + 0,9 Go |
| ED (Codex) | preuves et ménage du harnais de Codex ; le produit (ties, Vertex) est posé ; **Codex disait « aucun rouge effacé » : Lucie tranche** | `~/.cache/rag3weaver-build/{cx-ed-vertex-cleanup-contract-v1,cx-ed-cleanup-6309-exec-local-v1,cx-preuves-ed-*,cx-ed-ties-cleanup-archives-20261116.tar,cx-wt-ed-ci-writer-diagnostic,cx-ed-*.py/.json/.tar}` | 19 Go |
| tty | 5 worktrees (cx-tty-14, -15 atterris ; eval-codex ; deux de Codex poussés) ; targets morts | `~/git_workspaces/{cx-tty-14,cx-tty-15,cx-tty-eval-codex,cx-tty-english-aliases,cx-tty-english-aliases-union-99}`, `~/.cache/rag3weaver-build/{target-cx-tty-6,target-cx-tty-english-aliases-2,target-cx-tty-wt/cx-tty-14,target-cx-tty-wt/cx-tty-eval-codex}` | 6,7 Go (+ 21 Go `target-cx-tty-6-features` après tty-21) |
| recherche | 6 worktrees atterris ; target du 14 nov. ; deux instantanés reflink des comptes d'orphelins (écrits au ticket fermé) ; journal | `~/git_workspaces/rag3db-{filtre,orphelins,vues,exclu,attente,docs11}`, `~/.cache/rag3weaver-build/target-97o`, `~/.local/share/rag3weaver/instantanes/recherche-compte-orphelins-{1-20261115-1400,2-20261115-1500}`, `~/.cache/rag3weaver-build/recherche-orphelins` | 2,8 + 3,6 + 9,4 Go |
| hotfixs | copies de mesure finies (bases de Lucie = données de test), 8 et 14–15 nov. ; targets ; 10 worktrees de Codex (branches identiques à origin) | `~/.cache/rag3db-locks/{retrait-ancien-index,sonde-orphelins,releve-ancre-portable,coherence-tables,sonde-coherence,purge-orphelins,fenetre-purge,sonde-lucivy,rejeu-dadbf0041,rejeu-17a094d12,depot-9h05,sans-wal-8h45,demo-nodejs,memoire-13h40,trace-etude,releve-templates,purge-tmp,liens-sous-pdr,gabarits,ouvreur,ancre-*}`, `~/.cache/rag3weaver-build/target-hotfixs-{lucivy,gabarits}`, `~/git_workspaces/cx-hotfixs-*` | 105 + 17,6 + 4,6 Go |

| G | targets à lui (tête dans master, rien d'écrit depuis le 17) ; 26 worktrees (16 tête dans master, 11 piles reposées depuis, branches poussées) ; 18 libs du magasin que rien ne tient ; 97 binaires `.avant-*` sous `~/.local/bin` (6,9 Go) dont on garde le dernier retour `.avant-927941a6-59ac` et ceux d'avant 373b5477b | `~/.cache/rag3weaver-build/target-{rust-G,fils-G}`, `~/git_workspaces/rag3db-{p1-hash,p2-knownred,p3-confineur,p4-gabarits,r1-chemin,r2-plafond,r3-lib,required-suites,tests-only,temoin-voisins,garde-sm,plafond-menage,plafond-G,plafond-tag,pile-G,confineur,garde-hash,known-red,ticket-gabarits,ticket-gabarits-2,passe-chemin,poste-plafond,temoin-lib,q1-chemin,q2-plafond,q3-lib}`, `~/.cache/rag3db-libs/{0c0af5bf5,2822e314d,3cca0ae1e,3ebaadb5a,41d282843,4f9274a38,52101b7fd,65cb9a1f3,690029ab7,6ffec5a82,7cd11e5fa,8b98f19f7,a4505f0b3,c42fc98bf,c44af8031,dcbaef7b1,e93710f36,f6179ade0}`, `~/.local/bin/*.avant-*` sauf le dernier retour | 9,3 + 11,5 + 0,5 + ~6 Go |
| G (relevé pour les autres) | `vrai-c` 14 Go (wt-vrai, tête hors master, rien d'écrit depuis le 17) et `I-luciepc` 13 Go (preuves du 11 au 13) : à leurs auteurs (recherche ? cœur ?), pas encore réclamés | `~/.cache/rag3weaver-build/{vrai-c,I-luciepc}` | 27 Go |

| lucivy | 4 worktrees lucivy (PR fusionnées ou main), le worktree du pointeur atterri, la copie de mesure du dictionnaire, les deux crates posés par Codex, la pile FS de Codex (deux worktrees, 14 dossiers de preuves) | `~/git_workspaces/lucivy-{bm25-vue,cmp-ids,marge,version}`, `~/git_workspaces/rag3db-lucivy-439`, `~/.cache/rag3weaver-build/{mesure-dico-source,cx-tty-crate-ld-lucivy-439,cx-tty-crate-lucivy-core-439,cx-lucivy-admission-proof-archive-*,cx-lucivy-fs-*}`, `~/git_workspaces/cx-lucivy-{corpus-admission,fs-admission}` | 1,1 Go |
| lucivy (pas à elle) | branches de docs de Codex sur origin (`cx-lucivy-plan-shutdown` 476 Mo, `cx-lucivy-shutdown-destruction` 99 Mo) : rétention de la passation, Lucie tranche | `~/git_workspaces/cx-lucivy-{plan-shutdown,shutdown-destruction}` | 0,6 Go |

Les treize listes sont là.

## Ce qui reste vivant (ne pas toucher)

Les worktrees et targets des lots en cours : arbre wt-sondes/target-sondes (A1), ED wt-method-kind (2.6),
codeparsers wt-stopwords4 + `target-import-pointe-pile` (27 Go, meurt à l'atterrissage de (5)), tty cx-tty-16 +
`target-cx-tty-6-features` (21 Go, meurt à l'atterrissage de tty-21), archi wt-delegate + `delais-c` (26 Go, son
target de travail) + pont-fork, banc wt-banc-etalonnage + target-banc-etalonnage, recherche rag3db-fts-guard, cœur
rag3db-csr, hotfixs rag3db-gabarits ; les worktrees de docs de chaque session ; les instantanés de G de la bascule
(avant-59ac9c86b-20261116-927) ; l'instantané M8 de la mémoire (avant-m8-15-nov) ; `~/.cache/codeparsers-notes`
(preuves des tickets) ; les copies de parité du banc et le jeu figé (à luciepc) ; les binaires de Lucie
(rag3weaver-tty, rag3weaver-tty-essai).

## Deux choses que Lucie tranche à part

- Les dossiers de la session Codex du cœur `core-ro-01` et `core-ro-02` (67 Mo chacun) : la matière du refus de la
  plateforme (lecture d'un core de SIGSEGV).
- Les 19 Go de preuves et de ménage du harnais de Codex pour ED : le produit est posé, les manifestes sont dans les
  branches, mais Codex avait écrit « aucun rouge effacé ».

## Total apparent

Environ 350 Go apparents effaçables tout de suite (dont 128 chez hotfixs, 75 chez arbre, 31 chez mémoire, 27 chez G, 16 chez
recherche), plus 21 Go pour ED à trancher, plus 27 Go (codeparsers) et 21 Go (tty) après l'atterrissage de leurs
lots. La place réelle sera moindre.
