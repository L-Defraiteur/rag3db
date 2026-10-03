# Lance tout le banc, puis compare l'ensemble des cas rouges à known_red.txt.
# Échoue sur tout écart : un rouge nouveau, un rouge connu devenu vert, un rouge
# connu sauté ou absent, un rouge connu dont les étiquettes « [check: …] » ne sont pas
# exactement celles de known_red.txt (un rouge qui change de cause). Les cas de
# probabilistic.txt sont ignorés, dans un sens comme dans l'autre.
# Usage : cmake -DBENCH=… -DKNOWN_RED_FILE=… -DPROBABILISTIC_FILE=… -DRESULT_FILE=… -P …

# Les cas probabilistes ne sont pas lancés dans cette passe : la comparaison les
# ignore, et C5 peut faire planter tout le processus (course du chemin de suppression
# sans verrou, SIGSEGV dans VersionInfo::isSelected, 3 octobre) — la passe entière
# serait perdue. ctest les lance un par un sous le label concurrence-probabiliste.
file(STRINGS ${PROBABILISTIC_FILE} probabilistic ENCODING UTF-8 REGEX "^[^#]")
list(JOIN probabilistic ":" probabilistic_filter)
set(filter "*")
if (probabilistic_filter)
    set(filter "-${probabilistic_filter}")
endif()

file(REMOVE ${RESULT_FILE})
execute_process(COMMAND ${BENCH} --gtest_filter=${filter} --gtest_output=json:${RESULT_FILE}
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
        string(JSON num_failures ERROR_VARIABLE no_failure
            LENGTH "${result}" testsuites ${s} testsuite ${t} failures)
        set(full_name "${suite_name}.${test_name}")
        if (NOT no_failure)
            list(APPEND red ${full_name})
            # Les étiquettes « [check: …] » des messages d'échec : la raison du rouge.
            set(checks)
            math(EXPR last_failure "${num_failures} - 1")
            foreach (f RANGE ${last_failure})
                string(JSON text GET "${result}" testsuites ${s} testsuite ${t} failures ${f}
                    failure)
                string(REGEX MATCHALL "\\[check: [a-z0-9-]+\\]" found "${text}")
                if (NOT found)
                    list(APPEND checks untagged)
                endif()
                foreach (tag IN LISTS found)
                    string(REGEX REPLACE "\\[check: ([a-z0-9-]+)\\]" "\\1" tag "${tag}")
                    list(APPEND checks ${tag})
                endforeach()
            endforeach()
            list(REMOVE_DUPLICATES checks)
            list(SORT checks)
            list(JOIN checks "," checks)
            set(checks_of_${full_name} "${checks}")
        elseif (test_result STREQUAL "COMPLETED")
            list(APPEND passed ${full_name})
        endif()
    endforeach()
endforeach()

# known_red.txt : « nom  étiquette,étiquette » ; probabilistic.txt : des noms.
file(STRINGS ${KNOWN_RED_FILE} known_red_lines ENCODING UTF-8 REGEX "^[^#]")
set(known_red)
foreach (line IN LISTS known_red_lines)
    string(REGEX REPLACE "^([^ \t]+).*$" "\\1" name "${line}")
    # Par capture : un REGEX REPLACE ancré par « ^ » se réapplique au reste de la chaîne
    # et effacerait aussi les étiquettes.
    set(expected "")
    if (line MATCHES "^[^ \t]+[ \t]+([^ \t]+)")
        set(expected "${CMAKE_MATCH_1}")
    endif()
    string(REPLACE "," ";" expected "${expected}")
    list(SORT expected)
    list(JOIN expected "," expected)
    list(APPEND known_red ${name})
    set(expected_of_${name} "${expected}")
endforeach()
set(problems)
foreach (name IN LISTS red)
    if (name IN_LIST probabilistic)
        continue()
    endif()
    if (NOT name IN_LIST known_red)
        list(APPEND problems "new red: ${name} [${checks_of_${name}}]")
    elseif (NOT "${checks_of_${name}}" STREQUAL "${expected_of_${name}}")
        list(APPEND problems "known red for another reason: ${name} — expected [${expected_of_${name}}], got [${checks_of_${name}}]")
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
    # Le résultat d'une passe en échec est gardé : le suivant écraserait la preuve.
    string(TIMESTAMP stamp "%Y%m%d-%H%M%S")
    file(COPY_FILE ${RESULT_FILE} ${RESULT_FILE}.failed-${stamp})
    list(APPEND problems "result kept in ${RESULT_FILE}.failed-${stamp}")
    list(JOIN problems "\n  " text)
    message(FATAL_ERROR "the bench disagrees with known_red.txt "
        "(${num_red} red, ${num_passed} green):\n  ${text}")
endif()
message(STATUS "bench matches known_red.txt: ${num_red} red, ${num_passed} green")
