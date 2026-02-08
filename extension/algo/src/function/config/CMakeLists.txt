add_library(rag3db_algo_config
        OBJECT
        max_iterations_config.cpp)

set(ALGO_EXTENSION_OBJECT_FILES
        ${ALGO_EXTENSION_OBJECT_FILES} $<TARGET_OBJECTS:rag3db_algo_config>
        PARENT_SCOPE)
