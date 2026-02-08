add_library(rag3db_rs STATIC
    ${CMAKE_CURRENT_SOURCE_DIR}/rag3db_rs.cpp
    ${RAG3DB_RS_BINDINGS_DIR}/sources/rag3db/src/lib.rs.cc
)

target_include_directories(rag3db_rs
    PRIVATE
        ${CMAKE_CURRENT_SOURCE_DIR}/../include
        ${RAG3DB_RS_BINDINGS_DIR}/include
        ${RAG3DB_RS_BINDINGS_DIR}/crate
        ${RAG3DB_RS_BINDINGS_DIR}/sources
)

target_link_libraries(rag3db_rs rag3db)
