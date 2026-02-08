add_library(rag3db_json_type
        OBJECT
        json_type.cpp)

set(JSON_EXTENSION_OBJECT_FILES
        ${JSON_EXTENSION_OBJECT_FILES} $<TARGET_OBJECTS:rag3db_json_type>
        PARENT_SCOPE)
