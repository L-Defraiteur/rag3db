# La commande de décompression du core utilise une option refusée

- **État** : corrigé (16 novembre2026), commande qualifiée sous admission distincte core RO02
- **Famille** : moteur
- **Gravité** : diagnostic empêché par son pilote, aucune garantie moteur jugée
- **Atteignable en service** : non, commande privée d'analyse de core
- **Touche rag3weaver** : indirectement, diagnostic du moteur
- **Signalé par** : cœur C++, recette core RO120s
- **Pour** : cœur C++, coordination C7 et hotfixs

## Refus acquis avant correctif

[Bilan et reçus](../../coeur-cpp/16-11-2026/19-lecture-du-core-refusee-avant-gdb.md). Pilote7ec, commande zstd1.5.7-4 épinglée4f8335318c5471e1b481bbb9eebdcc9cd9a31b0bcfb5e2393857d656f33160e8 : --decompress --sparse --no-progress --output=<scratch>/core <core.system.zst>. Code1 sans timeout ; stderr dit Incorrect parameter: --output=... et montre -o OUTPUT. La préparation CPU avait contrôlé GDB et la supervision, pas ce vecteur réel de décompression.

Arrêt au premier écart, poste rendu après0,151556s, aucun GDB/inférieur lancé. Core et ELF originaux intacts, aucune connaissance nouvelle sur le SIGSEGV. Ne pas qualifier ce refus de crash moteur ou de problème de données.

## Suite et fin

Ticket avant fix. Témoin CPU appelant la vraie construction de commande contre la forme documentée dans le stderr conservé ; voir l'échec avant remplacement de --output= par -o et son argument distinct. Vérifier ordre des arguments, entrée, fichier privé, option sparse et absence de --rm/force/écrasement. Revue et recette avec chemins frais avant toute admission distincte ; aucun retry automatique. Fermer après commande effectivement qualifiée sous portée admise, sans fermer le ticket du SIGSEGV ou C1.

## Fermeture limitée au pilote

Microfix37f2, témoinASTpubliéavantCPUd16e2a4ba1 puis1FAILavantfix/9CPUvertsaprès ; GOdistinct C7-CORE-RO-37F-16nov120, zstd0 en1,366267s, core décompressé de taille/empreinte exacte, GDB0 et quiescence5,511915s. [Bilan et reçus](../../coeur-cpp/16-11-2026/21-core-lu-pile-du-checkpoint-dictionnaire-conservee.md). La forme de commande est qualifiée, sans fermeture du crash/C1 ni validation d’un test moteur.
