if (${BUILD_EXTENSION_TESTS})
    add_rag3db_test(geo_test geo_test.cpp)
    target_include_directories(geo_test PRIVATE
        ${PROJECT_SOURCE_DIR}/extension/geo/src/include)
    target_link_libraries(geo_test PRIVATE
        $<TARGET_OBJECTS:rag3db_geo_index>)

    add_rag3db_test(geo_math_test geo_math_test.cpp)
    target_include_directories(geo_math_test PRIVATE
        ${PROJECT_SOURCE_DIR}/extension/geo/src/include)
endif ()
