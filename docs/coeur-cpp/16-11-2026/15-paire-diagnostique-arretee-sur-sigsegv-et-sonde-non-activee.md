# Paire diagnostique arrêtée : SIGSEGV natif et sonde non activée

**ROUGE, résultat natif incomplet, aucune comparaison référence/candidat.** GO distinct C7-CORE-PAIR-50E2-16nov2400 consigné82e0b2f64e ; pilote50e2f6bfe0d04cf787977170a465d924fbfef469, candidat ed0c3c8af57f25e83ae5c4ba004ae3393e77812e/moteur888, référence75a4ba4d467aa5f07ef83494f5393154ab2392c6/moteur59. Recette688ae35b772f315eb8c64252486eaa418dc5486569becf19d1aecd565187c70b inchangée pendant la passe. Aucun réessai ou correctif en cours d'exécution.

## Entrée, refus et sortie

Vrai poste lourd, parent run-p4075024-i150852227.scope, cpu.max500000/100000, POSTE_FILS5, donc j5/CPU500%,32Gio/swap0. Unité cx-coeur-two-reds-01.service PID4075028. Départ monotonic1193341,317579432, plafond2400s comprenant arrêt/finalisation.

Compilation candidat1030/1030 code0. ELF, archive moteur, header, CMakeCache et compile_commands exportés avant le cas ; BUILD_ID888 du header retrouvé dans l'ELF. **Aucun BUILD_ID réellement émis dans stdout du cas**, aucune attestation runtime acquise. Aucune lib/SDK externe posé ou chargé par cette recette.

Premier cas StringRewriteAcrossAddColumn.AutomaticCheckpointsBetweenAddsAndDrops : wrapper code139, timeout=false, durée10,133810s, attente/reap faits, aucun résidu de groupe/cgroup. Stdout contient seulement le lancement GTest et RUN ; XML de résultat absent. Le journal coredump confirme le natif PID4141127, signal11/SEGV_MAPERR, user unit exacte. Stack déjà produite par le journal : #0 std::_Hash_bytes, #1 ELF+0x1023a43. **Pas de GDB, pas de pile symbolisée supplémentaire, pas d'attribution au crash C1 ni au correctif append.**

Ledger : candidat chaînes REFUSED, candidat lecteur NON JOUE, référence chaînes NON JOUE, référence lecteur NON JOUE. Aucun build référence. Ce refus est un crash, pas un rouge GTest complet.

Fin entrée251,851634s : unité failed, Result=exit-code, ExecMainCode=1/ExecMainStatus=1 (contrôleur), MainPID0/ControlGroup vide, quiescent=true, aucun PID attribué restant. MemoryPeak4359299072octets, MemorySwapPeak0. memory.events et maps runtime non capturés, cgroup disparu ; ne pas inférer une absence d'OOM par défaut. Aucun service, base de Lucie, modèle, GPU, master ou lib posé touché. Poste rendu ; GO clos.

## Second constat : défaut propre au pilote

env.json déclare RAG3DB_STRING_DIAGNOSTIC=1 dans l'environnement passé au wrapper. Mais argv.json montre le second /usr/bin/env -i reconstruit par confined(testing=True) depuis test_env global, **sans ce drapeau**. Le vrai constructeur publié établit la frontière fautive : la sonde chaînes n'a pas été activée. L'absence de logs de sonde ne localise donc pas le crash avant load/avant instrumentation. [Ticket du transport du drapeau](../tickets/2026-11-16-le-confinement-efface-le-drapeau-de-la-sonde-de-chaines.md), ouvert avant tout correctif. Les journaux restent intacts comme contre-exemple.

[Ticket du SIGSEGV observé](../tickets/2026-11-16-le-cas-de-chaines-isole-se-termine-sur-sigsegv-sans-resultat.md), distinct du défaut d'environnement. Le lien avec le précédent wrongRows228 est inconnu. Aucun défaut d'écriture de la sonde prouvé ; aucune donnée détaillée instrumentée acquise.

## Conservation et limites

[Lecture et empreintes](preuves/two-reds-01/lecture.json), archive recus-et-core-info.tar.gz : admission, limites, ledger, sorties/refus, commande réelle contre environnement déclaré, manifeste/header et métadonnées du core. Paquet complet aux deux postes,47 fichiers bruts et5 artefacts rehashés localement conformes ; rehash distant après quiescence également présent.

Le core système existant compressé est conservé aux deux postes : post-run-core/core.system.zst,2387501octets, SHA77c13010a68f6ce77aabb1db849b087eb1bd8f682503490b8bd3fa568122d104. Le core brut extrait reste sur PC :17548656640octets creux, SHA15a21f6deeacb0c8c1400acd4196f230189971b7bd325703c0091d6f14d34e2f. Une copie locale brute partielle de1168605184octets a été interrompue puis supprimée après conservation et vérification du compressé ; jamais présentée comme core complet. Le fichier système original reste intact. Les métadonnées ajoutées après arrêt sont séparées de raw et n'en réécrivent pas l'inventaire clos.

Suite autorisée : lecture des sources/journaux existants, contre-exemple CPU du vrai constructeur avant correction isolée de l'environnement. Tout GDB, nouveau test natif ou référence requiert une recette exacte et une admission distincte. Les deux tickets de la première passe, append et C1 restent ouverts ; aucun known_red modifié ni résultat rouge transformé en vert.
