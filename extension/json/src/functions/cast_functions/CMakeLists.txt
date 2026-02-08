add_library(rag3db_json_cast_functions
        OBJECT
        json_cast.cpp)

set(JSON_EXTENSION_OBJECT_FILES
        ${JSON_EXTENSION_OBJECT_FILES} $<TARGET_OBJECTS:rag3db_json_cast_functions>
        PARENT_SCOPE)
