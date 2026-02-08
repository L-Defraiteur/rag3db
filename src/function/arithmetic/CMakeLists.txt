add_library(rag3db_function_arithmetic
        OBJECT
        multiply.cpp
        add.cpp
        subtract.cpp
        divide.cpp
        modulo.cpp
        negate.cpp
        abs.cpp
        rand_function.cpp
        set_seed.cpp)

set(ALL_OBJECT_FILES
        ${ALL_OBJECT_FILES} $<TARGET_OBJECTS:rag3db_function_arithmetic>
        PARENT_SCOPE)
