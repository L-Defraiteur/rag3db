add_subdirectory(join_order)
add_subdirectory(operator)
add_subdirectory(plan)

add_library(rag3db_planner
        OBJECT
        join_order_enumerator_context.cpp
        planner.cpp
        query_planner.cpp
        subplans_table.cpp)

set(ALL_OBJECT_FILES
        ${ALL_OBJECT_FILES} $<TARGET_OBJECTS:rag3db_planner>
        PARENT_SCOPE)
