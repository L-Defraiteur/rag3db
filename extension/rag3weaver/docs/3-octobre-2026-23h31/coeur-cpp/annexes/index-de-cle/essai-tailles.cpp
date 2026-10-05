// Des COPY validés de tailles données dans une table neuve, puis chaque clé cherchée par l'index.
#include <cstdio>
#include <filesystem>
#include <fstream>
#include <iostream>
#include <string>
#include <vector>
#include <unistd.h>
#include "rag3db.hpp"
using namespace rag3db::main;
static bool isString;
static std::string key(long i) {
    if (!isString) return std::to_string(i * 7919 + 13);
    char buf[64];
    snprintf(buf, sizeof buf, "une-cle-assez-longue-pour-deborder-%09ld", i);
    return buf;
}
int main(int argc, char** argv) {
    std::cout.setf(std::ios::unitbuf);
    std::string path = argv[1]; std::string type = argv[2]; isString = type == "STRING";
    std::vector<long> sizes; for (int i = 3; i < argc; i++) sizes.push_back(std::stol(argv[i]));
    std::filesystem::remove(path); std::filesystem::remove(path + ".wal");
    SystemConfig config;
    auto db = std::make_unique<Database>(path, config);
    auto conn = std::make_unique<Connection>(db.get());
    auto run = [&](const std::string& q) {
        auto r = conn->query(q);
        if (!r->isSuccess()) { std::cout << "ERREUR " << q.substr(0, 70) << " : " << r->getErrorMessage() << "\n"; _exit(3); }
        return r;
    };
    run("CREATE NODE TABLE T(id " + type + " PRIMARY KEY, name STRING);");
    long total = 0;
    for (auto n : sizes) {
        std::string csv = path + ".csv";
        { std::ofstream f(csv); for (long i = total; i < total + n; i++) f << key(i) << ",f" << i << "\n"; }
        run("COPY T FROM '" + csv + "';");
        total += n;
    }
    long missed = 0; std::string firsts;
    for (long i = 0; i < total; i++) {
        auto r = conn->query(isString ? "MATCH (s:T {id: '" + key(i) + "'}) RETURN count(*);" : "MATCH (s:T {id: " + key(i) + "}) RETURN count(*);");
        if (!r->isSuccess() || r->getNext()->getValue(0)->getValue<int64_t>() != 1) { if (missed++ < 6) firsts += " " + std::to_string(i); }
    }
    std::cout << type; for (auto n : sizes) std::cout << " " << n;
    std::cout << " : " << run("MATCH (s:T) RETURN count(*);")->getNext()->toString() << " clés introuvables : " << missed << firsts << "\n";
    _exit(0);
}
