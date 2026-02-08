#include <inttypes.h>
#include <stdio.h>
#include <stdlib.h>

#include "rag3db.h"

int main() {
    rag3db_database db;
    rag3db_connection conn;
    rag3db_database_init("" /* fill db path */, rag3db_default_system_config(), &db);
    rag3db_connection_init(&db, &conn);

    // Create schema.
    rag3db_query_result result;
    rag3db_connection_query(
        &conn, "CREATE NODE TABLE Person(name STRING, age INT64, PRIMARY KEY(name));", &result);
    rag3db_query_result_destroy(&result);
    // Create nodes.
    rag3db_connection_query(&conn, "CREATE (:Person {name: 'Alice', age: 25});", &result);
    rag3db_query_result_destroy(&result);
    rag3db_connection_query(&conn, "CREATE (:Person {name: 'Bob', age: 30});", &result);
    rag3db_query_result_destroy(&result);

    // Execute a simple query.
    rag3db_connection_query(&conn, "MATCH (a:Person) RETURN a.name AS NAME, a.age AS AGE;", &result);

    // Fetch each value.
    rag3db_flat_tuple tuple;
    rag3db_value value;
    while (rag3db_query_result_has_next(&result)) {
        rag3db_query_result_get_next(&result, &tuple);

        rag3db_flat_tuple_get_value(&tuple, 0, &value);
        char* name;
        rag3db_value_get_string(&value, &name);

        rag3db_flat_tuple_get_value(&tuple, 1, &value);
        int64_t age;
        rag3db_value_get_int64(&value, &age);

        printf("name: %s, age: %" PRIi64 " \n", name, age);
        rag3db_destroy_string(name);
    }
    rag3db_value_destroy(&value);
    rag3db_flat_tuple_destroy(&tuple);

    // Print query result.
    char* result_string = rag3db_query_result_to_string(&result);
    printf("%s", result_string);
    rag3db_destroy_string(result_string);

    rag3db_query_result_destroy(&result);
    rag3db_connection_destroy(&conn);
    rag3db_database_destroy(&db);
    return 0;
}
