include_directories(
        include
        ${PROJECT_SOURCE_DIR}/third_party/taywee_args/include
        ${PROJECT_SOURCE_DIR}/third_party/yyjson/src
        ${PROJECT_SOURCE_DIR}/extension/json/src/include
)

add_subdirectory(printer)

add_executable(rag3db_shell
        ${RAG3DB_SHELL_OBJECT_FILES}
        embedded_shell.cpp
        linenoise.cpp
        shell_runner.cpp)

if (NOT MSVC)
    # On Windows, rename the executable to rag3db.exe will cause an error
    # "multiple rules generate src/rag3db.lib", so we do not set the output
    # name for MSVC.
    set_target_properties(rag3db_shell PROPERTIES OUTPUT_NAME rag3db)
endif ()


target_link_libraries(rag3db_shell rag3db)
if (ENABLE_BACKTRACES)
    target_link_libraries(rag3db_shell register_backtrace_signal_handler)
endif ()
if (MSVC)
    target_link_libraries(rag3db_shell ws2_32)
endif ()

install(TARGETS rag3db_shell)
