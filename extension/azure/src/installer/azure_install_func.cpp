#include "installer/duckdb_installer.h"

extern "C" {
// Because we link against the static library on windows, we implicitly inherit RAG3DB_STATIC_DEFINE,
// which cancels out any exporting, so we can't use RAG3DB_API.
#if defined(_WIN32)
#define INIT_EXPORT __declspec(dllexport)
#else
#define INIT_EXPORT __attribute__((visibility("default")))
#endif
INIT_EXPORT void install(const std::string& repo, rag3db::main::ClientContext& context) {
    rag3db::extension::InstallExtensionInfo info{"azure", repo, false /* forceInstall */};
    rag3db::duckdb_extension::DuckDBInstaller installer{info, context};
    installer.install();
}
}
