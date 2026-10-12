// Le verrou d'un fichier, une fois par processus (LocalFileSystem::acquireProcessLock ; ticket
// « sous Windows, une base qui existe ne se rouvre pas en écriture »).
//
// Avant, chaque ouverture verrouillée posait son propre verrou sur sa propre poignée. Sous Unix
// (fcntl, tenu par le processus) la seconde poignée passait, mais la fermeture de n'importe quel
// descripteur du fichier faisait tomber le verrou de tout le processus : après le rejeu des pages
// fantômes, qui ouvre et ferme une poignée sur le fichier de données, la garde B2 ne gardait plus
// rien. Sous Windows (LockFileEx, tenu par la poignée) la seconde poignée du même processus était
// refusée. Désormais la première ouverture prend le verrou sur une poignée à elle, les suivantes
// s'y joignent, la dernière fermeture le rend.
//
// Les témoins ouvrent le fichier par LocalFileSystem directement, et demandent à un processus fils
// (fork, sans attente) si le verrou tient encore.

#ifndef _WIN32

#include <sys/wait.h>
#include <chrono>
#include <filesystem>
#include <thread>
#include <unistd.h>

#include "common/exception/io.h"
#include "common/file_system/local_file_system.h"
#include "graph_test/private_graph_test.h"

using namespace rag3db::common;
using namespace rag3db::testing;

namespace {

class FileLockRegistry : public EmptyDBTest {
protected:
    void SetUp() override {
        EmptyDBTest::SetUp();
        path = databasePath + ".lock-test";
        // Le fichier, créé sans verrou.
        fs.openFile(path,
            FileOpenFlags(FileFlags::READ_ONLY | FileFlags::WRITE | FileFlags::CREATE_IF_NOT_EXISTS));
    }

    void TearDown() override {
        std::filesystem::remove(path);
        EmptyDBTest::TearDown();
    }

    std::unique_ptr<FileInfo> open(FileLockType lockType) {
        return fs.openFile(path, FileOpenFlags(FileFlags::READ_ONLY | FileFlags::WRITE, lockType));
    }

    // Un processus fils essaie le verrou sans attendre : vrai s'il l'obtient.
    bool anotherProcessCanLock(FileLockType lockType) {
        const auto pid = fork();
        if (pid == 0) {
            int code = 0;
            try {
                LocalFileSystem childFs{""};
                childFs.openFile(path,
                    FileOpenFlags(FileFlags::READ_ONLY | FileFlags::WRITE, lockType));
            } catch (IOException&) {
                code = 1;
            }
            _exit(code);
        }
        int status = 0;
        waitpid(pid, &status, 0);
        return WIFEXITED(status) && WEXITSTATUS(status) == 0;
    }

    LocalFileSystem fs{""};
    std::string path;
};

} // namespace

// Deux ouvertures en écriture du même processus se joignent au même verrou ; la fermeture de la
// seconde ne le rend pas (c'était le trou de fcntl), la fermeture de la première non plus tant que
// l'autre vit ; la dernière le rend.
TEST_F(FileLockRegistry, TheLockOutlivesTheOtherHandlesOfTheSameProcess) {
    auto first = open(FileLockType::WRITE_LOCK);
    std::unique_ptr<FileInfo> second;
    ASSERT_NO_THROW(second = open(FileLockType::WRITE_LOCK)) << "[check: second-handle-joins]";
    EXPECT_FALSE(anotherProcessCanLock(FileLockType::WRITE_LOCK)) << "[check: other-writer-refused]";
    EXPECT_FALSE(anotherProcessCanLock(FileLockType::READ_LOCK)) << "[check: other-reader-refused]";
    second.reset();
    EXPECT_FALSE(anotherProcessCanLock(FileLockType::WRITE_LOCK))
        << "[check: lock-kept-after-second-close]";
    first.reset();
    EXPECT_TRUE(anotherProcessCanLock(FileLockType::WRITE_LOCK)) << "[check: lock-released-by-last]";
    // L'ordre inverse : la première fermée avant la seconde.
    first = open(FileLockType::WRITE_LOCK);
    second = open(FileLockType::WRITE_LOCK);
    first.reset();
    EXPECT_FALSE(anotherProcessCanLock(FileLockType::WRITE_LOCK))
        << "[check: lock-kept-after-first-close]";
    second.reset();
    EXPECT_TRUE(anotherProcessCanLock(FileLockType::WRITE_LOCK));
}

// Un verrou de lecture laisse lire les autres processus ; une ouverture en écriture du même
// processus le renforce, et il reste en écriture jusqu'à la dernière fermeture.
TEST_F(FileLockRegistry, AWriterOfTheSameProcessStrengthensAReadLock) {
    auto reader = open(FileLockType::READ_LOCK);
    EXPECT_TRUE(anotherProcessCanLock(FileLockType::READ_LOCK)) << "[check: other-reader-joins]";
    EXPECT_FALSE(anotherProcessCanLock(FileLockType::WRITE_LOCK))
        << "[check: other-writer-refused-by-reader]";
    std::unique_ptr<FileInfo> writer;
    ASSERT_NO_THROW(writer = open(FileLockType::WRITE_LOCK)) << "[check: same-process-upgrade]";
    EXPECT_FALSE(anotherProcessCanLock(FileLockType::READ_LOCK))
        << "[check: other-reader-refused-after-upgrade]";
    writer.reset();
    EXPECT_FALSE(anotherProcessCanLock(FileLockType::READ_LOCK))
        << "[check: stays-exclusive-until-last-close]";
    reader.reset();
    EXPECT_TRUE(anotherProcessCanLock(FileLockType::WRITE_LOCK));
}

// Un autre processus qui tient le verrou en écriture : l'ouverture d'ici attend la borne puis est
// refusée par la phrase convenue (B2), et réussit une fois le verrou rendu.
TEST_F(FileLockRegistry, AnotherProcessHoldingTheLockIsNamedInTheRefusal) {
    int pipeFds[2];
    ASSERT_EQ(pipe(pipeFds), 0);
    const auto pid = fork();
    if (pid == 0) {
        LocalFileSystem childFs{""};
        auto held = childFs.openFile(path,
            FileOpenFlags(FileFlags::READ_ONLY | FileFlags::WRITE, FileLockType::WRITE_LOCK));
        [[maybe_unused]] auto written = write(pipeFds[1], "o", 1);
        std::this_thread::sleep_for(std::chrono::milliseconds(600));
        held.reset();
        _exit(0);
    }
    char byte = 0;
    ASSERT_EQ(read(pipeFds[0], &byte, 1), 1);
    FileOpenFlags flags{FileFlags::READ_ONLY | FileFlags::WRITE, FileLockType::WRITE_LOCK};
    flags.lockWaitMs = 100;
    std::string message;
    try {
        fs.openFile(path, flags);
    } catch (IOException& e) {
        message = e.what();
    }
    EXPECT_NE(message.find("waited 100 ms for the database file held by another writer process"),
        std::string::npos)
        << "[check: named-refusal] " << message;
    flags.lockWaitMs = 5000;
    EXPECT_NO_THROW(fs.openFile(path, flags)) << "[check: opens-once-released]";
    int status = 0;
    waitpid(pid, &status, 0);
    close(pipeFds[0]);
    close(pipeFds[1]);
}

#endif
