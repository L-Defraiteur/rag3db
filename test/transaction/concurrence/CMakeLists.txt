# Le banc de concurrence (marche A1). Les cas connus rouges tournent dans la passe et
# y restent rouges, sous le label concurrence-rouge-connu ; concurrence_test.known_red
# compare le résultat complet à known_red.txt. Les cas probabilistes
# (probabilistic.txt) tournent sous le label concurrence-probabiliste et ne sont pas
# comparés. Jamais de DISABLED_.

add_executable(concurrence_test concurrency_bench_test.cpp integrity_checker.cpp
    minimal_reproduction_test.cpp integrity_checker_test.cpp harness_mechanics_test.cpp
    lock_bench_test.cpp)
target_link_libraries(concurrence_test PRIVATE test_helper test_runner graph_test)
if (ENABLE_BACKTRACES)
    target_link_libraries(concurrence_test PRIVATE register_backtrace_signal_handler)
endif()
target_include_directories(concurrence_test PRIVATE ${PROJECT_SOURCE_DIR}/test/include)
include(GoogleTest)

if (ENABLE_THREAD_SANITIZER)
    # Le build ThreadSanitizer n'enregistre que la passe à signatures (label
    # concurrence-tsan) : sous TSan, les tests ordinaires sortiraient en erreur au premier
    # avertissement, et rien n'y est déterministe. Voir compare_tsan_signatures.py.
    find_package(Python3 COMPONENTS Interpreter REQUIRED)
    add_test(NAME concurrence_tsan.signatures
        COMMAND ${Python3_EXECUTABLE} ${CMAKE_CURRENT_SOURCE_DIR}/compare_tsan_signatures.py
            $<TARGET_FILE:concurrence_test> ${CMAKE_CURRENT_SOURCE_DIR}/tsan_signatures.txt
            ${CMAKE_CURRENT_BINARY_DIR}/tsan-reports
    )
    set_tests_properties(concurrence_tsan.signatures PROPERTIES LABELS concurrence-tsan)
    return()
endif()

set(KNOWN_RED_FILE ${CMAKE_CURRENT_SOURCE_DIR}/known_red.txt)
set(PROBABILISTIC_FILE ${CMAKE_CURRENT_SOURCE_DIR}/probabilistic.txt)
set_property(DIRECTORY APPEND PROPERTY CMAKE_CONFIGURE_DEPENDS
    ${KNOWN_RED_FILE} ${PROBABILISTIC_FILE})
file(STRINGS ${KNOWN_RED_FILE} KNOWN_RED_LINES ENCODING UTF-8 REGEX "^[^#]")
set(KNOWN_RED)
foreach (line IN LISTS KNOWN_RED_LINES)
    string(REGEX REPLACE "^([^ \t]+).*$" "\\1" name "${line}")
    list(APPEND KNOWN_RED ${name})
endforeach()
file(STRINGS ${PROBABILISTIC_FILE} PROBABILISTIC ENCODING UTF-8 REGEX "^[^#]")
list(JOIN KNOWN_RED ":" KNOWN_RED_FILTER)
list(JOIN PROBABILISTIC ":" PROBABILISTIC_FILTER)

# Trois groupes : les cas ordinaires, les rouges connus, les cas probabilistes.
set(SET_ASIDE ${KNOWN_RED} ${PROBABILISTIC})
list(JOIN SET_ASIDE ":" SET_ASIDE_FILTER)
gtest_discover_tests(concurrence_test
    DISCOVERY_TIMEOUT 600
    DISCOVERY_MODE PRE_TEST
    TEST_FILTER "-${SET_ASIDE_FILTER}"
)
if (KNOWN_RED_FILTER)
    gtest_discover_tests(concurrence_test
        DISCOVERY_TIMEOUT 600
        DISCOVERY_MODE PRE_TEST
        TEST_FILTER "${KNOWN_RED_FILTER}"
        PROPERTIES LABELS concurrence-rouge-connu
    )
endif()
if (PROBABILISTIC_FILTER)
    gtest_discover_tests(concurrence_test
        DISCOVERY_TIMEOUT 600
        DISCOVERY_MODE PRE_TEST
        TEST_FILTER "${PROBABILISTIC_FILTER}"
        PROPERTIES LABELS concurrence-probabiliste
    )
endif()

add_test(NAME concurrence_test.known_red
    COMMAND ${CMAKE_COMMAND}
        -DBENCH=$<TARGET_FILE:concurrence_test>
        -DKNOWN_RED_FILE=${KNOWN_RED_FILE}
        -DPROBABILISTIC_FILE=${PROBABILISTIC_FILE}
        -DRESULT_FILE=${CMAKE_CURRENT_BINARY_DIR}/concurrence_test_result.json
        -P ${CMAKE_CURRENT_SOURCE_DIR}/compare_known_red.cmake
)
