# Le banc de concurrence (marche A1). Les cas connus rouges tournent dans la passe et
# y restent rouges, sous le label concurrence-rouge-connu ; concurrence_test.known_red
# compare le résultat complet à known_red.txt. Jamais de DISABLED_.

add_executable(concurrence_test concurrency_bench_test.cpp integrity_checker.cpp)
target_link_libraries(concurrence_test PRIVATE test_helper test_runner graph_test)
if (ENABLE_BACKTRACES)
    target_link_libraries(concurrence_test PRIVATE register_backtrace_signal_handler)
endif()
target_include_directories(concurrence_test PRIVATE ${PROJECT_SOURCE_DIR}/test/include)
include(GoogleTest)

set(KNOWN_RED_FILE ${CMAKE_CURRENT_SOURCE_DIR}/known_red.txt)
set_property(DIRECTORY APPEND PROPERTY CMAKE_CONFIGURE_DEPENDS ${KNOWN_RED_FILE})
file(STRINGS ${KNOWN_RED_FILE} KNOWN_RED ENCODING UTF-8 REGEX "^[^#]")
list(JOIN KNOWN_RED ":" KNOWN_RED_FILTER)

if (KNOWN_RED_FILTER)
    gtest_discover_tests(concurrence_test
        DISCOVERY_TIMEOUT 600
        DISCOVERY_MODE PRE_TEST
        TEST_FILTER "-${KNOWN_RED_FILTER}"
    )
    gtest_discover_tests(concurrence_test
        DISCOVERY_TIMEOUT 600
        DISCOVERY_MODE PRE_TEST
        TEST_FILTER "${KNOWN_RED_FILTER}"
        PROPERTIES LABELS concurrence-rouge-connu
    )
else()
    gtest_discover_tests(concurrence_test
        DISCOVERY_TIMEOUT 600
        DISCOVERY_MODE PRE_TEST
    )
endif()

add_test(NAME concurrence_test.known_red
    COMMAND ${CMAKE_COMMAND}
        -DBENCH=$<TARGET_FILE:concurrence_test>
        -DKNOWN_RED_FILE=${KNOWN_RED_FILE}
        -DRESULT_FILE=${CMAKE_CURRENT_BINARY_DIR}/concurrence_test_result.json
        -P ${CMAKE_CURRENT_SOURCE_DIR}/compare_known_red.cmake
)
