include_directories(
    ${PROJECT_SOURCE_DIR}/third_party/yyjson/src
    ${PROJECT_SOURCE_DIR}/extension/json/src/include
)

add_library(rag3db_shell_printer
        OBJECT
        json_printer.cpp
        printer_factory.cpp
        ${PROJECT_SOURCE_DIR}/extension/json/src/utils/json_utils.cpp
        ${PROJECT_SOURCE_DIR}/extension/json/src/type/json_type.cpp
)

set(RAG3DB_SHELL_OBJECT_FILES
        ${RAG3DB_SHELL_OBJECT_FILES} $<TARGET_OBJECTS:rag3db_shell_printer>
        PARENT_SCOPE)
