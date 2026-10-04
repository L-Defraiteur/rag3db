#pragma once

#include <chrono>
#include <cstdlib>

namespace rag3db {
namespace storage {

// Le découpage du temps d'un commit et de son point de reprise, écrit sur la sortie d'erreur
// quand la variable d'environnement RAG3DB_PROFILE_CHECKPOINT est posée. Chaque ligne commence
// par [commit-profile] ou [checkpoint-profile] ; sur une ligne, les postes s'additionnent au
// total. Sans la variable, rien n'est mesuré ni écrit.
struct CheckpointProfile {
    using clock = std::chrono::steady_clock;

    static bool enabled() {
        static const bool on = std::getenv("RAG3DB_PROFILE_CHECKPOINT") != nullptr;
        return on;
    }

    // Les millisecondes écoulées depuis le dernier appel (ou depuis la construction).
    double lap() {
        const auto now = clock::now();
        const auto ms = std::chrono::duration<double, std::milli>(now - last).count();
        last = now;
        return ms;
    }
    double total() const {
        return std::chrono::duration<double, std::milli>(clock::now() - start).count();
    }

    clock::time_point start = clock::now();
    clock::time_point last = start;
};

} // namespace storage
} // namespace rag3db
