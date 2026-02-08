package com.rag3db;

import java.util.Map;
import java.io.File;
import java.io.IOException;
import java.net.URL;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.StandardCopyOption;

/**
 * Native is a wrapper class for the native library.
 * It is used to load the native library and call the native functions.
 * This class is not intended to be used by end users.
 */
public class Native {
    static {
        try {
            String os_name = "";
            String os_arch;
            String os_name_detect = System.getProperty("os.name").toLowerCase().trim();
            String os_arch_detect = System.getProperty("os.arch").toLowerCase().trim();
            boolean isAndroid = System.getProperty("java.runtime.name", "").toLowerCase().contains("android")
                || System.getProperty("java.vendor", "").toLowerCase().contains("android")
                || System.getProperty("java.vm.name", "").toLowerCase().contains("dalvik");
            switch (os_arch_detect) {
                case "x86_64":
                case "amd64":
                    os_arch = "amd64";
                    break;
                case "aarch64":
                case "arm64":
                    os_arch = "arm64";
                    break;
                case "i386":
                    os_arch = "i386";
                    break;
                default:
                    throw new IllegalStateException("Unsupported system architecture");
            }
            if (isAndroid){
                os_name = "android";
            }
            else if (os_name_detect.startsWith("windows")) {
                os_name = "windows";
            } else if (os_name_detect.startsWith("mac")) {
                os_name = "osx";
            } else if (os_name_detect.startsWith("linux")) {
                os_name = "linux";
            }
            String lib_res_name = "/librag3db_java_native.so" + "_" + os_name + "_" + os_arch;

            Path lib_file = Files.createTempFile("librag3db_java_native", ".so");
            URL lib_res = Native.class.getResource(lib_res_name);
            if (lib_res == null) {
                throw new IOException(lib_res_name + " not found");
            }
            Files.copy(lib_res.openStream(), lib_file, StandardCopyOption.REPLACE_EXISTING);
            new File(lib_file.toString()).deleteOnExit();
            String lib_path = lib_file.toAbsolutePath().toString();
            System.load(lib_path);
            if (os_name.equals("linux")) {
                rag3dbNativeReloadLibrary(lib_path);
            }
        } catch (IOException e) {
            e.printStackTrace();
        }
    }

    // Hack: Reload the native library again in JNI bindings to work around the
    // extension loading issue on Linux as System.load() does not set
    // `RTLD_GLOBAL` flag and there is no way to set it in Java.
    protected static native void rag3dbNativeReloadLibrary(String libPath);

    // Database
    protected static native long rag3dbDatabaseInit(String databasePath, long bufferPoolSize,
            boolean enableCompression, boolean readOnly, long maxDbSize, boolean autoCheckpoint,
            long checkpointThreshold,boolean throwOnWalReplayFailure, boolean enableChecksums);

    protected static native void rag3dbDatabaseDestroy(Database db);

    protected static native void rag3dbDatabaseSetLoggingLevel(String loggingLevel);

    // Connection
    protected static native long rag3dbConnectionInit(Database database);

    protected static native void rag3dbConnectionDestroy(Connection connection);

    protected static native void rag3dbConnectionSetMaxNumThreadForExec(
            Connection connection, long numThreads);

    protected static native long rag3dbConnectionGetMaxNumThreadForExec(Connection connection);

    protected static native QueryResult rag3dbConnectionQuery(Connection connection, String query);

    protected static native PreparedStatement rag3dbConnectionPrepare(
            Connection connection, String query);

    protected static native QueryResult rag3dbConnectionExecute(
            Connection connection, PreparedStatement preparedStatement, Map<String, Value> param);

    protected static native void rag3dbConnectionInterrupt(Connection connection);

    protected static native void rag3dbConnectionSetQueryTimeout(
            Connection connection, long timeoutInMs);

    // PreparedStatement
    protected static native void rag3dbPreparedStatementDestroy(PreparedStatement preparedStatement);

    protected static native boolean rag3dbPreparedStatementIsSuccess(PreparedStatement preparedStatement);

    protected static native String rag3dbPreparedStatementGetErrorMessage(
            PreparedStatement preparedStatement);

    // QueryResult
    protected static native void rag3dbQueryResultDestroy(QueryResult queryResult);

    protected static native boolean rag3dbQueryResultIsSuccess(QueryResult queryResult);

    protected static native String rag3dbQueryResultGetErrorMessage(QueryResult queryResult);

    protected static native long rag3dbQueryResultGetNumColumns(QueryResult queryResult);

    protected static native String rag3dbQueryResultGetColumnName(QueryResult queryResult, long index);

    protected static native DataType rag3dbQueryResultGetColumnDataType(
            QueryResult queryResult, long index);

    protected static native long rag3dbQueryResultGetNumTuples(QueryResult queryResult);

    protected static native QuerySummary rag3dbQueryResultGetQuerySummary(QueryResult queryResult);

    protected static native boolean rag3dbQueryResultHasNext(QueryResult queryResult);

    protected static native FlatTuple rag3dbQueryResultGetNext(QueryResult queryResult);

    protected static native boolean rag3dbQueryResultHasNextQueryResult(QueryResult queryResult);

    protected static native QueryResult rag3dbQueryResultGetNextQueryResult(QueryResult queryResult);

    protected static native String rag3dbQueryResultToString(QueryResult queryResult);

    protected static native void rag3dbQueryResultResetIterator(QueryResult queryResult);

    // FlatTuple
    protected static native void rag3dbFlatTupleDestroy(FlatTuple flatTuple);

    protected static native Value rag3dbFlatTupleGetValue(FlatTuple flatTuple, long index);

    protected static native String rag3dbFlatTupleToString(FlatTuple flatTuple);

    // DataType
    protected static native long rag3dbDataTypeCreate(
            DataTypeID id, DataType childType, long numElementsInArray);

    protected static native DataType rag3dbDataTypeClone(DataType dataType);

    protected static native void rag3dbDataTypeDestroy(DataType dataType);

    protected static native boolean rag3dbDataTypeEquals(DataType dataType1, DataType dataType2);

    protected static native DataTypeID rag3dbDataTypeGetId(DataType dataType);

    protected static native DataType rag3dbDataTypeGetChildType(DataType dataType);

    protected static native long rag3dbDataTypeGetNumElementsInArray(DataType dataType);

    // Value
    protected static native Value rag3dbValueCreateNull();

    protected static native Value rag3dbValueCreateNullWithDataType(DataType dataType);

    protected static native boolean rag3dbValueIsNull(Value value);

    protected static native void rag3dbValueSetNull(Value value, boolean isNull);

    protected static native Value rag3dbValueCreateDefault(DataType dataType);

    protected static native <T> long rag3dbValueCreateValue(T val);

    protected static native Value rag3dbValueClone(Value value);

    protected static native void rag3dbValueCopy(Value value, Value other);

    protected static native void rag3dbValueDestroy(Value value);

    protected static native Value rag3dbCreateMap(Value[] keys, Value[] values);

    protected static native Value rag3dbCreateList(Value[] values);

    protected static native Value rag3dbCreateList(DataType type, long numElements);

    protected static native long rag3dbValueGetListSize(Value value);

    protected static native Value rag3dbValueGetListElement(Value value, long index);

    protected static native DataType rag3dbValueGetDataType(Value value);

    protected static native <T> T rag3dbValueGetValue(Value value);

    protected static native String rag3dbValueToString(Value value);

    protected static native InternalID rag3dbNodeValGetId(Value nodeVal);

    protected static native String rag3dbNodeValGetLabelName(Value nodeVal);

    protected static native long rag3dbNodeValGetPropertySize(Value nodeVal);

    protected static native String rag3dbNodeValGetPropertyNameAt(Value nodeVal, long index);

    protected static native Value rag3dbNodeValGetPropertyValueAt(Value nodeVal, long index);

    protected static native String rag3dbNodeValToString(Value nodeVal);

    protected static native InternalID rag3dbRelValGetId(Value relVal);

    protected static native InternalID rag3dbRelValGetSrcId(Value relVal);

    protected static native InternalID rag3dbRelValGetDstId(Value relVal);

    protected static native String rag3dbRelValGetLabelName(Value relVal);

    protected static native long rag3dbRelValGetPropertySize(Value relVal);

    protected static native String rag3dbRelValGetPropertyNameAt(Value relVal, long index);

    protected static native Value rag3dbRelValGetPropertyValueAt(Value relVal, long index);

    protected static native String rag3dbRelValToString(Value relVal);

    protected static native Value rag3dbCreateStruct(String[] fieldNames, Value[] fieldValues);

    protected static native String rag3dbValueGetStructFieldName(Value structVal, long index);

    protected static native long rag3dbValueGetStructIndex(Value structVal, String fieldName);

    protected static native String rag3dbGetVersion();

    protected static native long rag3dbGetStorageVersion();
}
