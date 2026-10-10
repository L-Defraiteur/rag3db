// Où fuient les pages ? Une scène par argument, jouée sur une base neuve en deux sessions :
// la première écrit puis se ferme SANS point de reprise ; la seconde rouvre (rejeu), retire toutes
// les tables, fait un point de reprise, et compte les pages du fichier moins les pages libres.
// Comparé à la même scène fermée AVEC un point de reprise avant la réouverture.
#include <filesystem>
#include <fstream>
#include <iostream>
#include <string>
#include <unistd.h>
#include <signal.h>
#include <sys/wait.h>
#include "rag3db.hpp"
using namespace rag3db::main;
static std::string path;
static void run(Connection& c, const std::string& q) {
    auto r = c.query(q);
    if (!r->isSuccess()) { std::cout << "ERREUR " << q.substr(0, 80) << " : " << r->getErrorMessage() << "\n"; _exit(3); }
}
static long one(Connection& c, const std::string& q) {
    auto r = c.query(q); if (!r->isSuccess() || !r->hasNext()) return -1;
    auto* v = r->getNext()->getValue(0); return v->isNull() ? 0 : std::stol(v->toString());
}
static void scene(Connection& c, const std::string& name, long rows) {
    std::string csv = path + ".csv";
    { std::ofstream f(csv); for (long i = 0; i < rows; i++) f << i << ",name " << i << "\n"; }
    run(c, "CREATE NODE TABLE Doc(id INT64 PRIMARY KEY, name STRING);");
    if (name == "copy") { run(c, "COPY Doc FROM '" + csv + "';"); }
    else if (name == "copy-alter") { run(c, "COPY Doc FROM '" + csv + "';"); run(c, "ALTER TABLE Doc ADD extra INT64 DEFAULT 7;"); }
    else if (name == "copy-ckpt-drop") { run(c, "COPY Doc FROM '" + csv + "';"); run(c, "CHECKPOINT;"); run(c, "DROP TABLE Doc;"); }
    else if (name == "copy-ckpt-alter") { run(c, "COPY Doc FROM '" + csv + "';"); run(c, "CHECKPOINT;"); run(c, "ALTER TABLE Doc ADD extra INT64 DEFAULT 7;"); }
    else if (name == "create") { run(c, "UNWIND range(0, " + std::to_string(rows - 1) + ") AS i CREATE (:Doc {id: i, name: 'n'});"); }
    else { std::cout << "scène inconnue\n"; _exit(2); }
}
static std::string count(Connection& c) {
    run(c, "CALL enable_internal_catalog=true;");
    auto t = c.query("CALL show_tables() RETURN name, type;");
    std::vector<std::pair<std::string,std::string>> tables;
    while (t->hasNext()) { auto r = t->getNext(); tables.push_back({r->getValue(0)->toString(), r->getValue(1)->toString()}); }
    for (auto& [n, ty] : tables) if (ty == "REL") run(c, "DROP TABLE `" + n + "`;");
    for (auto& [n, ty] : tables) if (ty != "REL") run(c, "DROP TABLE `" + n + "`;");
    run(c, "CALL enable_internal_catalog=false;");
    run(c, "CHECKPOINT;");
    long file = one(c, "CALL file_info() RETURN num_pages;");
    long free = one(c, "CALL fsm_info() RETURN sum(num_pages);");
    return "fichier " + std::to_string(file) + " libres " + std::to_string(free) + " occupées " + std::to_string(file - free);
}
// Une mort base ouverte : le fils joue la scène dans un bloc BEGIN non validé (ou validé si
// `commit`), puis se tue.
static void dieAfter(const std::string& name, long rows, const std::string& mode) {
    auto child = fork();
    if (child == 0) {
        SystemConfig config; Database db(path, config); Connection c(&db);
        run(c, "CALL auto_checkpoint=false;"); run(c, "CALL force_checkpoint_on_close=false;");
        if (mode.find("journalise") != std::string::npos) run(c, "CALL force_checkpoint_on_copy=false;");
        if (mode.find("replie") != std::string::npos) { run(c, "CALL force_checkpoint_on_copy=false;"); run(c, "CALL copy_journal_threshold=65536;"); }
        if (mode.find("commit") == std::string::npos) run(c, "BEGIN TRANSACTION;");
        scene(c, name, rows);
        if (mode.find("commit") == std::string::npos) std::cout << "  (non validée) ";
        std::cout << "  fichier avant la mort : " << std::filesystem::file_size(path) / 4096 << " pages\n";
        kill(getpid(), SIGKILL);
    }
    int status = 0; waitpid(child, &status, 0);
    if (!WIFSIGNALED(status)) { std::cout << "le fils n'est pas mort tué : " << status << "\n"; _exit(4); }
}
int main(int argc, char** argv) {
    std::cout.setf(std::ios::unitbuf);
    path = argv[1]; std::string name = argv[2]; long rows = std::stol(argv[3]); std::string mode = argv[4];
    bool journaled = mode.find("journalise") != std::string::npos;
    if (mode.find("mort") != std::string::npos) {
        std::filesystem::remove(path); std::filesystem::remove(path + ".wal");
        dieAfter(name, rows, mode);
        SystemConfig config; Database db(path, config); Connection c(&db);
        std::cout << name << " " << rows << " " << mode << " : " << count(c) << "\n";
        _exit(0);
    }
    for (bool withCheckpoint : {true, false}) {
        std::filesystem::remove(path); std::filesystem::remove(path + ".wal");
        SystemConfig config;
        {
            Database db(path, config); Connection c(&db);
            run(c, "CALL auto_checkpoint=false;"); run(c, "CALL force_checkpoint_on_close=false;");
            if (journaled) run(c, "CALL force_checkpoint_on_copy=false;");
            scene(c, name, rows);
            if (withCheckpoint) run(c, "CHECKPOINT;");
            std::error_code ec; auto wal = std::filesystem::file_size(path + ".wal", ec);
            std::cout << "  journal avant fermeture : " << (ec ? 0 : wal) << " octets, fichier " << std::filesystem::file_size(path) / 4096 << " pages\n";
        }
        Database db(path, config); Connection c(&db);
        std::cout << name << " " << rows << (journaled ? " journalisé" : " forcé") << (withCheckpoint ? ", fermé après point de reprise : " : ", fermé SANS point de reprise : ") << count(c) << "\n";
    }
    std::filesystem::remove(path + ".csv");
    _exit(0);
}
