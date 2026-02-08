add_library(rag3db_processor_operator_table_scan
        OBJECT
        ftable_scan_function.cpp
        union_all_scan.cpp)

set(ALL_OBJECT_FILES
        ${ALL_OBJECT_FILES} $<TARGET_OBJECTS:rag3db_processor_operator_table_scan>
        PARENT_SCOPE)
