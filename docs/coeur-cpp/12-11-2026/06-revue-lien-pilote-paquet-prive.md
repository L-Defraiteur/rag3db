# Pilote de retrait — recette de lien assortie, à relire

Porteur cœur C++, [même ticket d’annulation](../tickets/2026-11-09-sous-vingt-ecrivains-un-pool-plein-laisse-une-annulation-inachevee.md).
Préparation seulement : aucune compilation, lien, staging d’espace pilote ou
mesure exécuté. Arbre a priorité pour son ciblé CLI ; aucune relance automatique.

## Entrées attestées, sans base

Paquet distant :
`~/.cache/rag3db-moteur-notes/annulation/cx-archive-privee-12nov-02/archive-assortie-0511dece7/`.
Manifeste au niveau parent, SHA256
`0049111df769914dbeea66fd0d2d07c6add58a670eb80e797589604d48c80df7`,
830 fichiers rehashés par le cœur puis par C7 sans écart.
Archive moteur SHA256
`3030fe674e33e2ea0b5b01ed7bdf1a919e538c02e5dfc0b68fff7b50e1d06758`.
Source `0511dece74f94db6937fb03e6ec45f0c58828c5a`, configuration privée
`cx-0511-tests-off-without-unused-spdlog-path-v1`, pas archive historique.

La [recette argv et empreintes](outils/cx-commandes-lien-revue.json) remplace
l’ancienne recette issue des tests. Elle conserve ready_to_compile=false.
Les flags sont désormais dérivés de la commande **effective privée** de
`src/storage/index/in_mem_hash_index.cpp`, dans compile_commands.json attesté,
SHA256 `9a00bdf2613d09370c5dbf845831e66863c15031c569d01a00fef370e0506e1b`.
Compilation d’une seule TU, le pilote ; aucune TU moteur.

- Définitions, -fPIC, langage, optimisation, visibilité, include forcé cstdint
  repris de cette commande privée ; aucune définition GTEST/HAVE_STDINT_H
  héritée du target de tests, aucun -fPIE, include de test ou spdlog.
- Le seul changement de TU est source/objet vers le pilote ; sorties -MD/-MF/-MT
  retirées. Les substitutions de racines/en-têtes sont documentées dans le JSON.
- Include moteur vers headers-source du paquet ; générés vers headers-generes.
  Les include tiers restent sur le worktree privé source exact cx-coeur-archive-0511,
  pas sur rag3db-colonnes. Aucun repli vers un en-tête ou objet de base.
- La macro ROOT_DIRECTORY conserve la racine source privée employée pour l’archive.

## Lien minimal, sans framework de tests

La cible moteur statique ne fournit pas une commande de lien d’exécutable.
La recette utilise le même driver /usr/bin/c++, pilote.o, l’archive attestée,
puis l’interface statique de la source exacte `0511:src/CMakeLists.txt`.
Aucun gtest/gmock, objet test_helper, rdynamic ou export-dynamic repris des tests.

Les 17 archives tierces pointent **toutes** sous le paquet attesté, avec taille
et SHA256 individuels dans dependency_archives du JSON :

- libantlr4_cypher.a
- libantlr4_runtime.a
- libbrotlidec.a
- libbrotlicommon.a
- libutf8proc.a
- libre2.a
- libfastpfor.a
- libparquet.a
- libsnappy.a
- libthrift.a
- libyyjson.a
- libzstd.a
- libminiz.a
- libmbedtls.a
- liblz4.a
- libroaring_bitmap.a
- libsimsimd.a

fast_float est une dépendance d’en-têtes du worktree source exact, pas une
archive manquante. dl et m sont des bibliothèques système de l’interface ;
Threads::Threads ne rajoute pas ici de flag ad hoc au contexte GNU/Linux
configuré avec pthread dans libc. Aucune recomposition d’objets/archives de base.
La recette reste à la revue du lien ; un lien réussi n’est pas encore revendiqué.

## Fidélité des chemins générés, préparation encore à faire

L’ancien chemin privé de build temporaire n’existe plus après le namespace.
Le include `-I<build>/src/include` utilise directement headers-generes. Pour
préserver aussi l’espace de noms de `-I<build>/src` (include/common/...),
la recette déclare une vue privée future generated-build-src/include contenant
**seulement les deux générés attestés**, avec leurs empreintes. Aucun répertoire
ni lien de vue n’a encore été créé, aucune copie exécutée pour le pilote.

Après revue seulement : espace pilote neuf, copier le source pilote de SHA
consigné dans le JSON et les deux générés dans la vue, vérifier leurs SHA et
les archives avant d’exécuter la moindre commande. Aucun include sur un build
temporaire disparu et aucune copie de moteur à compiler.

## Ce que la revue peut autoriser ensuite

Les argv préparés sont disponibles pour une compilation **1 TU** et un lien,
mais aucun lancement ne découle de ce document. Après revue et fin du créneau
CLI arbre, C7 doit autoriser cette étape et son lanceur borné. Le pilote reste
un témoin de mécanisme clé/plage, pas une comparaison des moteurs baseline/fix.

La [première cellule](outils/cx-premiere-cellule-revue.json), ready_to_run=false,
reste 65 536 entrées × 64 plages, R=4 096, trois paires. Plafond global de mesure
15 minutes, distinct du build privé terminé ; aucune matrice complète ni
attribution au scénario DDL autorisée. Aucun magasin/lib distribuée modifié.
