# Passation week-end — architecture, 13 novembre 2026

Arrêt demandé par Lucie pour limite d’usage. Cette passation sauvegarde l’état ; elle ne donne aucun GO de calcul, runtime, intégration ou nettoyage. Les anciennes échéances monotoniques sont périmées pour une reprise ultérieure : NE PAS les réactiver, ni remplacer discrètement leurs constantes. Nouvelle décision C7 et nouveau couple exact requis avant toute suite. Aucun push master dans cette passation.

## Livré et sauvegardé

Navigation symbolique : code intégré sur master `3edbb8475ace557b87316da6a269938055d23f9d`, source qualifiée d006. Navigation fichier/parents/enfants/membres par contrats existants, bornée ; containment par lignes, pas promesse AST/héritage. Grep enrichi intégré `d3f5a264d7fb7f77775644f549ff77267e6517d6`, bilan `f15394e7979db6f2fc164ff508a306c2879f6046`, source qualifiée `6786c03b5f9d667e1df567b710a6ce52a7b452f7`. Profondeur 0 préserve le rendu ; contexte positif qualifié source/fraîcheur, ambiguïtés explicites. PreparedNavigation mutualisée une fois par fichier. Pas de pose du démon personnel.

Ces lots et leurs nettoyages avaient été achevés avant la veille de revue : aucun WIP Rust personnel restant à pousser. Les W/targets personnels symboles/grep ont été retirés avec garde et archives fidèles sur les deux postes ; branches distantes et preuves conservées. Le petit lot documentaire RO pilotes read_scope `f66105e8e` est également poussé, son W retiré ; section reprise par ED, pas de nouveau code caché.

La session a ensuite réalisé des revues statiques et des réempreintes de preuves pour C7/ED/Recherche/Arbre. Les avis sont attachés à leurs SHA, pas transposables automatiquement à un nouveau pilote. Aucun Cargo, rustc, navigateur ou service n’a été lancé par cette session pendant ces revues.

## État en cours à reprendre avec les porteurs

### Garde Recherche

Produit qualifié APP `9ec421069320b2f0cca4099fa27f83059e944de9`, pilote `65ad0810ca3f4f1496539d5e2945e5c959b94a07`, preuves `cdcc032dfde186f9b33fb8d9804ca50fcf2c1a4b`. Revue locale : 558 fichiers intégralement réhashés, manifeste `cece3588719044bf461eb6f7f322eb8f9b0db524e2b63e979eb07f782522cb3d`. F8 8/8, lib entière 2308/0/66, NAV40/0/0 ; deux corps HTTP d’un seul juge conformes aux blobs Git, 223 gabarits par ELF. PID/start même inferior, SDK exec/main, detach TracerPid0 puis waitpid réel recoupés. Budget original respecté, rouges antérieurs conservés. Cela ne réparait pas la CI physique.

Candidat `8eefcd65687d8ae999620ed745385aa705432cdb`, branche `cx-recherche-garde-target-integration-r1`, revu favorablement : 6783 entrées hors docs identiques APP9ec, entrants master documentaires préservés, 33 patches dont 32 égaux et un conflit documentaire de ticket. Landing à confirmer depuis master courant et rapport du porteur ; des messages ultérieurs indiquent garde intégrée sur les bases utilisées par Arbre. Ne pas rejouer l’ancien candidat ni ses délais.

Ticket garde déplacé dans `docs/tickets/closed/2026-11-11-un-target-repris-embarque-les-gabarits-d-un-autre-worktree.md`. Cargo brut externe reste hors frontière des scripts gardés ; aucun partage concurrent autorisé par le marqueur.

### Couches 2b-A, CPU qualifié puis union en arrêt diagnostique

CPU A : APP `affa6d03b5a57dd99810b6853783dc1a267f76b9`, pilote `c75ad3ae4b437495fb4544045894a986ac23bebb`, preuves `33a64da41c0d68a4fd0653c71167fc8887e41f7d`. 315 fichiers réhashés localement, manifeste `59bf3e14da0adfc99f1788ff431a6ae4d707602e32f860bb51b654019682d6ba`. Ledger `22e3ccb9af4c9711db3491d545486a41b231bff0cbb96ae23b78dfda909b1be2`, ELF `c71b48d315952845b272f0ab47118a9078516c0ffe5635ae078c0ba6a2d1e69d`. 15 exacts + lib entière1662/0/6 ; 1662 fonctions distinctes, 1677 invocations, publication25 incluse. Portée admission CPU/fixtures/spies seulement, ni chargeur durable/snapshot composé B ni outils C.

Union initiale `193d77cd71fa4cc1608e961ea4c8a8d3d445412e`, pilote `0d5d96ffe4f925b1f5878a9a957e2eb7dffc89b3` : RO favorable avant exécution, puis ROUGE F8 dead_code hors cfg(test), aucun runtime qualifié. Preuves `e6e6b8677742c6b10efde5cbb28b27bdf19bb540`, rapport36, 48 fichiers conservés. Correctif source `d9aa4be86c6f21665cf85c22e47134c87ab6aab8` : huit cfg(test) ciblés sur champs/méthodes fixture ; corps/juges et garanties prod conservés. RO source favorable ; ne pas confondre avec compilation réussie.

Pilote correctif `9625c810c347e6b288b21cc7d7e4e5621967522f` retiré AVANT exécution (assert APP193 résiduel). Corrigé en `d7d00dcca62d8ae1b3d2b8bb21b4fe88ec338123`, puis run rouge v2 : preuves `237614f8343a196f384d2b01361a8f83e7a0b490`, rapport39, 59 fichiers manifeste `fd4e8bb02f79559458341a178e4c7ae740bc992d24a7e95abf773465cc572aab`. Reprise193→d9 verte, trois jeux F8 verts, quatrième Cargo profil sélectionné None ; cause polling/exec/env NON ATTRIBUÉE. Aucun ELF/juge runtime/NAV union qualifié ; rouge non requalifiable par observation après arrêt.

DERNIER COUPLE RELU : APPd9aa ci-dessus / PILOTE `4fca0dd38378d5bb31269dab76713d6bc0060ebc`, branche `cx-arbre-couches-2b-union-observe-pilote-r1`. RO statique favorable après correction du pilote776 retiré. Capture owned par pidfd/SIGSTOP, thread set arrêté stable, identité relue, profil sélectionné archivé avant assertion, SIGCONT même fd en finally ; aucun signal réel émis par notre revue. Politique de présence explicite : SDK sept champs présents ; CPU six absents et LD rustlib présent ; valeurs strictes, rawlen non nul. Erreurs capture enveloppées RuntimeError avec cause pour éviter leur absorption par handlers OSError du poller ; premier rouge immuable, cleanup attribution seule. 24 fonctions CPU hermétiques annoncées par le porteur, NON REJOUÉES ici. Nouveau run de ce couple NON OBSERVÉ par cette session, aucun résultat inventé. Demander au porteur/C7 l’état réel au moment de reprise.

Ancienne fenêtre Arbre INITIAL918138.385510088 / END922938.385510088 : historique UNIQUEMENT. Aucun GO de week-end ne découle de cette constante. T était marqué WTcfgd9 au dernier message ; ELF de preuve conservé reste affa CPU, jamais un ELF193 inventé.

## Branches, worktrees et postes

Racine cache portable et luciepc : `/home/lucied/.cache/rag3weaver-build` (R). Origin : `git@github.com:L-Defraiteur/rag3db.git`. Les branches de porteurs sont annoncées publiées ; ne jamais les éditer à leur place. État portable constaté pendant passation :

- `cx-wt-couches-2b-admission` / `cx-arbre-couches-2b-admission-r1` → affa6 complet ci-dessus.
- `cx-wt-couches-2b-pilote` / `cx-arbre-couches-2b-pilote-r1` → c75 complet ci-dessus.
- `cx-wt-couches-2b-preuves-v1` / `cx-arbre-couches-2b-preuves-v1` →33a complet ci-dessus.
- `cx-wt-couches-2b-union` / `cx-arbre-couches-2b-union-r1` →193 complet ci-dessus.
- `cx-wt-couches-2b-union-cfg` / `cx-arbre-couches-2b-union-cfg-r1` →d9aa complet ci-dessus.
- `cx-wt-couches-2b-union-cfg-pilote` / `cx-arbre-couches-2b-union-cfg-pilote-r1` →d7d complet ci-dessus.
- `cx-wt-couches-2b-union-observe-pilote` / `cx-arbre-couches-2b-union-observe-pilote-r1` →4fca complet ci-dessus.
- `cx-wt-couches-2b-union-pilote` →0d5 complet ; W preuves-v1→e6e, preuves-v2→237614 complets ci-dessus.

PC : mêmes chemins de source annoncés propres/préparés Git seulement par Arbre, à réattester par son porteur. Aucune inspection distante nouvelle ni certification de HEAD PC par cette passation. Ressources Arbre `R/cx-target-contenu-derive`, preuves/contrôleurs/confinements `cx-*-couches-2b-*-v1/v2/v3` conservés ; ne pas déduire que tous existent. Aucun contrôle de ces ressources à déclencher automatiquement.

Notre seul nouveau W est `R/cx-wt-archi-passation-week-end`, branche `cx-archi-passation-week-end-20261113`, rapport présent. Il reste conservé après push. Base origin/master locale constatée `3a5080d93bdaeba05fd333b25bdd0565b49ad18b` ; master de l’arbre Lucie `41497ba4eacb9ba23445b8de91d27f5a90006fc8`, avec sous-module modifié et `extension/rag3weaver/docs/lucie-tmp/` non suivi : INTACTS, pas notre WIP, ne pas ajouter/commiter/nettoyer.

## Tickets et preuves à retrouver

Tickets ouverts : `docs/arbre-principal/tickets/2026-11-13-appliquer-la-visibilite-des-couches-avant-les-requetes-des-vues.md`, parent `2026-11-12-migrer-les-ancres-de-code-vers-le-contenu-partage.md`, CI `docs/orchestration/tickets/2026-11-13-la-ci-refuse-la-reouverture-du-temoin-physique.md`. Navigation parent : `docs/rag3weaver-archi/tickets/2026-11-12-exposer-la-navigation-dans-les-symboles.md` (vérifier déplacement éventuel depuis master courant). 2a pur livré/clôturé ; 2b B/C demeurent ouverts.

Copies locales réhashées par cette session : `R/cx-preuves-garde-target-union-v2-final/{client,pilote,superviseur}`, `R/cx-preuves-couches-2b-a-v1-local/{client,pilote,superviseur}` ; Recherche ciblé `R/cx-preuves-garde-target-v2-local` et v1-local. Preuves grep historiques : `cx-grep-symboles-6786c03b5-globale-20261113` (rouge brut), `cx-grep-symboles-6786c03b5-complement-20261113` (composé vert, trois vrais ELF archivés), dossiers de nettoyage `cx-grep-symboles-nettoyage-{pc,portable}-20261113`. Conservés deux postes lors de leur clôture ; pas réaudit distant ce soir.

Grep global acquis : F5 5/5+minimal0, lib2278/0/66, NAV40, hôte13, PG36 ; lectures27 distinctes jugées composées26+CREATE1, 28invocations brutes. Flag moteur ancien1 imposé par âge forçait CREATE nonjugé ; complément exact mêmeELF/profil sauf seulflag retiré, 1/0/0. Pas validation format207701 déduite du SDK29. Dump PG privé sauvegardé puis base propre supprimée à l’époque, aucune DB personnelle touchée.

## Ressources, verrous et règles de reprise

Aucun calcul, service, GPU ou verrou poste possédé par cette session de revue/passation. Derniers messages porteurs : rouges arrêtés, autorités terminées, bus/cgroups absents ; une mesure Lucivy avait priorité avant tout nouveau lourd. Ce n’est pas une attestation universelle actuelle : C7 doit demander les statuts aux propriétaires avant réserver une place. Ne pas tuer/processer/nettoyer par nom ni toucher leurs W/T.

Reprise de target UNIQUEMENT garde commune avec provenance connue, scan frais après poste et contrôleurs exacts ; aucune purge Python/fingerprint manuelle/adoption target inconnu. Supervision commune obligatoire,32GiB/HIGHmax,j8, quatre rôles CPU, bus privé ; `.env`/`.vault` jamais lus/copîés. Aucun SDK/modèle/service reconstruit sans périmètre autorisé. Premier rouge STOP, diagnostic avant nouveau SHA/GO, pas retry/skip/sommeil pour peindre vert. SDK29 vert n’innocente ni ne répare la CI physique.

## Trois prochains pas, commandes RO et limites

1. C7 rassemble les passations et constate les têtes sans exécuter de pilote. Depuis ce W : `git fetch origin`, puis `git ls-remote origin refs/heads/cx-arbre-couches-2b-union-observe-pilote-r1 refs/heads/cx-arbre-couches-2b-union-cfg-r1`. Relire le rapport40 du SHA4fca et les éventuels nouveaux messages ; demander au porteur si un run a démarré depuis ce document.
2. Relire la chaîne rouge/preuves et conserver son diagnostic : `git show 237614f8343a196f384d2b01361a8f83e7a0b490:docs/arbre-principal/13-11-2026/39-arret-union-cfg-observation-cargo.md` (retrouver le nom exact avec `git ls-tree -r --name-only 237614f8343a196f384d2b01361a8f83e7a0b490 docs/arbre-principal/13-11-2026` si déplacé). `git show 4fca0dd38378d5bb31269dab76713d6bc0060ebc:docs/arbre-principal/13-11-2026/couches-2b-union-observe/cargo-stop.py`. Aucun appel au lancer.
3. Après arbitrage Lucie/C7 seulement : rédiger une NOUVELLE recette concrète et un nouveau couple pour fenêtre autorisée, ressources conservées réattestées, source cfg inchangée si justifié ; relire, puis réserver après mesures prioritaires. Ne pas lancer4fca avec l’ancienne END. Ni master ni retrait ne sont autorisés par cette passation ; revue de candidat exact et tour C7 distincts après preuves suffisantes.
