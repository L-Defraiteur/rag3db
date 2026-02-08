add_library(rag3db_parser_antlr_parser
        OBJECT
        rag3db_cypher_parser.cpp
        parser_error_listener.cpp
        parser_error_strategy.cpp)

set(ALL_OBJECT_FILES
        ${ALL_OBJECT_FILES} $<TARGET_OBJECTS:rag3db_parser_antlr_parser>
        PARENT_SCOPE)
