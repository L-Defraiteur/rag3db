#!/usr/bin/env python3

import rag3db
import timeit
import psutil

db = rag3db.Database("mydb")
con = rag3db.Connection(db)

start = timeit.default_timer()
con.execute(open("create.cypher").read())
con.execute("COPY hits FROM 'hits.csv' (PARALLEL=false);")
end = timeit.default_timer()
print(end - start)
