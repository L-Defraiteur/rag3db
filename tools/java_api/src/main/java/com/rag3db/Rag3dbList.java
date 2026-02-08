package com.rag3db;

public class Rag3dbList implements AutoCloseable {
    private Value listVal;

    /**
     * @return Gets the underlying Value for the list
     */
    public Value getValue() {
        return listVal;
    }

    /**
     * Construct a list from a value
     *
     * @param value the value to construct the list from
     */
    public Rag3dbList(Value value) {
        listVal = value;
    }

    /**
     * Construct a list literal from an array of values
     *
     * @param values: the array to construct the list from
     */
    public Rag3dbList(Value[] values) {
        listVal = Native.rag3dbCreateList(values);
    }

    /**
     * Construct a list of a specific size populated with the default element
     *
     * @param numElements: the size of the list to construct
     */
    public Rag3dbList(DataType type, long numElements) {
        listVal = Native.rag3dbCreateList(type, numElements);
    }

    /**
     * Get the size of the list.
     *
     * @return The size of the list.
     * @throws RuntimeException If the list has been destroyed.
     */
    public long getListSize() {
        listVal.checkNotDestroyed();
        return Native.rag3dbValueGetListSize(listVal);
    }

    /**
     * Get the element at the given index from the given list.
     *
     * @param index: The index of the element.
     * @return The element at the given index from the given list.
     * @throws RuntimeException If the list has been destroyed.
     */
    public Value getListElement(long index) {
        listVal.checkNotDestroyed();
        return Native.rag3dbValueGetListElement(listVal, index);
    }

    /**
     * Gets the elements the list as a Java array. This will be truncated if the
     * size of the list doesn't fit in a 32-bit integer.
     *
     * @return the list as a Java array
     * @throws RuntimeException
     */
    public Value[] toArray() {
        int arraySize = ((Long) getListSize()).intValue();
        Value[] ret = new Value[arraySize];
        for (int i = 0; i < arraySize; ++i) {
            ret[i] = getListElement(i);
        }
        return ret;
    }

    /**
     * Closes this object, relinquishing the underlying value
     *
     * @throws RuntimeException
     */
    public void close() {
        listVal.close();
    }
}
