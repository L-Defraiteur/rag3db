# L'identifiant de bâti du moteur et des extensions, régénéré à chaque bâti (cible
# rag3db_build_id) : un identifiant pris à la configuration se périmerait au premier commit sans
# reconfiguration. LOAD EXTENSION refuse une extension dont l'identifiant diffère de celui du moteur
# (ExtensionManager::loadExtension).
#
# Entrées : SOURCE_DIR (la racine des sources), OUTPUT (l'en-tête à écrire), OPTIONS (les options qui
# changent l'interface binaire, déjà mises en forme), GIT_DIR (facultatif : le dépôt dont les
# sources sont une copie, lu seulement si git ne voit rien depuis SOURCE_DIR).
#
# L'identifiant : « rag3db-build-<commit>[-dirty-<empreinte du diff>]-<options> », où <commit> est
# le dernier commit qui touche le moteur ou ses extensions (les chemins ci-dessous) et le diff celui
# de ces mêmes chemins par rapport à la tête : un commit de docs, de tests ou de rag3weaver ne change
# pas l'identifiant, et n'oblige pas à rebâtir une extension. Déterministe à arbre et options égaux,
# sans horodatage ; les fichiers non suivis ne comptent pas. Sans dépôt git (une copie de l'arbre)
# ni GIT_DIR, « nogit ». L'en-tête n'est réécrit que si son contenu change : un bâti sans changement ne
# recompile rien.

set(enginePaths src extension third_party cmake CMakeLists.txt ":(exclude)extension/rag3weaver"
    ":(exclude)extension/lucivy")

# L'identité de l'arbre vu par git depuis `dir` : « <commit>[-dirty-<empreinte>] », ou vide si git
# n'y voit aucun commit des chemins du moteur (pas de dépôt, ou un dossier que le dépôt qui le
# contient ne suit pas).
function(gitIdentity dir out)
    set(${out} "" PARENT_SCOPE)
    execute_process(
        COMMAND git log -1 --format=%H -- ${enginePaths}
        WORKING_DIRECTORY "${dir}"
        OUTPUT_VARIABLE commit
        OUTPUT_STRIP_TRAILING_WHITESPACE
        RESULT_VARIABLE commitResult
        ERROR_QUIET)
    if (NOT commitResult EQUAL 0 OR commit STREQUAL "")
        return()
    endif ()
    set(identity "${commit}")
    execute_process(
        COMMAND git diff HEAD --no-ext-diff -- ${enginePaths}
        WORKING_DIRECTORY "${dir}"
        OUTPUT_VARIABLE diff
        RESULT_VARIABLE diffResult
        ERROR_QUIET)
    if (diffResult EQUAL 0 AND NOT diff STREQUAL "")
        string(SHA1 diffHash "${diff}")
        string(SUBSTRING "${diffHash}" 0 12 diffHash)
        set(identity "${identity}-dirty-${diffHash}")
    endif ()
    set(${out} "${identity}" PARENT_SCOPE)
endfunction()

gitIdentity("${SOURCE_DIR}" identity)
# Une copie de l'arbre sans son `.git` (la crate Rust sous Windows copie les sources dans
# tools/rust_api/rag3db-src) : GIT_DIR désigne le dépôt d'où vient la copie, et git y est lu à sa
# place, à chaque bâti comme ici. Jamais à la place d'un git que la source voit elle-même.
if (identity STREQUAL "" AND NOT "${GIT_DIR}" STREQUAL "")
    gitIdentity("${GIT_DIR}" identity)
endif ()
if (identity STREQUAL "")
    set(identity "nogit")
endif ()

set(content "// Généré par cmake/build_id.cmake à chaque bâti ; ne pas éditer.\n#pragma once\n\n#define RAG3DB_BUILD_ID \"rag3db-build-${identity}-${OPTIONS}\"\n")

set(previous "")
if (EXISTS "${OUTPUT}")
    file(READ "${OUTPUT}" previous)
endif ()
if (NOT previous STREQUAL content)
    file(WRITE "${OUTPUT}" "${content}")
endif ()
