#pragma once

#include <cstdint>

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

struct TestHooks {
    static constexpr int DIE_EXIT_CODE = 137;

    // Lit les deux variables, et dit sur la sortie d'erreur ce qui est armé ou ignoré.
    static TestDiePoint readFromEnvironment();
    // Meurt par _Exit(DIE_EXIT_CODE) si `armed` est `point`.
    static void dieIfAt(TestDiePoint armed, TestDiePoint point);
};

} // namespace common
} // namespace rag3db
