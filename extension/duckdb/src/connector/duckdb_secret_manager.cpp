#include "connector/duckdb_secret_manager.h"

#include "s3fs_config.h"

namespace rag3db {
namespace duckdb_extension {

static std::string getDuckDBExtensionOptions(httpfs_extension::S3AuthParams rag3dbOptions) {
    std::string options = "";
    options.append(common::stringFormat("KEY_ID '{}',", rag3dbOptions.accessKeyID));
    options.append(common::stringFormat("SECRET '{}',", rag3dbOptions.secretAccessKey));
    options.append(common::stringFormat("ENDPOINT '{}',", rag3dbOptions.endpoint));
    options.append(common::stringFormat("URL_STYLE '{}',", rag3dbOptions.urlStyle));
    options.append(common::stringFormat("REGION '{}',", rag3dbOptions.region));
    return options;
}

std::string DuckDBSecretManager::getRemoteS3FSSecret(main::ClientContext* context,
    const httpfs_extension::S3FileSystemConfig& config) {
    KU_ASSERT(config.fsName == "S3" || config.fsName == "GCS");
    std::string templateQuery = R"(CREATE SECRET {}_secret (
        {}
        TYPE {}
    );)";
    return common::stringFormat(templateQuery, config.fsName,
        getDuckDBExtensionOptions(config.getAuthParams(context)), config.fsName);
}

} // namespace duckdb_extension
} // namespace rag3db
