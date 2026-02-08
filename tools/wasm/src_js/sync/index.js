/**
 * @file index.js is the root file for the synchronous version of Rag3db
 * WebAssembly module. It exports the module's public interface.
 */
"use strict";

const Rag3dbWasm = require("./rag3db.js");
const Database = require("./database.js");
const Connection = require("./connection.js");
const PreparedStatement = require("./prepared_statement.js");
const QueryResult = require("./query_result.js");

/**
 * The synchronous version of Rag3db WebAssembly module.
 * @module rag3db-wasm
 */
module.exports = {
  /**
   * Initialize the Rag3db WebAssembly module.
   * @memberof module:rag3db-wasm
   * @returns {Promise<void>} a promise that resolves when the module is 
   * initialized. The promise is rejected if the module fails to initialize.
   */
  init: () => {
    return Rag3dbWasm.init();
  },

  /**
   * Get the version of the Rag3db WebAssembly module.
   * @memberof module:rag3db-wasm
   * @returns {String} the version of the Rag3db WebAssembly module.
   */
  getVersion: () => {
    return Rag3dbWasm.getVersion();
  },

  /**
   * Get the storage version of the Rag3db WebAssembly module.
   * @memberof module:rag3db-wasm
   * @returns {BigInt} the storage version of the Rag3db WebAssembly module.
   */
  getStorageVersion: () => {
    return Rag3dbWasm.getStorageVersion();
  },
  
  /**
   * Get the standard emscripten filesystem module (FS). Please refer to the 
   * emscripten documentation for more information.
   * @memberof module:rag3db-wasm
   * @returns {Object} the standard emscripten filesystem module (FS).
   */
  getFS: () => {
    return Rag3dbWasm.getFS();
  },

  /**
   * Get the WebAssembly memory. Please refer to the emscripten documentation 
   * for more information.
   * @memberof module:rag3db-wasm
   * @returns {Object} the WebAssembly memory object.
   */
  getWasmMemory: () => {
    return Rag3dbWasm.getWasmMemory();
  },

  Database,
  Connection,
  PreparedStatement,
  QueryResult,
};
