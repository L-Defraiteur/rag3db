include_directories(
        ./include
        ${PROJECT_SOURCE_DIR}/test/include/test_helper)

add_executable(rag3db_benchmark
        benchmark.cpp
        benchmark_parser.cpp
        benchmark_runner.cpp
        main.cpp)

target_link_libraries(rag3db_benchmark rag3db test_helper)
