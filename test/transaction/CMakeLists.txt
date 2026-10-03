add_rag3db_test(current_time_test current_time_test.cpp)

# Also tested for coverage in connection_test.cpp
# but full testing requires some private APIs
add_rag3db_test(transaction_test transaction_test.cpp checkpoint_test.cpp wal_test.cpp
        read_only_open_test.cpp local_offset_remap_test.cpp)

add_subdirectory(concurrence)
