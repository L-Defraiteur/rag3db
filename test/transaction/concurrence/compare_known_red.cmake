# Lance tout le banc, puis compare l'ensemble des cas rouges à known_red.txt.
# Échoue sur tout écart : un rouge nouveau, un rouge connu devenu vert, un rouge
# connu sauté ou absent. Les cas de probabilistic.txt sont ignorés, dans un sens
# comme dans l'autre.
# Usage : cmake -DBENCH=… -DKNOWN_RED_FILE=… -DPROBABILISTIC_FILE=… -DRESULT_FILE=… -P …

file(REMOVE ${RESULT_FILE})
execute_process(COMMAND ${BENCH} --gtest_output=json:${RESULT_FILE}
    RESULT_VARIABLE bench_status
    OUTPUT_QUIET ERROR_QUIET)
if (NOT EXISTS ${RESULT_FILE})
    message(FATAL_ERROR "the bench wrote no result (exit status: ${bench_status}): "
        "it probably crashed; run ${BENCH} directly")
endif()

file(READ ${RESULT_FILE} result)
set(red)
set(passed)
string(JSON num_suites LENGTH "${result}" testsuites)
math(EXPR last_suite "${num_suites} - 1")
foreach (s RANGE ${last_suite})
    string(JSON suite_name GET "${result}" testsuites ${s} name)
    string(JSON num_tests LENGTH "${result}" testsuites ${s} testsuite)
    math(EXPR last_test "${num_tests} - 1")
    foreach (t RANGE ${last_test})
        string(JSON test_name GET "${result}" testsuites ${s} testsuite ${t} name)
        string(JSON test_result GET "${result}" testsuites ${s} testsuite ${t} result)
        string(JSON failures ERROR_VARIABLE no_failure
            GET "${result}" testsuites ${s} testsuite ${t} failures)
        set(full_name "${suite_name}.${test_name}")
        if (NOT no_failure)
            list(APPEND red ${full_name})
        elseif (test_result STREQUAL "COMPLETED")
            list(APPEND passed ${full_name})
        endif()
    endforeach()
endforeach()

file(STRINGS ${KNOWN_RED_FILE} known_red ENCODING UTF-8 REGEX "^[^#]")
file(STRINGS ${PROBABILISTIC_FILE} probabilistic ENCODING UTF-8 REGEX "^[^#]")
set(problems)
foreach (name IN LISTS red)
    if (NOT name IN_LIST known_red AND NOT name IN_LIST probabilistic)
        list(APPEND problems "new red: ${name}")
    endif()
endforeach()
foreach (name IN LISTS known_red)
    if (name IN_LIST passed)
        list(APPEND problems "known red turned green (remove it from known_red.txt in the commit of the step that fixed it): ${name}")
    elseif (NOT name IN_LIST red)
        list(APPEND problems "known red neither red nor green (skipped or missing): ${name}")
    endif()
endforeach()

list(LENGTH red num_red)
list(LENGTH passed num_passed)
if (problems)
    list(JOIN problems "\n  " text)
    message(FATAL_ERROR "the bench disagrees with known_red.txt "
        "(${num_red} red, ${num_passed} green):\n  ${text}")
endif()
message(STATUS "bench matches known_red.txt: ${num_red} known red, ${num_passed} green")
