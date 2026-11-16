# Référence chaînes verte une fois, sans attribution causale

GO exact C7 de la référence75 seule, pilotea202a1032a0eb83c672cdb4ccad8d54cf6a1ff19/recette2ccafae5af5714774c5500dd02d15f87655f44f34aece8264ce0b8538530ab7b. **1cas passé sur1, aucune inférence de cause du crash candidat.** Une exécution de StringRewriteAcrossAddColumn.AutomaticCheckpointsBetweenAddsAndDrops, sans sonde. Aucun candidat, lecteur, GDB ou autre cas exécuté dans cette passe.

## Profil et résultat acquis

Source75a4ba4d467aa5f07ef83494f5393154ab2392c6, moteur59ac9c86bc2b3529b590a7462ab7051e3ad26b8a du header/ELF privés. Source propre et derniers chemins build_id, deux tests et dictionnaire vérifiés ; compilation1030/1030 code0, cinq artefacts exportés avant le cas. Release/rcOFF/stOFF/ASANOFF/TSANOFF/UBSANOFF/LTOOFF/extensions vides, aucune lib59 posée.

Vrai poste parent run-p694702-i151600763.scope, départ monotonic1197918,210189401 ; cpu.max500000/100000, POSTE_FILS5/j5/CPU500%,32Gio/swap0/scratch32G/réseau absent. Même MAX_NUM_THREADS2/OMP5/OPENBLAS5 que le véritable argv du crash50e2. RAG3DB_STRING_DIAGNOSTIC **absent** dans env.json et le second env-i de argv.json, aucune valeur0 ou sonde active. Test/helpers concurrence identiques aux sources du candidat ; chemins privés et ordonnancement restent différents.

XML :1test/0failure/0error/0skip/0disabled, GTest922ms ; processus0 en0,965178s, pas de timeout, attente/reap effectués, aucun résidu. **BUILD_ID stdout non émis et non inventé** ; provenance limitée explicitement au contrôle source/header/ELF/hash/chemin d'exécution.

Fin entrée225,250341s sur900 : unité inactive/dead/Result=success/MainPID0/ControlGroup vide, quiescent=true, aucun PID attribué restant, premier refus null. Artefacts rehashés après quiescence. memory.events : oom0/oom_kill0/oom_group_kill0/max0 ; memory.peak4102991872octets, swap.peak0. Ces compteurs couvrent **toute l'unité build+cas**, pas le seul test et pas le candidat précédent. Poste rendu, aucune réservation, GO clos.

## Contraste conservé

| Exécution | Source / moteur | Sonde effective | Observation |
|---|---|---|---|
| Candidat historique50e2 | ed0 /888 | absente par défaut de pilote | wrapper139, SIGSEGV natif confirmé par core, aucun XML complet ; pile dictionnaire/write/checkpoint acquise ensuite |
| Référence uniquea202 |75a /59ac | absente explicitement | code0, XML1/1passé, aucune erreur de valeurs |

Les deux fichiers test sont identiques et le seul delta des chemins moteur est dictionary_chunk.cpp. Cette mesure unique **n'attribue pas le SIGSEGV au delta append** : une course ou autre dépendance à l'ordonnancement reste possible, et les deux exécutions ne sont pas simultanées/bit-à-bit identiques. Elle ne montre pas que wrongRows228 et le SIGSEGV partagent une origine. Aucun candidat supplémentaire, boucle de répétition ou debug build implicite pour transformer cette observation en causalité.

## Preuves et suites

[Lecture et empreinte d'archive](preuves/reference-strings-01/lecture.json), archive recus-reference-et-xml.tar.gz : recette, commandes effectives, environnement, XML, classification, limites/métriques et reçus hash/quiescence. Paquet complet avec ELF/archive/cache/compile_commands aux deux postes ;53 fichiers bruts et5 artefacts rehashés localement conformes. Les gros ELF/archive/compile_commands restent dans les paquets privés, leur manifeste/empreintes figurent dans l'archive du dépôt.

ParentC1, ticket du SIGSEGV, ajout refusé, wrongRows228 et plan lecteur restent ouverts. Le défaut de drapeau du pilote dispose de son correctif CPU, aucune validation native instrumentée acquise par cette référence sans drapeau ; ticket drapeau toujours ouvert. Aucun known_red changé, qualification globale ou intégration master/lib. Suite admissible sans nouveau runtime : relire le delta append et les durées de vie/alias des chaînes sur le trajet StringChunkData::write/checkpoint ; toute expérience supplémentaire demande recette/RO/GO distincts.
