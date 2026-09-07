COPY physical FROM "dataset/copy-test/escaped-newlines/physical.csv" (PARALLEL=FALSE);
COPY escaped FROM "dataset/copy-test/escaped-newlines/escaped.csv" (ESCAPED_NEWLINES=TRUE);
