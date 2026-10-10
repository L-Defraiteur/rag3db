# L'identifiant de bâti du moteur et des extensions, régénéré à chaque bâti (cible
# rag3db_build_id) : un identifiant pris à la configuration se périmerait au premier commit sans
# reconfiguration. LOAD EXTENSION refuse une extension dont l'identifiant diffère de celui du moteur
# (ExtensionManager::loadExtension).
#
# Entrées : SOURCE_DIR (la racine du dépôt), OUTPUT (l'en-tête à écrire), OPTIONS (les options qui
# changent l'interface binaire, déjà mises en forme).
#
# L'identifiant : « rag3db-build-<commit>[-dirty-<empreinte du diff>]-<options> », où <commit> est
# le dernier commit qui touche le moteur ou ses extensions (les chemins ci-dessous) et le diff celui
# de ces mêmes chemins par rapport à la tête : un commit de docs, de tests ou de rag3weaver ne change
# pas l'identifiant, et n'oblige pas à rebâtir une extension. Déterministe à arbre et options égaux,
# sans horodatage ; les fichiers non suivis ne comptent pas. Sans dépôt git (une copie de l'arbre),
# « nogit ». L'en-tête n'est réécrit que si son contenu change : un bâti sans changement ne
# recompile rien.

set(enginePaths src extension third_party cmake CMakeLists.txt ":(exclude)extension/rag3weaver"
    ":(exclude)extension/lucivy")

execute_process(
    COMMAND git log -1 --format=%H -- ${enginePaths}
    WORKING_DIRECTORY "${SOURCE_DIR}"
    OUTPUT_VARIABLE commit
    OUTPUT_STRIP_TRAILING_WHITESPACE
    RESULT_VARIABLE commitResult
    ERROR_QUIET)

if (NOT commitResult EQUAL 0 OR commit STREQUAL "")
    set(identity "nogit")
else ()
    set(identity "${commit}")
    execute_process(
        COMMAND git diff HEAD --no-ext-diff -- ${enginePaths}
        WORKING_DIRECTORY "${SOURCE_DIR}"
        OUTPUT_VARIABLE diff
        RESULT_VARIABLE diffResult
        ERROR_QUIET)
    if (diffResult EQUAL 0 AND NOT diff STREQUAL "")
        string(SHA1 diffHash "${diff}")
        string(SUBSTRING "${diffHash}" 0 12 diffHash)
        set(identity "${identity}-dirty-${diffHash}")
    endif ()
endif ()

set(content "// Généré par cmake/build_id.cmake à chaque bâti ; ne pas éditer.\n#pragma once\n\n#define RAG3DB_BUILD_ID \"rag3db-build-${identity}-${OPTIONS}\"\n")

set(previous "")
if (EXISTS "${OUTPUT}")
    file(READ "${OUTPUT}" previous)
endif ()
if (NOT previous STREQUAL content)
    file(WRITE "${OUTPUT}" "${content}")
endif ()
