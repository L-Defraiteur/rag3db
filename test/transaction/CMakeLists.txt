add_rag3db_test(current_time_test current_time_test.cpp)

# Also tested for coverage in connection_test.cpp
# but full testing requires some private APIs
add_rag3db_test(transaction_test transaction_test.cpp checkpoint_test.cpp wal_test.cpp
        read_only_open_test.cpp failed_transaction_test.cpp local_offset_remap_test.cpp
        readers_during_delete_test.cpp vector_search_during_insert_test.cpp
        vector_index_wal_replay_test.cpp vector_index_crash_reopen_test.cpp uncommitted_rels_scan_test.cpp
        vector_index_after_failed_copy_test.cpp checkpoint_after_failed_copy_test.cpp
        rolled_back_copies_test.cpp double_open_test.cpp lock_manager_test.cpp
        insert_lock_test.cpp
        journaled_copy_test.cpp
        vector_update_in_one_transaction_test.cpp rel_checkpoint_untouched_regions_test.cpp
        rel_string_property_checkpoint_test.cpp vector_index_rollback_test.cpp)

add_subdirectory(concurrence)
