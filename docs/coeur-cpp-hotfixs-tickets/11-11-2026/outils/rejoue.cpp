// Étude du 8 novembre 2026 (hotfixs, ne pas atterrir) : rejoue une trace Cypher exacte
// (RAG3WEAVER_TRACE_CYPHER_FICHIER, une instruction par ligne, paramètres en littéraux) contre
// une copie, et relève les deux sens des tables surveillées en cours de route.
// Usage : rejoue <copie>/base.rag3db <trace> <dernière ligne, 0 = toutes> <contrôle toutes les N
//         écritures surveillées, 0 = à la fin seulement> <table>...
// Code de sortie : 0 aucun écart, 1 écart (la ligne est imprimée), 2 erreur d'usage.
#include "check_rel_directions.cpp"

#include <fstream>
#include <sstream>

int main(int argc, char** argv) {
    if (argc < 6) {
        std::cerr << "usage : rejoue <base> <trace> <dernière ligne> <intervalle> <table>...\n";
        return 2;
    }
    const std::string base = argv[1];
    std::ifstream trace(argv[2]);
    const uint64_t last = std::stoull(argv[3]);
    const uint64_t every = std::stoull(argv[4]);
    std::vector<std::string> watched(argv + 5, argv + argc);
    SystemConfig config;
    config.bufferPoolSize = 8ull << 30; // la règle de l'hôte : la moitié de la RAM, plafonnée à 8 Gio
    Database db(base, config);
    Connection conn(&db);
    Connection reader(&db);
    std::string line;
    uint64_t n = 0, errors = 0, writes = 0, sinceCheck = 0;
    auto isWrite = [](const std::string& l) {
        for (auto* w : {"MERGE", "CREATE", "DELETE", "SET ", "COPY", "DROP", "ALTER"}) {
            if (l.find(w) != std::string::npos) return true;
        }
        return false;
    };
    // REJOUE_DEPUIS=<ligne> : à partir de cette ligne, toute écriture est suivie d'un relevé.
    const uint64_t from = getenv("REJOUE_DEPUIS") ? std::stoull(getenv("REJOUE_DEPUIS")) : UINT64_MAX;
    auto touches = [&](const std::string& l) {
        if (n >= from) return true;
        if (l.find("DELETE") != std::string::npos || l.find("COPY") != std::string::npos) return true;
        for (auto& t : watched) {
            if (l.find(t) != std::string::npos) return true;
        }
        return false;
    };
    auto checkAll = [&](uint64_t at) {
        bool gap = false;
        std::ostringstream out;
        auto* old = std::cout.rdbuf(out.rdbuf());
        for (auto& t : watched) gap = check(reader, t) || gap;
        std::cout.rdbuf(old);
        if (gap) {
            std::cout << "ÉCART après la ligne " << at << "\n" << out.str() << std::flush;
        }
        return gap;
    };
    while (std::getline(trace, line)) {
        n++;
        if (last && n > last) break;
        if (line.empty()) continue;
        auto r = conn.query(line);
        if (!r->isSuccess()) {
            if (errors++ < 30) {
                std::cout << "ERREUR ligne " << n << " : " << r->getErrorMessage().substr(0, 300)
                          << " ← " << line.substr(0, 200) << "\n" << std::flush;
            }
        }
        while (r->hasNext()) r->getNext();
        if (isWrite(line) && touches(line)) {
            writes++;
            if (every && ++sinceCheck >= every) {
                sinceCheck = 0;
                if (checkAll(n)) {
                    std::cout << "ligne " << n << " : " << line.substr(0, 600) << "\n";
                    std::cout << "lignes jouées " << n << ", erreurs " << errors << "\n";
                    return 1;
                }
            }
        }
        if (n % 5000 == 0) {
            std::cout << "… ligne " << n << ", écritures surveillées " << writes << ", erreurs " << errors << "\n" << std::flush;
        }
    }
    const bool gap = checkAll(n);
    std::cout << "lignes jouées " << n << ", écritures surveillées " << writes << ", erreurs " << errors
              << (gap ? " — ÉCART à la fin" : " — aucun écart") << "\n";
    return gap ? 1 : 0;
}
