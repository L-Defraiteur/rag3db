#pragma once

#include <cstdint>

#include "common/api.h"

namespace rag3db {
namespace common {

// Les crochets de test du moteur lus dans l'environnement, pour les témoins d'un autre langage
// que le C++ (rag3weaver, en Rust), qui ne peuvent pas poser les doublures du moteur. Ils sont
// dans la lib livrée : décision de l'orchestration du 11 octobre 2026, renversable par Lucie.
//
// Deux gardes contre l'erreur :
//  - deux variables, RAG3DB_TEST_HOOKS=1 qui arme et RAG3DB_TEST_DIE_AT=<point> qui dit où ;
//    l'une sans l'autre ne fait rien ;
//  - armé, le moteur le dit sur la sortie d'erreur à l'ouverture de la base (« TEST HOOK: this
//    process will _exit(137) at checkpoint-logged »), pour que personne ne cherche une panne.
//    Un point inconnu est ignoré, en le disant.
//
// Lu à l'ouverture de chaque base (Database), pas une fois par processus : un témoin qui forke
// un fils après avoir ouvert des bases arme le fils seul.
//
// Les points :
//  - checkpoint-logged : juste après la marque CHECKPOINT au journal, avant la recopie des
//    pages fantômes (le même instant que DyingCheckpointer::AfterCheckpointLogged). Le journal
//    se termine par CHECKPOINT, le fichier fantôme est plein : la reprise d'un écrivain rejoue
//    le point de reprise ; une ouverture en lecture seule est refusée.
enum class TestDiePoint : uint8_t {
    NONE = 0,
    CHECKPOINT_LOGGED = 1,
};

// Les points de faute : RAG3DB_TEST_HOOKS=1 et RAG3DB_TEST_FAIL_AT=<point>, lus à l'ouverture de
// chaque base comme les points de mort. Armé, le moteur lève une RuntimeException la PREMIÈRE fois
// qu'il passe par ce point, puis le point se désarme : un témoin fait échouer une écriture, et les
// suivantes passent.
//  - rel-commit-between-directions : dans RelTable::commit, après le versement du premier sens
//    d'une table de relations et avant le second (le ticket des deux index d'une table de
//    relations qui ne s'accordent plus, 14 novembre 2026).
//  - add-column-between-chunked-groups : dans NodeGroup::addColumn, après la colonne neuve posée
//    sur le premier groupe de lignes d'un groupe de nœuds : un ADD COLUMN refusé à mi-chemin,
//    comme par un pool plein (le ticket du SIGSEGV de DdlEveryMinute, 15 novembre 2026).
enum class TestFailPoint : uint8_t {
    NONE = 0,
    REL_COMMIT_BETWEEN_DIRECTIONS = 1,
    ADD_COLUMN_BETWEEN_CHUNKED_GROUPS = 2,
};

struct TestHooks {
    static constexpr int DIE_EXIT_CODE = 137;

    // Lit les deux variables, et dit sur la sortie d'erreur ce qui est armé ou ignoré.
    static TestDiePoint readFromEnvironment();
    // Meurt par _Exit(DIE_EXIT_CODE) si `armed` est `point`.
    static void dieIfAt(TestDiePoint armed, TestDiePoint point);

    // Arme le point de faute que disent les deux variables (ou aucun), et le dit sur la sortie
    // d'erreur.
    static void armFailPointFromEnvironment();
    // Arme `point` (ou désarme, avec NONE), pour un témoin C++ qui veut faire échouer une écriture
    // précise, après sa mise en place. `delayMs` : le point attend ce délai avant de lever, pour
    // qu'une autre connexion ait le temps de se mettre en file sur le verrou que l'écriture tient.
    RAG3DB_API static void armFailPoint(TestFailPoint point, uint32_t delayMs = 0);
    // Lève une RuntimeException si `point` est armé, et le désarme.
    static void failOnceIfArmed(TestFailPoint point);
};

} // namespace common
} // namespace rag3db
