#!/bin/bash

rm -rf cpp/docs cpp/headers c/docs c/rag3db.h
python3 collect_files.py
mv headers ./cpp/
cp ../../src/include/c_api/rag3db.h ./c/
cd cpp && doxygen Doxyfile
cd ..
cd c && doxygen Doxyfile
rm -rf cpp/headers c/rag3db.h
