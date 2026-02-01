include_directories(
        ${PROJECT_SOURCE_DIR}/src/include
        ${CMAKE_BINARY_DIR}/src/include
        src/include
        third_party/snowball/libstemmer
        ${PROJECT_SOURCE_DIR}/third_party/cppjieba/include
        ${PROJECT_SOURCE_DIR}/third_party/cppjieba/deps/limonp/include
        ${PROJECT_SOURCE_DIR}/third_party/fuzzy-fst/include)

add_subdirectory(src/catalog)
add_subdirectory(src/function)
add_subdirectory(src/main)
add_subdirectory(test)
add_subdirectory(src/index)
add_subdirectory(src/utils)

add_subdirectory(third_party/snowball)
add_subdirectory(${PROJECT_SOURCE_DIR}/third_party/fuzzy-fst ${CMAKE_CURRENT_BINARY_DIR}/fuzzy-fst)

build_extension_lib(${BUILD_STATIC_EXTENSION} "fts")

target_link_libraries(kuzu_${EXTENSION_LIB_NAME}_extension
        PRIVATE
        snowball
        re2
        cppjieba
        fuzzy_fst)

# Copy Jieba dictionaries next to the built extension so runtime can find a default path
add_custom_command(TARGET kuzu_${EXTENSION_LIB_NAME}_extension POST_BUILD
        COMMAND ${CMAKE_COMMAND} -E copy_directory
        ${PROJECT_SOURCE_DIR}/third_party/cppjieba/dict
        ${PROJECT_SOURCE_DIR}/extension/fts/build/dict)
