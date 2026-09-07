COPY physical FROM "dataset/copy-test/escaped-newlines-large/physical.csv" (PARALLEL=FALSE);
COPY escaped FROM "dataset/copy-test/escaped-newlines-large/escaped.csv" (ESCAPED_NEWLINES=TRUE, AUTO_DETECT=FALSE);
