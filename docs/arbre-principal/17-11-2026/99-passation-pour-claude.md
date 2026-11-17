# Passation autonome arbre-principal vers Claude — 17 novembre 2026

**STOP prioritaire demandé par Lucie pour quota, relayé par C7.** Les travaux
sont arrêtés. Tous les coupons non consommés sont RÉVOQUÉS. Aucun ancien END,
GO13nov/SDK29, GO Qv9, A, CPU12 ou proposition900 ne se réactive à la reprise.
Ne pas reprendre aux rappels automatiques. Seules sauvegarde/push WIP et
passation ont été faites après STOP : aucun test, native, DB, provider, service,
nettoyage, pose, tag ou master. Une nouvelle consigne explicite est nécessaire.

Le **17 novembre prime** les longs messages de revue historiques du fil.
Le message 4fca/d9/END922938 concerne le13nov et ne concerne PAS ce chantier.

## État actuel en une lecture

**A acquis : build privé CLI99 + paire statique + framing seulement.**
**B HOLD : deux défauts source du NOUVEAU Reader, cinq témoins écrits NON JOUÉS,
correctif NON APPLIQUÉ. C trois bases NON AUTORISÉ/non fait.**

Dernier WIP de travail publié :
**3240c0a4693aecabf45ddafbeadd5547752766b2**, branche
`cx-arbre-cli99-runtime-b-20261117-r1`, égale origin avant le commit de cette
passation. Cette branche reçoit ensuite uniquement les présents documents.
Ne pas interpréter ce HEAD documentaire comme un nouvel APP/pilote qualifié.
Le SHA du commit de passation est celui transmis à C7 ; le retrouver par
`git log -1 --format='%H %s'` dans ce même worktree, sans lancer de recette.

WIP ancien retrouvé puis sauvegardé SANS qualification au STOP :
**fdf62741ba9363f8d360e7258de420d672646182**, branche
`cx-arbre-bascule-bases-20261116-r1`, push terminé/origin égal/propre.
Quatre anciens fichiers non suivis : return-proof-gate.py,
test-return-proof-gate.py, read-typed.cpp et typed_rows.py sous16nov.
Seul typed_rows est identique à la source courante ; les trois autres ne le
sont PAS. C'est une sauvegarde WIP, **pas** une version à substituer aux
primitives qualifiées. Aucun fichier perdu ni WIP laissé hors index connu.

## Où reprendre et quoi lire

Worktree LOCAL courant :
`/home/lucied/git_workspaces/rag3db-cx-arbre-cli99-runtime-b-20261117-r1`.
Lire d'abord docs/README.md, règles orchestration11nov, procédure push, puis
[recette15](15-preparation-runtime-cli99-sur-copie.md), surtout son RETRAIT,
[témoins cinq frontières](runtime-99/test_reader_boundaries.py) et
[pins rouges](runtime-99/red-boundary-pins.json).
Ne jamais commencer par lancer lifecycle.py.

Tickets TOUJOURS OUVERTS, aucun déplacement/close autorisé au STOP :

- [Parent trois bases](../tickets/2026-11-16-basculer-les-trois-bases-vers-les-generations.md),
  porte préparation B, réserves R1/R2 et cinq pré-témoins avant correction.
- [Cohorte469](../tickets/2026-11-16-la-migration-de-scopes-laisse-469-morceaux-sans-vecteur-transfere.md),
  diagnostic attribué/copie traitée, aucune bascule originale acquise.
- [Contrat postwriter](../tickets/2026-11-17-la-garde-readonly-est-utilisee-apres-le-writer-du-cycle.md),
  source/19CPU/Qv9 acquis bornés ; parent/cycle restent ouverts.
- [Armement du producteur](../../orchestration/tickets/2026-11-17-le-cycle-demarre-avant-l-armement-effectif-du-producteur-de-grant.md),
  contrôleur/HB qualifiés sur Qv9, aucune extension aux originaux.

C7 décisions actuelles : suffisance A
**9a771e58012490db46fa99bc7dd4c2dfca3fc9bd** ; HOLD B
**d10a34f11960a5413687fcd544ed50b2834f9a15** ; STOP Lucie reçu ensuite.
Contact coordination C7 thread01a1cb5c-62b9-7ea0-8b7e-cf38398ece09.
Arbre thread01a1cb5d-2191-7112-82be-648e153fe4cd.

## A livré et suffisamment prouvé, portée étroite

Preuves **90aa9d053d583eab07a7b299d9ee3dba6dd85c50**,
branche `cx-arbre-cli99-static-a-preuves-20261117-r1` =origin, propre.
[Rapport14](14-preuves-cli99-statique-a.md). Deux RO Recherche/ED
**1bb4cc301d0c00bbed610400db73d7eef6cb659c** + C7 rehash personnel deux hôtes.
GO **C7-CLI99-STATIC-A-17nov2400**, C78b54d1bbac, consommé UNE fois rc0.

APP compilée **99fef92ce6ee9a0007eaf99c7c3a6a3789061c57**, pilote
**4f17c4cc00ede65756c7b8f27e98bdc0d7ba1cca**, Q prepared
**f88671ab5b27d3d6723dfbdc7e1bbee08734077def4d4911a07954a1b4ff6eb5**.
Parseur privé **bd133850f89d6b12a3ca2b2f6ce238785ed37e3a**.
Build réel release/offline/locked, deux bins, jobs8, cinq features JSON exactes
code/daemon/default/openai-llm/rag3db-native ; deux artefacts uniques.
Cargo rc0/Finished2m50,28warnings conservés, aucun cargo fix.

| Copie privée A | SHA256 |
| --- | --- |
| backend nouveau99 | 687de930ed499b2ced472af6e8ccba17b2b4837b6c282d60bdf5f3d01f958f78 |
| checker nouveau99 | c587d700ee4cf08d45c87e21742898c0962c365604acf7f7cea3aaa460298a86 |
| typed-reader PC historique | add01058f3b0d6efd28d936c865b1ba9071c0e2eb85f559743bf0c52aeea6371 |
| IDs Rust Lucivy4.3.9 historique | d0dd13e6d403e614038b251f413637d5f6d02a98ba1c10509388d0be1f5e15c7 |
| query PC historique NON invoqué | 48ae2328a29676fa69154cb9c67d9125d4d04cf92b029bd2b2a09981dea95616 |

Ledger cinq originaux/copies fsync AVANT invocation SHA
**02b6117e36374de7d1b0bf8eb98192b8ad637a991c7df6d638aa2b18a455da31**.
Seulement checker statique rc0 (backend FILE ARGUMENT, NON exécuté), puis
typed --self-test rc0/golden R3TYP002/NUL/newline/FLOAT1.25/vide.
Driver BUILT_AND_STATIC_PAIR_CHECKED_ONLY, composé BUILD_STATIC_PAIR_OBSERVED.
Common **validation-059qix5h**, archives TERMINE/VERTint0/nettoyé/restes[].
END1287682.655859896 clos, poste-entry1285352.498234195 ; constat opérateur
1285591.291963132, pas un elapsed exact de fin. Bus absent constaté à la fin
du lot ; pas de nouvelle observation processus globale au STOP.

Profils CONFIGURED_NOT_PROCESS_OBSERVED, **aucune maps/env/ABI runtime99,
backend/DB/provider ni lecteurDB acquis**. F8/lib/NAV produit exact d'ED sont
carried (rapport42/6add025610), sans rejeu A ni conversion de l'ancien composé
observateur ROUGE en VERT. Aucun ancien binaire927 renommé99.

## B préparé puis retiré : faits, réserves et ce qui n'a PAS été joué

Source pilote **4e3fc8402eb308a9b9693fd68f88593e826c3e88** ; publication metadata
**7a78310d6db34c86cb708b5a2b7df5a5a9665ad0** ; APP99 inchangée.
Q prepared **1d1441034dc2f811239bfd9688a63ebc3244cf65173890080b04171205f5737d**,
29sources/JSON+prepared30e, modes0600/UID1000/Q0700, copies Git/inventaire dans
runtime-99/{prepared-pc,preparation-pc}.json. **Metadata seulement** : prepare
n'importe pas l'ancien driver ; aucun test/GDB/ELF/DB/provider lancé.

Adaptateur runtime-99/lifecycle.py réemploie22primitives E9 byte-identiques,
ancien lifecycle sous alias, chemin/PID/nonce/profil/policy/postwriter/graph/
FTS/visibilité/gens/bits/dette stricts. FUTURE copie privée neuve de FIN froide
qualifiée/pinnée ; ELF privésA réutilisés, query/portable/C exclus. Proposition
900s/Qv9, NON accordée, GDB coût inconnu borné query90. Nouveau Reader : première
vraie query seule EXEC→MAIN sous GDB, mapslib59/env/start/CG du même inferior,
événement exited0, framing réel ; pas detachTracer0/waitpid horsGDB ni maps
des autres queries/vector chargé après main. Ce code est NON qualifié.

RO ED initiale favorable bornée
**d667f001ca912c4b5cfcbdf9d4bdb9646550c886** (rapport47) est HISTORIQUE.
Recherche puis C7 établissent deux réserves groupées :

1. R1 : first_query prend q.sha(reader.binary) courant comme SHA attendu.
   Il faut une référence indépendante provenant du ledger A/add010 ; les
   guards avant/final ne ferment pas une substitution ABA à cette frontière.
2. R2 : reader_exec vérifie END avant hash/open, puis exec sans contrôle frais ;
   first_query calcule durée/deadline avec deux now et n'a pas de recheck frais
   juste avant Popen. Hash/open/publication peuvent consommer le temps restant.

**Aucun fix appliqué.** Les vrais reader_runtime.py/reader_exec.py/adapter
sont inchangés depuis4e3, anciens Q/WT figés conservés, zéro consommation B.

Douze tests initiaux écrits NON JOUÉS (8validator+4vrai first_query mocks).
Cinq pré-témoins supplémentaires publiés **3240c0a4693aecabf45ddafbeadd5547752766b2**,
NON JOUÉS : promotion SHA, END pendant hash, END pendant open, deux now186>185,
END juste avant Popen (refus tardif ne doit pas innocenter un départ).
Ils appellent les vrais corps ; proc/clock/subprocess/exec/dup2 fictifs, IO/hash
réels privés. Pins LOCAL Pythonc81d885…9517/timeoutccecc427…29c3 dans JSON.
Attendu cinq FAIL/0ERROR/0skip/rc1 SANSfailfast, douze autres non sélectionnés.
**17 fonctions écrites, ZÉRO jouée. Aucun GO CPU5/CPU12/B accordé**, tous coupons
non consommés révoqués au STOP. Ni5FAIL ni17PASS ne sont des faits acquis.

RO ciblée des cinq pré-témoins ED favorable PRÉPA seulement
**5508931ab0a8894b42090d92e45e8ca06eed0030**, rapport48 ; R ciblée attendue
à la coupure. Ses réserves totales R1/R2 restent pleinement valides.
Un message tardif b9e5 health/communicate concerne seulement un ancien lot13nov
déjà remplacé par transport E9 ; ne pas créer un nouvel incident sur E9.

## Preuves et ressources à conserver sur les deux hôtes

R = `/home/lucied/.cache/rag3weaver-build` ; D = R/bascule-live-20261116.
Jamais de copie/lecture du .env ou de .vault. Ne pas copier le jeton Common.

| Pièce immuable | PC | Copie locale | SHA manifeste |
| --- | --- | --- | --- |
| A34fichiers117387802B | R/qualification-cli99-static-a-proofs-01 | R/qualification-cli99-static-a-proofs-pc-local-01 | fab645b8020848f4a3119fb45ebddfca59189a1a74f83f8c060901c20515d0ee |
| Qv9 cycle444fichiers315611900B | D/lifecycle-v9-observed-01 | D/lifecycle-v9-observed-pc-local-01 | feb5ec30687b904b11583c938721cb19ff9b89fd4f97ab139be490beda009edf |
| Qv8 rouge103fichiers | D/lifecycle-v8-red-01 | D/lifecycle-v8-red-pc-local-01 | 497baaf330eaa519aa73c8e3488e812b8c8ba15da5de80227ecbb2020aea2d5d |
| FIN RO76fichiers | D/debt-fin-ro-proofs-01 | D/debt-fin-ro-proofs-pc-local-01 | 05661b1fddc82c1e66d21f218d0c8cdfd12b69d8ef16432a2c55c3b127a9462a |

Ces ensembles ont été réempreintés intégralement auparavant, avec RO, pas
rejoués au STOP. Ne pas imprimer les arrays97247digest ou les énormes listes
FTS ; comparer silencieusement taille/SHA et afficher uniquement les comptes.

A PC : R/qualification-cli99-{pc,control,scopes}-r1, cinqELF dans pc-r1/artifacts,
target unique **R/target-arbre-principal-qualif** présent et conservé. Q_SOURCE
qualification-cli99-source-review-r1 distinct et conservé. Aucun nettoyage.
B PC : D/cli99-runtime-b-control-r1 seul EXISTE ; P
D/cli99-runtime-b-copy-r1 et S D/cli99-runtime-b-scopes-r1 ABSENTS ;
Q/allocation.json et Q/outer.log ABSENTS, constat RO metadata au STOP.
Ne pas réutiliser ce Qr1 pour le futur fix ni lancer le vieux pilote.

SDK59 répertoire exact : R remplacé ici par
`/home/lucied/.cache/rag3db-libs/rag3db-build-59ac9c86bc2b3529b590a7462ab7051e3ad26b8a-Release-p12-v11-g17-s18-rcOFF-stOFF-asanOFF-tsanOFF-ubsanOFF`.
librag3db.so SHA b7e9fc75a3d2de04c72680b0309e8bb1df9eaadb5211ebbacf39716c6d0a2101,
vector1669d6392ec8067fcf92ae2ccc972fd7cd3086629fb82d1f5d254e90d72d7561.
Pas de lien courant, remplacement SDK ni nouvelle compilation.

## Worktrees, branches et WIP : relevé RO au STOP

[Inventaire LOCAL14](99-inventaire-local.json) et [PC13](99-inventaire-pc.json)
listent chemins/SHA/branches/status/origin_sha sans modifier les checkouts.
Le relevé local précède la sauvegarde des quatre fichiers : sa ligne bascule
est supersédée par fdf62741ba9363f8d360e7258de420d672646182 propre/origin égal.
La ligne de cette branche B précède le commit documentaire de passation.
Autres lignes locales : propres et branches origin égales au relevé.

PC13 propres. Les WTs PC sont volontairement GELÉS à leurs sources : certaines
branches locales n'ont pas d'alias distant homonyme, d'autres sont derrière
leur branche documentaire publiée. Ce n'est PAS du WIP non poussé : les commits
sont déjà conservés par les branches source/preuves. Ne pas avancer un WT
gelé pour rendre HEAD==origin, cela casserait la provenance de Q. En particulier :

| PC worktree sous /home/lucied/git_workspaces/ | SHA figé |
| --- | --- |
| rag3db-cx-arbre-qualif-cli99-20261117-r1 | 99fef92ce6ee9a0007eaf99c7c3a6a3789061c57 |
| rag3db-cx-arbre-cli99-pilot-control-20261117-r1 | 4f17c4cc00ede65756c7b8f27e98bdc0d7ba1cca |
| rag3db-cx-arbre-cli99-runtime-b-control-20261117-r1 | 4e3fc8402eb308a9b9693fd68f88593e826c3e88 |
| rag3db-cx-arbre-cycle-postwriter-20261117-r1 | e9f037be2aa25da2c7f6c76f79e2eb57b6c676c9 |

Sources/preuves LOCAL actives archivées : prépa02fecfa4905893531cfbf4fa859f10c599fd22b8
sur cx-arbre-prepa-trois-bases-20261117-r1, preuvesA90aa sur branche ci-dessus,
Qv9preuves6e97b2ff8a6a8f4e4ca095ce3033fce10ceb8549 et docbornée
965a2841706782d74aa7a4e8272445e20dfb6a56, branches homonymes de l'inventaire.
L'arbre de Lucie ~/git_workspaces/rag3db n'a été ni commité ni basculé. Son HEAD
LOCAL au relevé était927941a67a5805450d49f1e04b39fdd542a6059b ; cela ne change
pas l'APP99 compilée. Ses modifications antérieures ne sont pas du WIP Arbre.

## Historiques à ne pas repeindre en vert

Qv9 e9f observé/accepté C7b27e, preuves6e97+965 : UNE copie FIN, writer927
mêmePID/maps/ACKrc0/publication, PRE/POST/COLD/graph/FTS/vecteurs passés.
97247vecteurs768 finite, dette0, indexREADY, cohorte469 couverte, graph38,
FTS5 mêmes ensembles/gens/fichiers. Scope validation-qls5drhl archives TERMINE
VERT0/nettoyé/restes[] ; ancien END1269461.757652623 NON réactivable.
11UNKNOWNglobaux conservés, pas d'absenceGPU déduite, calculés/réutilisésUNKNOWN.
Pas de durabilité crash ni observation globale de zéro RPC/calcul.

Qv8 source6c6 refusé après shutdown : garde readonly inadaptée aux retraits
WAL/.ecrivain ; preuves65eb82372e4dec638f0e9557e2e83f44d9d6c1f6 ROUGES,
ses POST n'ont pas été joués. Correctif postwriter E9 a19CPU verts distincts,
pas un blanchiment Qv8. Archives de tous rouges/verts CPU restent sous R :
postwriter-causal-red-48a2-20261117-01, postwriter-cold-red-336c-20261117-01,
postwriter-green-e9f0-20261117-01 ; Common/HB/finalizer autres archives référencées
dans rapports03/04/05/08. Aucune raison de les rejouer sans nouvelle cause.

Qv4 watcher armé trop tard, preuves e65cbd34a582c3bbb82773bfd9fd6f9b8abff695,
et B5 grant arrivé après premier rouge2e38a947650d639b2b37aecd11f8514f427421ea
restent rouges sans writer. COPYA camelCase et auditEDD comparaison tuples/JSON
restent leurs rouges historiques. FIN5b7 observe la donnée sans reconstruire
après coup le shutdown ancien. Les13nov 2a/2b, SDK29 et END922938 ne sont pas
des coupons17nov et ne qualifient aucune bascule des trois bases.

## Processus, verrous, services et limites d'observation

Aucune exécution Arbre active ni verrou tenu annoncé à la coupure. Dernières
exécutions A et Qv9 déjà terminées, rc0/scope nettoyé/restes[] selon leurs
archives relues metadata au STOP. B et ses17tests jamais lancés ; aucun PID
B à arrêter. Aucun processus nouveau, signal, scan /proc ou slot au STOP ;
pas de garantie globale fraîche déduite de ces archives.

Services7970/originaux/snapshots/retours appartiennent à G. API avait été
reprise par G le16nov13:37, reçu d9396ba55a2e538770d75f36f8dd45308c2761e5ee53107298781e0fc61fd814 ;
ce n'est PAS une observation de son état actuel. Arbre n'a fait aucune CLI sur
original, aucune pose, restauration ou service actionné. Provider7878 n'est
pas interrogé ici. Embar confirme seulement profil ATTENDU historique
0ce4548bcc0ccbc44357f6c9f71cbd8301afce7dd9740bdeb5680178947a5e8c/PID3906704
start111727101 ; aucun bail frais, mismatches futurs doivent refuser.

Trois bases originales non migrées ici : PC code-principal/code-lourd et
portable code-rag3db-depot sous ~/.local/share/rag3weaver. Pas de DROP/reset/
purge/cache/target retrait. Les modèles/services d'autrui restent intouchés.
G future fenêtre/snapshot frais+RET72h et Banc corpusMatched restent nécessaires
à C, pas anticipés par A/B. Oracle BM25RAW vs RRF, vector cosine/maxabs et
pertinence/rangs/corpus doivent rester séparés (plan10/Bancdf02/9bcac).

## Trois prochains pas — INACTIFS, seulement après nouvelle consigne

1. Reprendre le cadrage STOP puis obtenir nouvelle consigne, lire RO Recherche
   des cinq témoins si arrivée après coup, ED550893 et HOLD C7d10. Vérifier Git/
   pins/cwd source3240 propre. **Aucun test maintenant.** Exemple RO inactif :
   `git -C /home/lucied/git_workspaces/rag3db-cx-arbre-cli99-runtime-b-20261117-r1 status --short`.
2. Seulement avec **NOUVEAU GO CPU5 exact** : une invocation LOCAL des cinq noms
   de red-boundary-pins.json, sans failfast, timeout TERM58/KILL2. Forme INACTIVE :
   `/usr/bin/timeout --signal=TERM --kill-after=2s 58s /usr/bin/python3.14 -B -m unittest -v CINQ_NOMS_EXACTS_DU_JSON`.
   Attendre5FAIL/0ERROR/0skip/rc1 pour les bonnes raisons ; tout autre compte/
   cause/import/délai STOP sans retry. Sauver RAW/source/pins/rc/temps/manifeste
   et réempreinter deux hôtes AVANT correction. Les douze autres restent non joués.
3. Seulement après ces rouges conservés : correction GROUPÉE R1/R2, SHA attendu
   indépendant ledgerA/add010 (jamais hash observé), UN now pour MINcap/END-15,
   contrôles frais après hash/open et juste avant Popen/exec. Adapter fixtures
   valides si besoin sans effacer contre-exemples ; source/pins/ROtotales,
   NOUVEAU GO verts17 distincts, puis nouveau W/Qmetadata B (garderQr1) et deux
   RO TOTALES avant un NOUVEAU GO B/créneau. A/F8/lib ne se rebâtissent/rejouent
   pas sans cause. C/G/RET72h/Matched restent une autre étape à autoriser.

Toutes ces commandes sont des indications de reprise, **PAS une autorisation**.
Après publication de cette passation : STOP, aucune reprise automatique.
