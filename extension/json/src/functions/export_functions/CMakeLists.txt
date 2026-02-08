add_library(rag3db_json_export_functions
        OBJECT
        json_export.cpp)

set(JSON_EXTENSION_OBJECT_FILES
        ${JSON_EXTENSION_OBJECT_FILES} $<TARGET_OBJECTS:rag3db_json_export_functions>
        PARENT_SCOPE)
