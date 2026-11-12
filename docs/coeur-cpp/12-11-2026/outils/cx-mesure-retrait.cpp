// Pilote de mécanisme : retrait de clés déjà connues contre retrait par plage.
// Ce n’est pas un rollback moteur ni une comparaison de deux versions du moteur.
// Ticket :
// docs/coeur-cpp/tickets/2026-11-09-sous-vingt-ecrivains-un-pool-plein-laisse-une-annulation-inachevee.md
#include <sys/resource.h>

#include <algorithm>
#include <chrono>
#include <cstdint>
#include <iostream>
#include <memory>
#include <stdexcept>
#include <string>
#include <vector>

#include "storage/buffer_manager/buffer_manager.h"
#include "storage/buffer_manager/memory_manager.h"
#include "storage/index/in_mem_hash_index.h"

using namespace rag3db::common;
using namespace rag3db::storage;

namespace {
using Index = InMemHashIndex<ku_string_t>;
using Clock = std::chrono::steady_clock;

struct Options {
    uint64_t entries = 65'536;
    uint64_t ranges = 64;
    uint64_t removed = 4'096;
    uint64_t poolMiB = 512;
    std::string mode;
};

Options parse(int argc, char** argv) {
    Options o;
    for (int i = 1; i < argc; i += 2) {
        if (i + 1 == argc) {
            throw std::runtime_error("valeur manquante");
        }
        const std::string arg = argv[i];
        const std::string value = argv[i + 1];
        if (arg == "--mode") {
            o.mode = value;
        } else {
            if (value.empty() || value.find_first_not_of("0123456789") != std::string::npos) {
                throw std::runtime_error("entier positif attendu");
            }
            const auto n = std::stoull(value);
            if (arg == "--entries")
                o.entries = n;
            else if (arg == "--ranges")
                o.ranges = n;
            else if (arg == "--remove")
                o.removed = n;
            else if (arg == "--pool-mib")
                o.poolMiB = n;
            else
                throw std::runtime_error("option inconnue");
        }
    }
    if ((o.mode != "key-discard" && o.mode != "range-discard") || o.entries < o.removed ||
        o.entries > 262'144 || o.removed != 4'096 || o.ranges == 0 || o.ranges > 256 ||
        o.removed % o.ranges != 0 || o.poolMiB != 512) {
        throw std::runtime_error("hors du protocole borne");
    }
    return o;
}

void require(bool ok, const char* message) {
    if (!ok)
        throw std::runtime_error(message);
}

uint64_t addByte(uint64_t hash, uint8_t byte) {
    return (hash ^ byte) * 1099511628211ULL;
}

uint64_t addRow(uint64_t hash, const std::string& key, uint64_t offset) {
    for (const auto c : key)
        hash = addByte(hash, static_cast<uint8_t>(c));
    hash = addByte(hash, 0);
    for (unsigned shift = 0; shift < 64; shift += 8) {
        hash = addByte(hash, static_cast<uint8_t>(offset >> shift));
    }
    return hash;
}

uint64_t cpuMicros(const rusage& usage) {
    return (usage.ru_utime.tv_sec + usage.ru_stime.tv_sec) * 1'000'000ULL + usage.ru_utime.tv_usec +
           usage.ru_stime.tv_usec;
}

struct Sample {
    uint64_t first;
    uint64_t count;
    uint64_t elapsedNs;
    uint64_t poolBefore;
    uint64_t poolAfter;
};
} // namespace

int main(int argc, char** argv) {
    try {
        const auto o = parse(argc, argv);
        BufferManager bm(":memory:", "", o.poolMiB * 1024 * 1024, 2ULL * 1024 * 1024 * 1024,
            nullptr, true);
        MemoryManager mm(&bm, nullptr);
        std::vector<std::string> keys;
        keys.reserve(o.entries);
        std::vector<uint64_t> partitions;
        partitions.reserve(o.entries);
        std::vector<uint64_t> counts(NUM_HASH_INDEXES, 0);
        auto inputHash = 14695981039346656037ULL;
        for (uint64_t i = 0; i < o.entries; ++i) {
            keys.push_back(std::string(128, 'k') + std::to_string(i));
            const auto partition =
                HashIndexUtils::getHashIndexPosition(std::string_view{keys.back()});
            partitions.push_back(partition);
            ++counts[partition];
            inputHash = addRow(inputHash, keys.back(), i);
        }
        std::vector<std::unique_ptr<Index>> indexes;
        indexes.reserve(NUM_HASH_INDEXES);
        for (uint64_t p = 0; p < NUM_HASH_INDEXES; ++p) {
            auto index = std::make_unique<Index>(mm, nullptr);
            index->reserve(counts[p]);
            indexes.push_back(std::move(index));
        }
        const auto visible = [](offset_t) { return true; };
        for (uint64_t i = 0; i < o.entries; ++i) {
            require(indexes[partitions[i]]->append(std::string{keys[i]}, i, visible),
                "insertion initiale refusee");
        }
        const auto size = [&] {
            uint64_t n = 0;
            for (const auto& index : indexes)
                n += index->size();
            return n;
        };
        require(size() == o.entries, "compte initial incorrect");
        std::vector<Sample> samples;
        samples.reserve(o.ranges);
        const auto count = o.removed / o.ranges;
        const auto firstRemoved = o.entries - o.removed;
        const auto poolInitial = bm.getUsedMemory();
        uint64_t poolPeakAtBoundaries = poolInitial;
        rusage cpuBefore{}, cpuAfter{};
        getrusage(RUSAGE_SELF, &cpuBefore);
        uint64_t elapsedTotal = 0;
        for (uint64_t u = 0; u < o.ranges; ++u) {
            const auto first = firstRemoved + u * count;
            const auto poolBefore = bm.getUsedMemory();
            const auto start = Clock::now();
            if (o.mode == "range-discard") {
                for (auto& index : indexes)
                    index->discardRange(first, count);
            } else {
                for (uint64_t i = first; i < first + count; ++i) {
                    require(indexes[partitions[i]]->deleteKey(keys[i]), "cle non retiree");
                }
            }
            const auto end = Clock::now();
            const auto ns =
                std::chrono::duration_cast<std::chrono::nanoseconds>(end - start).count();
            const auto poolAfter = bm.getUsedMemory();
            elapsedTotal += ns;
            poolPeakAtBoundaries = std::max(poolPeakAtBoundaries, std::max(poolBefore, poolAfter));
            samples.push_back({first, count, static_cast<uint64_t>(ns), poolBefore, poolAfter});
        }
        getrusage(RUSAGE_SELF, &cpuAfter);
        require(size() == o.entries - o.removed, "compte apres retrait incorrect");
        auto survivorHash = 14695981039346656037ULL;
        for (uint64_t i = 0; i < o.entries; ++i) {
            offset_t found = INVALID_OFFSET;
            const auto present = indexes[partitions[i]]->lookup(keys[i], found, visible);
            require(present == (i < firstRemoved), "visibilite incorrecte");
            if (present) {
                require(found == i, "offset incorrect");
                survivorHash = addRow(survivorHash, keys[i], found);
            }
        }
        for (uint64_t i = firstRemoved; i < o.entries; ++i) {
            require(indexes[partitions[i]]->append(std::string{keys[i]}, i, visible),
                "reinsertion refusee");
            offset_t found = INVALID_OFFSET;
            require(indexes[partitions[i]]->lookup(keys[i], found, visible) && found == i,
                "reinsertion non visible");
        }
        require(size() == o.entries, "compte apres reinsertion incorrect");
        for (uint64_t u = 0; u < samples.size(); ++u) {
            const auto& s = samples[u];
            std::cout << "{\"kind\":\"range\",\"mode\":\"" << o.mode << "\",\"u\":" << u
                      << ",\"first\":" << s.first << ",\"count\":" << s.count
                      << ",\"ns\":" << s.elapsedNs << ",\"pool_before\":" << s.poolBefore
                      << ",\"pool_after\":" << s.poolAfter << "}\n";
        }
        std::cout << "{\"kind\":\"summary\",\"mechanism_only\":true,\"mode\":\"" << o.mode
                  << "\",\"entries\":" << o.entries << ",\"ranges\":" << o.ranges
                  << ",\"removed\":" << o.removed << ",\"input_fnv1a64\":" << inputHash
                  << ",\"survivors_fnv1a64\":" << survivorHash << ",\"latency_ns\":" << elapsedTotal
                  << ",\"cpu_us_with_boundary_sampling\":"
                  << cpuMicros(cpuAfter) - cpuMicros(cpuBefore)
                  << ",\"pool_initial\":" << poolInitial
                  << ",\"pool_peak_at_boundaries\":" << poolPeakAtBoundaries
                  << ",\"rss_peak_kib_before_validation\":" << cpuAfter.ru_maxrss
                  << ",\"validated\":true}\n";
        return 0;
    } catch (const std::exception& error) {
        std::cerr << error.what() << '\n';
        return 2;
    }
}
