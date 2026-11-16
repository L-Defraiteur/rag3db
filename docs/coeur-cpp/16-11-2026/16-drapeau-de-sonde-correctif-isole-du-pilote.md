# Drapeau de sonde : correctif isolé du pilote, aucun rejeu

La passe50e2 est close : SIGSEGV natif du premier cas, lecteur et référence non joués. Preuves et deux tickets publiés **avant correction** sous a693b7373a sur cx-coeur-dictionary-append-atomic, document15. Le ticket concerné est docs/coeur-cpp/tickets/2026-11-16-le-confinement-efface-le-drapeau-de-la-sonde-de-chaines.md ; le crash a un ticket distinct. Aucun lien causal prouvé entre le défaut du pilote et le signal.

## Contre-exemple et correction

Le témoin CPU extrait le vrai constructeur confined et la vraie boucle des cas du contrôleur, puis observe le vecteur de commande donné au faux lanceur. Avant fix : environnement externe avec RAG3DB_STRING_DIAGNOSTIC=1, mais argument absent du second env-i ; assertion effectivement rouge (1joué/1échec). Aucun bwrap, build ou processus C++ exécuté par ce témoin.

confined reçoit désormais explicitement environment=env, le même dictionnaire que le reçu env.json et que le lanceur externe. Un appel de test sans environnement explicite est refusé. Aucun héritage implicite de l'environnement hôte ; le témoin injecte un réglage hôte synthétique et vérifie l'égalité exacte des champs internes avec le dictionnaire explicite. Le dictionnaire de base n'est pas muté.

Après fix : **12 contrôles CPU passés**, dont format XML natif, identités, premier rouge STOP, refus avant unité, rehash et petit Python expiré. [Preuve et empreintes](preuves/two-reds-preparation/environment-fix-cpu.json). Aucune modification de source C++, donnée, assertion native, SDK, statut known_red ou logique de verdict pour139.

## État et suite

Aucune recopie sur PC, staging50e2 et reçus historiques intacts. Les chemins/unité du lancement terminé restent fermés ; la recette révisée porte explicitement NON ADMIS et ne constitue pas un pin prêt à relancer. Tout futur runtime demandera une recette et des chemins frais, une RO et un GO distincts. Le ticket du drapeau reste ouvert en attendant une validation opérationnelle admise, et C1 reste non attribué.

La lecture des seuls métadonnées/stack existants n'identifie pas le frame ELF+0x1023a43. Proposition séparée, **non exécutée** : une analyse du core existant avec l'ELF archivé exact, GDB en mode batch sans init/auto-load/debuginfod/réseau, commandes seulement info files, info sharedlibrary et backtraces bornées ; aucune exécution, appel de fonction, service ou base. Budget proposé120s sous vrai poste mémoire4Gio/swap0, core/ELF RO, logs privés. Le pilote exact d'analyse et la vérification des bibliothèques devront être publiés avant une admission ; aucune autorisation n'est déduite de cette proposition. Aucun rebuild/replay pour obtenir une pile.
