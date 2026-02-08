add_library(rag3db_planner_sip
        OBJECT
        logical_semi_masker.cpp)

set(ALL_OBJECT_FILES
        ${ALL_OBJECT_FILES} $<TARGET_OBJECTS:rag3db_planner_sip>
        PARENT_SCOPE)
