# Écart de configuration spdlog — relecture sans nouveau lancement

Porteur : cœur C++, [même ticket d’annulation](../tickets/2026-11-09-sous-vingt-ecrivains-un-pool-plein-laisse-une-annulation-inachevee.md).
Le créneau C++ est terminé ; recherche FTS a la priorité. Aucune configuration,
compilation, mesure, gate ou modification de source moteur pendant cette relecture.

## Comparaison complète des unités moteur

Preuves distantes : `annulation/cx-archive-privee-12nov-01/revue.json`,
`resume-refus.json`, `dependances-spdlog.json`, `limites-resultat.txt`.

| Contrôle | Résultat |
|---|---|
| Unités source compilables sous `src/` | mêmes 739 chemins de chaque côté |
| Définition `-D` différente | 0 unité |
| Compilateur | GNU 16.2.1, `/usr/bin/c++`, pointeur 8 octets, ABI ELF identiques |
| Optimisation/langage/visibilité | mêmes arguments après normalisation des racines |
| Écart restant sur les 739 commandes | référence seule : `-I<SOURCE>/third_party/spdlog` |
| Objets du journal Ninja lus | 739 demandés, 739 présents, 739 VALID |
| Dépendances effectivement consignées | 267 507 lignes, **0 chemin sous third_party/spdlog/** |

Les 741 objets physiques inventoriés ne sont pas un décompte des unités actuelles
comparées ; aucune unité source n’a été ajoutée ou retirée entre ces deux plans.
La lecture de dépendances utilise `ninja -t deps`, pas un compilateur. Le journal
est celui des objets de référence existants : cela ne produit pas d’objet privé.

Les macros communes sont `ANTLR4CPP_STATIC`, `HAS_FDATASYNC`,
`RAG3DB_CMAKE_VERSION="0.11.2.2"`, `RAG3DB_EXPORTS`,
`RAG3DB_EXTENSION_VERSION="0.11.1"`, `RAG3DB_ROOT_DIRECTORY` (racine normalisée),
`__64BIT__`, `NDEBUG`. Aucun champ ni option ABI accepté par exception.

## Origine et portée du chemin

Dans `CMakeLists.txt:409–413`, les configurations BUILD_TESTS ou
BUILD_EXTENSION_TESTS ajoutent ce chemin ; sinon BUILD_BENCHMARK peut aussi
l’ajouter. Le privé désactive les trois. Le répertoire contient uniquement
`LICENSE` et le sous-répertoire `spdlog/` : aucun en-tête à plat susceptible de
remplacer directement un `common.h` d’un autre composant.

Une recherche textuelle dans `src/` ne trouve aucune mention de spdlog. Le diff
C++ `65486c02d..0511dece7` n’ajoute pas de directive include ou préprocesseur.
Ces constats et les dépendances Ninja étayent l’**innocuité de l’absence de ce
chemin pour les unités moteur de référence examinées**, et expliquent pourquoi
le refus ne prouve pas une régression du correctif. Ils ne constituent pas une
comparaison de deux binaires privés (aucun nouveau binaire n’existe), ni une
preuve du différentiel mémoire DDL, ni un motif de désarmer la garde.

## Proposition historique d’option — rejetée par la revue, ne pas appliquer

Activer les tests pour récupérer indirectement un include est exclu. Activer
BUILD_BENCHMARK aurait aussi cet effet, mais configurerait un autre travail :
ce n’est pas le mécanisme souhaité. Injecter un chemin dans CMAKE_CXX_FLAGS
changerait sa position de recherche et ne reproduirait pas littéralement les
commandes ; ne pas masquer cette différence dans le comparateur.

L’option dédiée proposée est `RAG3DB_INCLUDE_SPDLOG_HEADERS=ON`, défaut OFF,
qui ajoute ce seul chemin **au même endroit** que la référence, sans activer
les cibles de test ou de benchmark. [Diff proposé](outils/cx-proposition-spdlog.patch),
non appliqué à un worktree ni au dépôt moteur. Aucun source C++ modifié.
Après éventuelle revue, garder BUILD_TESTS, BUILD_EXTENSION_TESTS et
BUILD_BENCHMARK à OFF, et demander uniquement la cible statique rag3db.

**Contrainte à arbitrer avant toute nouvelle configuration** : ajouter cette
option modifie CMakeLists.txt. Un build ainsi configuré ne serait plus un arbre
`0511dece7` strictement propre ; il faudrait nommer explicitement sa révision
ou son patch de configuration, conserver le diff, les sources C++ identiques
et le manifeste. Ne pas le présenter comme l’archive initiale retrouvée ou un
build clean inchangé de 0511. L’autorisation précédente ne couvre pas tacitement
cette modification. Si la contrainte de source clean exacte est maintenue sans
cette variante, la proposition ne s’applique pas et le build reste arrêté.

## Critères historiques de la proposition — décision effective ci-dessous

- Comparer les mêmes 739 unités, toutes leurs définitions, options et chemins
  ordonnés ; zéro différence après la seule normalisation des racines.
- Compiler/version/ABI et tailles p12/v11/g17/s18 inchangés, aucune tolérance
  générique ajoutée au comparateur.
- Plan sec limité à rag3db statique et ses dépendances ; aucun objet test,
  benchmark ou extension, aucun téléchargement.
- Nouveau lancement et identité de source explicitement revus après le slot FTS ;
  mêmes limites mémoire/scratch/durée, pas de réemploi d’objets de base.
- Sauvegarde archive/en-têtes/dépendances/manifeste, avant tout pilote ou mesure.

La préparation est terminée pour revue. Le ticket reste ouvert et aucune relance
n’est programmée automatiquement.

## Décision de revue : exception exacte, aucun changement CMake

L’orchestration refuse l’option dédiée ; le diff proposé reste une trace rejetée,
non appliquée. Source `0511dece74f94db6937fb03e6ec45f0c58828c5a` propre maintenue.
La seule différence acceptée explicitement est l’absence de l’unique argument
`-I<SOURCE>/third_party/spdlog`. Ce n’est ni une identité littérale de commandes
ni la récupération de l’archive historique.

Identité de configuration privée à consigner :
`cx-0511-tests-off-without-unused-spdlog-path-v1`. Le manifeste doit inclure
`historical_archive=false`, `literal_commands_equal=false`, les unités qui
emploient l’exception et la raison de revue. Aucune exception pour un autre -I,
-D, option de compilation, compilateur, ABI, unité absente ou nouvelle.

La règle préparée vérifie que le tableau privé est **exactement** le tableau
référence après retrait d’une seule occurrence de ce token. Une occurrence
double est refusée ; l’ordre et toutes les autres valeurs restent vérifiés.
Les mêmes 739 chemins sont exigés. La gate de revue du plan reste obligatoire.

### Audit conditionnel réalisé en lecture seule

`annulation/cx-archive-privee-12nov-01/audit-conditionnels-spdlog.json` :
3 955 fichiers lus, dont 3 326 projet et 629 système ; 0 fichier absent,
0 mention de spdlog. Le périmètre réunit les dépendances réelles de référence
et les sources C/C++/en-têtes du worktree privé du correctif. Empreinte du jeu
fichiers/contenus :
`b1b905c3a2efb2baa7cda16eed2e9e9a480213af8e09a93a1719d9a24f5e7363`.

21 lignes `__has_include`/`__has_include_next` relues avec contexte, y compris
les appels sur plusieurs lignes : version, filesystem, endian/byteswap, atomic,
en-têtes Linux, numeric_traits, sanitizer/tsan, stdint, single_threaded et PSTL/TBB.
Aucun test conditionnel spdlog ; le seul cas générique est la macro de repli
`__has_include(x) 0` dans re2. La racine du chemin omis ne contient que LICENSE
et le sous-dossier spdlog, pas ces en-têtes.

### Contrôles préparés pour un futur lancement revu

Le lanceur révisé réauditera les dépendances de référence et les sources avant
gate ; après un éventuel build, il exigera aussi 739 objets privés VALID et
0 dépendance ni mention conditionnelle spdlog avant de sauvegarder le manifeste.
Toute dépendance absente/périmée, mention spdlog ou autre différence arrête.
Le manifeste nomme la configuration distincte et conserve les audits.

Vérification pure de la règle, sans lancer le moteur : le fichier de comparaison
du premier préflight reproduit l’exception pour les 739 commandes ; 12
contre-exemples sont refusés (macro, compilateur, optimisation, autre include,
ordre, doublon, etc.), ainsi qu’une macro ajoutée sur une commande réelle.
Outil : `outils/cx-verifier-exception-spdlog.py`. Aucun compilateur, lanceur,
pilote ou benchmark exécuté par cette vérification.

Le préflight révisé est à partager ; **aucune nouvelle configuration ou build**
n’est autorisée par la seule fin du créneau FTS. Pas de relance automatique.
