#include "test_helper/test_helper.h"

#ifdef _WIN32
#include <process.h>
#else
#include <signal.h>
#include <unistd.h>
#endif

#include <cerrno>
#include <chrono>
#include <fstream>
#include <iostream>

#include "gtest/gtest.h"
#include "spdlog/spdlog.h"

using namespace rag3db::common;
using namespace rag3db::planner;
using namespace rag3db::main;

namespace rag3db {
namespace testing {

std::vector<std::string> TestHelper::convertResultToString(QueryResult& queryResult,
    bool checkOutputOrder) {
    std::vector<std::string> actualOutput;
    while (queryResult.hasNext()) {
        auto tuple = queryResult.getNext();
        actualOutput.push_back(tuple->toString(std::vector<uint32_t>(tuple->len(), 0)));
    }
    if (!checkOutputOrder) {
        sort(actualOutput.begin(), actualOutput.end());
    }
    return actualOutput;
}

void TestHelper::executeScript(const std::string& cypherScript, Connection& conn) {
    std::cout << "cypherScript: " << cypherScript << std::endl;
    if (!std::filesystem::exists(cypherScript)) {
        std::cout << "cypherScript: " << cypherScript << " doesn't exist. Skipping..." << std::endl;
        return;
    }
    std::ifstream file(cypherScript);
    if (!file.is_open()) {
        throw Exception(stringFormat("Error opening file: {}, errno: {}.", cypherScript, errno));
    }
    std::string line;
    while (getline(file, line)) {
        // If this is a COPY statement, we need to append the RAG3DB_ROOT_DIRECTORY to the csv
        // file path. There maybe multiple csv files in the line, so we need to find all of them.
        std::vector<std::string> csvFilePaths;
        size_t index = 0;
        while (true) {
            size_t start = line.find('"', index);
            if (start == std::string::npos) {
                break;
            }
            size_t end = line.find('"', start + 1);
            if (end == std::string::npos) {
                // No matching double quote, must not be a file path.
                break;
            }
            std::string substr = line.substr(start + 1, end - start - 1);
            // convert to lower case
            auto substrLower = substr;
            std::transform(substrLower.begin(), substrLower.end(), substrLower.begin(), ::tolower);
            if (substrLower.find(".csv") != std::string::npos ||
                substrLower.find(".parquet") != std::string::npos ||
                substrLower.find(".npy") != std::string::npos ||
                substrLower.find(".ttl") != std::string::npos ||
                substrLower.find(".nq") != std::string::npos ||
                substrLower.find(".json") != std::string::npos ||
                substrLower.find(".rag3db_extension") != std::string::npos) {
                csvFilePaths.push_back(substr);
            }
            index = end + 1;
        }
        for (auto& csvFilePath : csvFilePaths) {
            auto fullPath = appendRag3dbRootPath(csvFilePath);
            line.replace(line.find(csvFilePath), csvFilePath.length(), fullPath);
        }
#ifdef __STATIC_LINK_EXTENSION_TEST__
        if (line.starts_with("load extension")) {
            continue;
        }
#endif
        std::cout << "Starting to execute query: " << line << std::endl;
        auto result = conn.query(line);
        std::cout << "Executed query: " << line << std::endl;
        if (!result->isSuccess()) {
            throw Exception(stringFormat("Failed to execute statement: {}.\nError: {}", line,
                result->getErrorMessage()));
        }
    }
}

std::unique_ptr<SystemConfig> TestHelper::getSystemConfigFromEnv() {
    auto systemConfig = std::make_unique<SystemConfig>();
    auto bufferPoolSizeEnv = getSystemEnv("BUFFER_POOL_SIZE");
    auto maxNumThreadsEnv = getSystemEnv("MAX_NUM_THREADS");
    auto enableCompressionEnv = getSystemEnv("ENABLE_COMPRESSION");
    auto checkpointThresholdEnv = getSystemEnv("CHECKPOINT_THRESHOLD");
    auto forceCheckpointOnCloseEnv = getSystemEnv("FORCE_CHECKPOINT_ON_CLOSE");
    auto enableChecksumEnv = getSystemEnv("ENABLE_CHECKSUMS");
    auto maxDBSizeEnv = getSystemEnv("MAX_DB_SIZE");
    systemConfig->bufferPoolSize = bufferPoolSizeEnv.empty() ?
                                       DEFAULT_BUFFER_POOL_SIZE_FOR_TESTING :
                                       std::stoull(bufferPoolSizeEnv);
    systemConfig->maxNumThreads = maxNumThreadsEnv.empty() ? 2 : std::stoull(maxNumThreadsEnv);
    if (!enableCompressionEnv.empty()) {
        systemConfig->enableCompression =
            StringUtils::caseInsensitiveEquals(enableCompressionEnv, "true");
    }
    systemConfig->checkpointThreshold = checkpointThresholdEnv.empty() ?
                                            systemConfig->checkpointThreshold :
                                            std::stoull(checkpointThresholdEnv);
    systemConfig->forceCheckpointOnClose =
        forceCheckpointOnCloseEnv.empty() ?
            systemConfig->forceCheckpointOnClose :
            StringUtils::caseInsensitiveEquals(forceCheckpointOnCloseEnv, "true");
    systemConfig->enableChecksums =
        enableChecksumEnv.empty() ? systemConfig->enableChecksums :
                                    StringUtils::caseInsensitiveEquals(enableChecksumEnv, "true");
    systemConfig->maxDBSize =
        maxDBSizeEnv.empty() ? systemConfig->maxDBSize : std::stoull(maxDBSizeEnv);
    return systemConfig;
}

std::string TestHelper::getTempSuffix() {
    uint64_t val = std::chrono::high_resolution_clock::now().time_since_epoch().count();
    return std::to_string(val);
}

namespace {
// Pris à l'initialisation statique, donc avant main et avant tout fork.
#ifdef _WIN32
const int LAUNCHER_PID = _getpid();
#else
const int LAUNCHER_PID = static_cast<int>(getpid());
#endif

bool processExists(int pid) {
#ifdef _WIN32
    // Pas de lecture des processus sous Windows : on ne touche pas aux autres parents.
    (void)pid;
    return true;
#else
    return kill(static_cast<pid_t>(pid), 0) == 0 || errno != ESRCH;
#endif
}
} // namespace

int TestHelper::launcherPid() {
    return LAUNCHER_PID;
}

void TestHelper::removeDeadRunTempDirs() {
    std::error_code ec;
    const auto shared = getSharedTempRoot();
    if (!std::filesystem::is_directory(shared, ec)) {
        return;
    }
    for (const auto& entry : std::filesystem::directory_iterator(shared, ec)) {
        if (!entry.is_directory(ec)) {
            continue;
        }
        const auto name = entry.path().filename().string();
        if (name.empty() || name.size() > 9 ||
            name.find_first_not_of("0123456789") != std::string::npos) {
            continue; // un dossier d'avant le parent par passe, ou d'un autre outil
        }
        const int pid = std::stoi(name);
        if (pid == LAUNCHER_PID || processExists(pid)) {
            continue;
        }
        std::filesystem::remove_all(entry.path(), ec);
    }
}

namespace {
// Le parent de la passe : effacé au début (une passe tuée au même pid, rare) avec ceux des
// passes mortes, et à la fin par le lanceur seul — un enfant fork ne passe pas par là.
class TempDirPerRun : public ::testing::Environment {
public:
    void SetUp() override {
        std::error_code ec;
        std::filesystem::remove_all(TestHelper::getRootTempDir(), ec);
        TestHelper::removeDeadRunTempDirs();
    }

    void TearDown() override {
#ifndef _WIN32
        if (static_cast<int>(getpid()) != LAUNCHER_PID) {
            return;
        }
#endif
        std::error_code ec;
        std::filesystem::remove_all(TestHelper::getRootTempDir(), ec);
    }
};

[[maybe_unused]] const auto* const TEMP_DIR_PER_RUN =
    ::testing::AddGlobalTestEnvironment(new TempDirPerRun());
} // namespace

} // namespace testing
} // namespace rag3db
