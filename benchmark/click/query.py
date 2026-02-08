#!/usr/bin/env python3

import timeit
import sys

import rag3db

query = sys.stdin.read()
print(query)

db = rag3db.Database(":memory:")
con = rag3db.Connection(db)
ver = con.execute("call DB_Version() return *;").get_next()[0]
db = rag3db.Database(f"mydb-{ver}", read_only=True)
con = rag3db.Connection(db)
for try_num in range(3):
    start = timeit.default_timer()
    results = con.execute(query.replace('\\', '\\\\'))
    end = timeit.default_timer()
    print(end - start)
