include_directories(
        ${PROJECT_SOURCE_DIR}/src/include
        ${CMAKE_BINARY_DIR}/src/include
        src/include)

add_subdirectory(src/catalog)
add_subdirectory(src/function)
add_subdirectory(src/index)
add_subdirectory(src/main)

if(BUILD_EXTENSION_TESTS)
    add_subdirectory(test)
endif()

build_extension_lib(${BUILD_STATIC_EXTENSION} "geo")
