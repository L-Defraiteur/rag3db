add_library(rag3db_planner_simple
        OBJECT
        logical_simple.cpp)

set(ALL_OBJECT_FILES
        ${ALL_OBJECT_FILES} $<TARGET_OBJECTS:rag3db_planner_simple>
        PARENT_SCOPE)
