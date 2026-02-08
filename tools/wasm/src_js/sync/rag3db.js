/**
 * @file rag3db.js is the internal wrapper for the WebAssembly module.
 */
const rag3db_wasm = require("../rag3db/rag3db_wasm.js");

class rag3db {
  constructor() {
    this._rag3db = null;
  }

  async init() {
    this._rag3db = await rag3db_wasm();
  }

  checkInit() {
    if (!this._rag3db) {
      throw new Error("The WebAssembly module is not initialized.");
    }
  }

  getVersion() {
    this.checkInit();
    return this._rag3db.getVersion();
  }

  getStorageVersion() {
    this.checkInit();
    return this._rag3db.getStorageVersion();
  }

  getFS() {
    this.checkInit();
    return this._rag3db.FS;
  }

  getWasmMemory() {
    this.checkInit();
    return this._rag3db.wasmMemory;
  }
}

const rag3dbInstance = new rag3db();
module.exports = rag3dbInstance;
