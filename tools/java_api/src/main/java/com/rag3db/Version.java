package com.rag3db;

/**
 * Version is a class to get the version of the Rag3db.
 */
public class Version {

    /**
     * Get the version of the Rag3db.
     *
     * @return The version of the Rag3db.
     */
    public static String getVersion() {
        return Native.rag3dbGetVersion();
    }

    /**
     * Get the storage version of the Rag3db.
     *
     * @return The storage version of the Rag3db.
     */
    public static long getStorageVersion() {
        return Native.rag3dbGetStorageVersion();
    }
}
