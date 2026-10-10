#pragma once

#include "common/types/value/value.h"

namespace rag3db {
namespace main {

struct ThreadsSetting {
    static constexpr auto name = "threads";
    static constexpr auto inputType = common::LogicalTypeID::UINT64;
    static void setContext(ClientContext* context, const common::Value& parameter);
    static common::Value getSetting(const ClientContext* context);
};

struct WarningLimitSetting {
    static constexpr auto name = "warning_limit";
    static constexpr auto inputType = common::LogicalTypeID::UINT64;
    static void setContext(ClientContext* context, const common::Value& parameter);
    static common::Value getSetting(const ClientContext* context);
};

struct TimeoutSetting {
    static constexpr auto name = "timeout";
    static constexpr auto inputType = common::LogicalTypeID::UINT64;
    static void setContext(ClientContext* context, const common::Value& parameter);
    static common::Value getSetting(const ClientContext* context);
};

// L'attente maximale d'un verrou d'écriture, en millisecondes (CALL lock_timeout=…).
struct LockTimeoutSetting {
    static constexpr auto name = "lock_timeout";
    static constexpr auto inputType = common::LogicalTypeID::UINT64;
    static void setContext(ClientContext* context, const common::Value& parameter);
    static common::Value getSetting(const ClientContext* context);
};

// Le chargement en masse journalisé est le défaut : un COPY écrit ses lignes au journal de sa
// transaction. CALL force_checkpoint_on_copy=true rend le point de reprise forcé d'avant — pour
// une transaction dont le journal serait énorme (le plein texte en base), ou un test qui regarde
// l'état des pages juste après un COPY.
struct ForceCheckpointOnCopySetting {
    static constexpr auto name = "force_checkpoint_on_copy";
    static constexpr auto inputType = common::LogicalTypeID::BOOL;
    static void setContext(ClientContext* context, const common::Value& parameter);
    static common::Value getSetting(const ClientContext* context);
};

// Le journal d'une transaction vit en mémoire jusqu'à sa validation. Quand un COPY journalisé le
// porte au-delà de ce seuil (en octets ; 0 rend le défaut), la transaction se replie sur le
// point de reprise forcé : son journal est vidé, plus rien n'y est écrit, et sa validation fait
// un point de reprise, comme un COPY d'avant. CALL copy_journal_threshold=…
struct CopyJournalThresholdSetting {
    static constexpr auto name = "copy_journal_threshold";
    static constexpr auto inputType = common::LogicalTypeID::UINT64;
    static void setContext(ClientContext* context, const common::Value& parameter);
    static common::Value getSetting(const ClientContext* context);
};

// En lecture seule : combien de transactions de cette base se sont repliées ainsi depuis son
// ouverture — les replis, pas les validations : une transaction repliée puis annulée compte.
// CALL current_setting('copy_journal_fallbacks') RETURN *
struct CopyJournalFallbacksSetting {
    static constexpr auto name = "copy_journal_fallbacks";
    static constexpr auto inputType = common::LogicalTypeID::UINT64;
    static void setContext(ClientContext* context, const common::Value& parameter);
    static common::Value getSetting(const ClientContext* context);
};

struct ProgressBarSetting {
    static constexpr auto name = "progress_bar";
    static constexpr auto inputType = common::LogicalTypeID::BOOL;
    static void setContext(ClientContext* context, const common::Value& parameter);
    static common::Value getSetting(const ClientContext* context);
};

struct VarLengthExtendMaxDepthSetting {
    static constexpr auto name = "var_length_extend_max_depth";
    static constexpr auto inputType = common::LogicalTypeID::INT64;
    static void setContext(ClientContext* context, const common::Value& parameter);
    static common::Value getSetting(const ClientContext* context);
};

struct SparseFrontierThresholdSetting {
    static constexpr auto name = "sparse_frontier_threshold";
    static constexpr auto inputType = common::LogicalTypeID::INT64;
    static void setContext(ClientContext* context, const common::Value& parameter);
    static common::Value getSetting(const ClientContext* context);
};

struct EnableSemiMaskSetting {
    static constexpr auto name = "enable_semi_mask";
    static constexpr auto inputType = common::LogicalTypeID::BOOL;
    static void setContext(ClientContext* context, const common::Value& parameter);
    static common::Value getSetting(const ClientContext* context);
};

struct DisableMapKeyCheck {
    static constexpr auto name = "disable_map_key_check";
    static constexpr auto inputType = common::LogicalTypeID::BOOL;
    static void setContext(ClientContext* context, const common::Value& parameter);
    static common::Value getSetting(const ClientContext* context);
};

struct EnableZoneMapSetting {
    static constexpr auto name = "enable_zone_map";
    static constexpr auto inputType = common::LogicalTypeID::BOOL;
    static void setContext(ClientContext* context, const common::Value& parameter);
    static common::Value getSetting(const ClientContext* context);
};

struct HomeDirectorySetting {
    static constexpr auto name = "home_directory";
    static constexpr auto inputType = common::LogicalTypeID::STRING;
    static void setContext(ClientContext* context, const common::Value& parameter);
    static common::Value getSetting(const ClientContext* context);
};

struct FileSearchPathSetting {
    static constexpr auto name = "file_search_path";
    static constexpr auto inputType = common::LogicalTypeID::STRING;
    static void setContext(ClientContext* context, const common::Value& parameter);
    static common::Value getSetting(const ClientContext* context);
};

struct RecursivePatternSemanticSetting {
    static constexpr auto name = "recursive_pattern_semantic";
    static constexpr auto inputType = common::LogicalTypeID::STRING;
    static void setContext(ClientContext* context, const common::Value& parameter);
    static common::Value getSetting(const ClientContext* context);
};

struct RecursivePatternFactorSetting {
    static constexpr auto name = "recursive_pattern_factor";
    static constexpr auto inputType = common::LogicalTypeID::INT64;
    static void setContext(ClientContext* context, const common::Value& parameter);
    static common::Value getSetting(const ClientContext* context);
};

struct EnableMVCCSetting {
    static constexpr auto name = "debug_enable_multi_writes";
    static constexpr auto inputType = common::LogicalTypeID::BOOL;
    static void setContext(ClientContext* context, const common::Value& parameter);
    static common::Value getSetting(const ClientContext* context);
};

struct CheckpointThresholdSetting {
    static constexpr auto name = "checkpoint_threshold";
    static constexpr auto inputType = common::LogicalTypeID::INT64;
    static void setContext(ClientContext* context, const common::Value& parameter);
    static common::Value getSetting(const ClientContext* context);
};

struct AutoCheckpointSetting {
    static constexpr auto name = "auto_checkpoint";
    static constexpr auto inputType = common::LogicalTypeID::BOOL;
    static void setContext(ClientContext* context, const common::Value& parameter);
    static common::Value getSetting(const ClientContext* context);
};

struct ForceCheckpointClosingDBSetting {
    static constexpr auto name = "force_checkpoint_on_close";
    static constexpr auto inputType = common::LogicalTypeID::BOOL;
    static void setContext(ClientContext* context, const common::Value& parameter);
    static common::Value getSetting(const ClientContext* context);
};

struct SpillToDiskSetting {
    static constexpr auto name = "spill_to_disk";
    static constexpr auto inputType = common::LogicalTypeID::BOOL;
    static void setContext(ClientContext* context, const common::Value& parameter);
    static common::Value getSetting(const ClientContext* context);
};

struct EnableOptimizerSetting {
    static constexpr auto name = "enable_plan_optimizer";
    static constexpr auto inputType = common::LogicalTypeID::BOOL;
    static void setContext(ClientContext* context, const common::Value& parameter);
    static common::Value getSetting(const ClientContext* context);
};

struct EnableInternalCatalogSetting {
    static constexpr auto name = "enable_internal_catalog";
    static constexpr auto inputType = common::LogicalTypeID::BOOL;
    static void setContext(ClientContext* context, const common::Value& parameter);
    static common::Value getSetting(const ClientContext* context);
};

} // namespace main
} // namespace rag3db
